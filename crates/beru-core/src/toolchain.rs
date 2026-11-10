use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use tracing::{debug, warn};

use crate::abi::AbiProfile;
use crate::cache::BeruCache;

/// Detected C++ compiler information.
#[derive(Debug, Clone)]
pub struct CompilerInfo {
    /// Absolute path to the compiler binary.
    pub path: PathBuf,
    /// Compiler family.
    pub family: CompilerFamily,
    /// Version string.
    pub version: String,
    /// Standard library being used.
    pub stdlib: String,
}

/// Known compiler families.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerFamily {
    /// GCC (GNU Compiler Collection).
    Gcc,
    /// Clang (LLVM).
    Clang,
    /// MSVC (Microsoft Visual C++).
    Msvc,
}

impl std::fmt::Display for CompilerFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerFamily::Gcc => write!(f, "gcc"),
            CompilerFamily::Clang => write!(f, "clang"),
            CompilerFamily::Msvc => write!(f, "msvc"),
        }
    }
}

impl CompilerFamily {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "gcc" => Some(Self::Gcc),
            "clang" => Some(Self::Clang),
            "msvc" => Some(Self::Msvc),
            _ => None,
        }
    }
}

type EnvKey = (Option<OsString>, Option<OsString>);

#[derive(Default)]
struct MemoryToolchainCache {
    compilers: HashMap<EnvKey, CompilerInfo>,
    features: HashMap<(PathBuf, String, String), bool>,
}

static MEMORY_CACHE: OnceLock<Mutex<MemoryToolchainCache>> = OnceLock::new();
static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

fn memory_cache() -> &'static Mutex<MemoryToolchainCache> {
    MEMORY_CACHE.get_or_init(|| Mutex::new(MemoryToolchainCache::default()))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DiskProbeCache {
    #[serde(default)]
    entries: BTreeMap<String, CachedCompilerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedCompilerEntry {
    family: String,
    version: String,
    stdlib: String,
    file_len: u64,
    mtime_secs: i64,
    mtime_nanos: u32,
    #[serde(default)]
    features: BTreeMap<String, bool>,
}

#[derive(Debug, Clone)]
struct BinaryFingerprint {
    cache_key: String,
    file_len: u64,
    mtime_secs: i64,
    mtime_nanos: u32,
}

fn current_env_key() -> EnvKey {
    (std::env::var_os("CXX"), std::env::var_os("PATH"))
}

fn disk_cache_path() -> Option<PathBuf> {
    BeruCache::default_location()
        .ok()
        .map(|c| c.root().join("cache").join("toolchain-probe.toml"))
}

fn binary_fingerprint(path: &Path) -> Option<BinaryFingerprint> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let key = canon.to_string_lossy().into_owned();
    let lower = key.to_ascii_lowercase();
    if lower.contains("ccache") || lower.contains("sccache") {
        return None;
    }

    let meta = std::fs::metadata(&canon).ok()?;
    let modified = meta.modified().ok()?;
    let (mtime_secs, mtime_nanos) = match modified.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_nanos()),
        Err(e) => {
            let d = e.duration();
            (-(d.as_secs() as i64), d.subsec_nanos())
        }
    };

    Some(BinaryFingerprint {
        cache_key: key,
        file_len: meta.len(),
        mtime_secs,
        mtime_nanos,
    })
}

fn load_disk_cache(path: &Path) -> DiskProbeCache {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_disk_cache(path: &Path, cache: &DiskProbeCache) {
    if let Ok(serialized) = toml::to_string(cache) {
        let _ = crate::fs::write_str_if_changed(path, &serialized);
    }
}

fn lookup_cached_compiler(path: &Path) -> Option<CompilerInfo> {
    let fp = binary_fingerprint(path)?;
    let cache_path = disk_cache_path()?;
    let disk = load_disk_cache(&cache_path);
    let entry = disk.entries.get(&fp.cache_key)?;
    if entry.file_len != fp.file_len
        || entry.mtime_secs != fp.mtime_secs
        || entry.mtime_nanos != fp.mtime_nanos
    {
        return None;
    }
    let family = CompilerFamily::parse(&entry.family)?;
    Some(CompilerInfo {
        path: path.to_path_buf(),
        family,
        version: entry.version.clone(),
        stdlib: entry.stdlib.clone(),
    })
}

fn store_cached_compiler(info: &CompilerInfo) {
    let Some(fp) = binary_fingerprint(&info.path) else {
        return;
    };
    let Some(cache_path) = disk_cache_path() else {
        return;
    };
    let mut disk = load_disk_cache(&cache_path);
    let existing_features = disk
        .entries
        .get(&fp.cache_key)
        .filter(|e| {
            e.file_len == fp.file_len
                && e.mtime_secs == fp.mtime_secs
                && e.mtime_nanos == fp.mtime_nanos
        })
        .map(|e| e.features.clone())
        .unwrap_or_default();

    disk.entries.insert(
        fp.cache_key,
        CachedCompilerEntry {
            family: info.family.to_string(),
            version: info.version.clone(),
            stdlib: info.stdlib.clone(),
            file_len: fp.file_len,
            mtime_secs: fp.mtime_secs,
            mtime_nanos: fp.mtime_nanos,
            features: existing_features,
        },
    );
    save_disk_cache(&cache_path, &disk);
}

fn lookup_cached_feature(compiler: &CompilerInfo, feature_key: &str) -> Option<bool> {
    let fp = binary_fingerprint(&compiler.path)?;
    let cache_path = disk_cache_path()?;
    let disk = load_disk_cache(&cache_path);
    let entry = disk.entries.get(&fp.cache_key)?;
    if entry.file_len != fp.file_len
        || entry.mtime_secs != fp.mtime_secs
        || entry.mtime_nanos != fp.mtime_nanos
    {
        return None;
    }
    entry.features.get(feature_key).copied()
}

fn store_cached_feature(compiler: &CompilerInfo, feature_key: &str, supported: bool) {
    let Some(fp) = binary_fingerprint(&compiler.path) else {
        return;
    };
    let Some(cache_path) = disk_cache_path() else {
        return;
    };
    let mut disk = load_disk_cache(&cache_path);
    let entry = disk
        .entries
        .entry(fp.cache_key)
        .or_insert_with(|| CachedCompilerEntry {
            family: compiler.family.to_string(),
            version: compiler.version.clone(),
            stdlib: compiler.stdlib.clone(),
            file_len: fp.file_len,
            mtime_secs: fp.mtime_secs,
            mtime_nanos: fp.mtime_nanos,
            features: BTreeMap::new(),
        });
    if entry.file_len != fp.file_len
        || entry.mtime_secs != fp.mtime_secs
        || entry.mtime_nanos != fp.mtime_nanos
    {
        entry.family = compiler.family.to_string();
        entry.version = compiler.version.clone();
        entry.stdlib = compiler.stdlib.clone();
        entry.file_len = fp.file_len;
        entry.mtime_secs = fp.mtime_secs;
        entry.mtime_nanos = fp.mtime_nanos;
        entry.features.clear();
    }
    entry.features.insert(feature_key.to_string(), supported);
    save_disk_cache(&cache_path, &disk);
}

/// Detect the C++ compiler available on the system.
///
/// Tries, in order: `CXX` env var, `c++`, `g++`, `clang++`, `cl`, `cl.exe`.
/// Results are cached in memory per `(CXX, PATH)` and persisted by binary
/// fingerprint under the cache directory to skip redundant `--version` spawns.
pub fn detect_compiler() -> Result<CompilerInfo> {
    let env_key = current_env_key();
    if let Ok(guard) = memory_cache().lock()
        && let Some(cached) = guard.compilers.get(&env_key)
    {
        return Ok(cached.clone());
    }

    let info = detect_compiler_uncached()?;

    if let Ok(mut guard) = memory_cache().lock() {
        guard.compilers.insert(env_key, info.clone());
    }

    Ok(info)
}

fn detect_compiler_uncached() -> Result<CompilerInfo> {
    if let Ok(cxx) = std::env::var("CXX") {
        debug!("using CXX environment variable: {}", cxx);
        if let Ok(path) = which::which(&cxx)
            && let Ok(info) = probe_compiler_path(&path)
        {
            return Ok(info);
        }
        warn!("CXX={} did not work, falling back to auto-detection", cxx);
    }

    let candidates = ["c++", "g++", "clang++", "cl", "cl.exe"];
    for candidate in &candidates {
        if let Ok(path) = which::which(candidate) {
            debug!("trying compiler: {}", path.display());
            if let Ok(info) = probe_compiler_path(&path) {
                return Ok(info);
            }
        }
    }

    bail!("no C++ compiler found. Install g++ or clang++, or set the CXX environment variable.")
}

/// Probe a resolved compiler binary path to determine its family, version, and stdlib.
fn probe_compiler_path(path: &Path) -> Result<CompilerInfo> {
    if let Some(cached) = lookup_cached_compiler(path) {
        debug!("using cached compiler probe for {}", path.display());
        return Ok(cached);
    }

    let output = Command::new(path)
        .arg("--version")
        .output()
        .with_context(|| format!("failed to run '{} --version'", path.display()))?;

    let version_output = String::from_utf8_lossy(&output.stdout);
    let stderr_output = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{version_output}\n{stderr_output}");

    let (family, version) = parse_compiler_version(&combined)?;
    let stdlib = detect_stdlib(&family);

    let info = CompilerInfo {
        path: path.to_path_buf(),
        family,
        version,
        stdlib,
    };
    store_cached_compiler(&info);
    Ok(info)
}

/// Parse compiler family and version from `--version` output.
fn parse_compiler_version(output: &str) -> Result<(CompilerFamily, String)> {
    let output_lower = output.to_lowercase();

    if output_lower.contains("clang") {
        let version = extract_version_number(output).unwrap_or_else(|| "unknown".to_string());
        return Ok((CompilerFamily::Clang, version));
    }

    if output_lower.contains("gcc") || output_lower.contains("g++") {
        let version = extract_version_number(output).unwrap_or_else(|| "unknown".to_string());
        return Ok((CompilerFamily::Gcc, version));
    }

    if output_lower.contains("microsoft") || output_lower.contains("msvc") {
        let version = extract_version_number(output).unwrap_or_else(|| "unknown".to_string());
        return Ok((CompilerFamily::Msvc, version));
    }

    bail!("could not determine compiler family from version output:\n{output}")
}

/// Extract a version number (like `14.1.0` or `18.1.6`) from text.
///
/// Finds the first occurrence of a pattern matching `X.Y` or `X.Y.Z`
/// where X, Y, Z are ASCII digits.
fn extract_version_number(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && bytes[i] == b'.' {
                i += 1;
                if i < len && bytes[i].is_ascii_digit() {
                    while i < len && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i < len && bytes[i] == b'.' {
                        let dot_pos = i;
                        i += 1;
                        if i < len && bytes[i].is_ascii_digit() {
                            while i < len && bytes[i].is_ascii_digit() {
                                i += 1;
                            }
                        } else {
                            i = dot_pos;
                        }
                    }
                    return std::str::from_utf8(&bytes[start..i])
                        .ok()
                        .map(str::to_string);
                }
            }
        }
        i += 1;
    }
    None
}

/// Determine the standard library based on compiler family and platform.
fn detect_stdlib(family: &CompilerFamily) -> String {
    match family {
        CompilerFamily::Gcc => "libstdc++".to_string(),
        CompilerFamily::Clang => {
            if cfg!(target_os = "macos") {
                "libc++".to_string()
            } else {
                "libstdc++".to_string()
            }
        }
        CompilerFamily::Msvc => "msvc-stl".to_string(),
    }
}

/// Detect the current system architecture.
pub fn detect_architecture() -> String {
    std::env::consts::ARCH.to_string()
}

/// Detect the current operating system.
pub fn detect_os() -> String {
    std::env::consts::OS.to_string()
}

/// Check if the compiler supports a specific C++ feature by compiling a dummy source file.
pub fn check_compiler_feature(compiler: &CompilerInfo, source: &str, cxx_std: &str) -> bool {
    let dir = std::env::temp_dir().join("beru_feature_checks");
    let _ = std::fs::create_dir_all(&dir);

    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
        crate::fs::hex_encode(&hasher.finalize()[..8])
    };
    let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let src_path = dir.join(format!("check_{pid}_{seq}_{hash}.cpp"));
    let out_path = dir.join(format!("check_{pid}_{seq}_{hash}.out"));

    if std::fs::write(&src_path, source).is_err() {
        return false;
    }

    let mut cmd = Command::new(&compiler.path);
    if compiler.family == CompilerFamily::Msvc {
        cmd.arg(format!("/std:c++{}", cxx_std))
            .arg("/c")
            .arg(&src_path)
            .arg(format!("/Fo{}", out_path.display()));
    } else {
        cmd.arg(format!("-std=c++{}", cxx_std))
            .arg("-c")
            .arg(&src_path)
            .arg("-o")
            .arg(&out_path);
    }

    let result = match cmd.output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&out_path);

    result
}

fn check_cached_feature(
    compiler: &CompilerInfo,
    feature_name: &str,
    source: &str,
    cxx_std: &str,
) -> bool {
    let mem_key = (
        compiler.path.clone(),
        feature_name.to_string(),
        cxx_std.to_string(),
    );
    if let Ok(guard) = memory_cache().lock()
        && let Some(&cached) = guard.features.get(&mem_key)
    {
        return cached;
    }

    let disk_key = format!("{feature_name}:{cxx_std}");
    if let Some(cached) = lookup_cached_feature(compiler, &disk_key) {
        if let Ok(mut guard) = memory_cache().lock() {
            guard.features.insert(mem_key, cached);
        }
        return cached;
    }

    let supported = check_compiler_feature(compiler, source, cxx_std);
    store_cached_feature(compiler, &disk_key, supported);
    if let Ok(mut guard) = memory_cache().lock() {
        guard.features.insert(mem_key, supported);
    }
    supported
}

/// Check if the compiler supports C++20 concepts.
pub fn supports_concepts(compiler: &CompilerInfo, cxx_std: &str) -> bool {
    const SOURCE: &str = r#"
#include <concepts>
template<typename T>
concept Integral = requires(T a) {
    { a + a } -> std::same_as<T>;
};
int main() { return 0; }
"#;
    check_cached_feature(compiler, "concepts", SOURCE, cxx_std)
}

/// Check if the compiler supports C++20 modules.
pub fn supports_modules(compiler: &CompilerInfo, cxx_std: &str) -> bool {
    const SOURCE: &str = r#"
export module test;
export int foo() { return 42; }
"#;
    check_cached_feature(compiler, "modules", SOURCE, cxx_std)
}

/// Build a complete ABI profile from the detected toolchain and manifest config.
pub fn build_abi_profile(
    cxx_std: &str,
    build_type: &str,
    shared_libs: bool,
    mut features: Vec<String>,
) -> Result<AbiProfile> {
    let compiler = detect_compiler()?;

    if cxx_std == "20" || cxx_std == "23" || cxx_std == "26" {
        if supports_concepts(&compiler, cxx_std) && !features.contains(&"cxx_concepts".to_string())
        {
            features.push("cxx_concepts".to_string());
        }
        if supports_modules(&compiler, cxx_std) && !features.contains(&"cxx_modules".to_string()) {
            features.push("cxx_modules".to_string());
        }
    }

    features.sort();

    Ok(AbiProfile {
        compiler: compiler.family.to_string(),
        compiler_version: compiler.version,
        stdlib: compiler.stdlib,
        architecture: detect_architecture(),
        os: detect_os(),
        build_type: build_type.to_string(),
        cxx_std: cxx_std.to_string(),
        shared_libs,
        features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gcc_version() {
        let output = "g++ (Ubuntu 14.1.0-1ubuntu1) 14.1.0\nCopyright ...";
        let (family, version) = parse_compiler_version(output).unwrap();
        assert_eq!(family, CompilerFamily::Gcc);
        assert_eq!(version, "14.1.0");
    }

    #[test]
    fn test_parse_clang_version() {
        let output =
            "Ubuntu clang version 18.1.6 (++20240518023432+...)\nTarget: x86_64-pc-linux-gnu";
        let (family, version) = parse_compiler_version(output).unwrap();
        assert_eq!(family, CompilerFamily::Clang);
        assert_eq!(version, "18.1.6");
    }

    #[test]
    fn test_extract_version_number() {
        assert_eq!(
            extract_version_number("g++ (Ubuntu 14.1.0) 14.1.0"),
            Some("14.1.0".to_string())
        );
        assert_eq!(
            extract_version_number("clang version 18.1"),
            Some("18.1".to_string())
        );
        assert_eq!(extract_version_number("no version here"), None);
    }

    #[test]
    fn test_detect_arch_and_os() {
        let arch = detect_architecture();
        let os = detect_os();
        assert!(!arch.is_empty());
        assert!(!os.is_empty());
    }

    #[test]
    fn test_disk_probe_cache_roundtrip_and_invalidation() {
        let dir = tempfile::tempdir().unwrap();
        let fake_compiler = dir.path().join("my-g++");
        std::fs::write(&fake_compiler, b"binary-v1").unwrap();

        let cache_file = dir.path().join("toolchain-probe.toml");
        let fp = binary_fingerprint(&fake_compiler).unwrap();
        let mut disk = DiskProbeCache::default();
        let mut features = BTreeMap::new();
        features.insert("concepts:20".to_string(), true);

        disk.entries.insert(
            fp.cache_key.clone(),
            CachedCompilerEntry {
                family: "gcc".to_string(),
                version: "14.2.0".to_string(),
                stdlib: "libstdc++".to_string(),
                file_len: fp.file_len,
                mtime_secs: fp.mtime_secs,
                mtime_nanos: fp.mtime_nanos,
                features,
            },
        );
        save_disk_cache(&cache_file, &disk);

        let loaded = load_disk_cache(&cache_file);
        let entry = loaded.entries.get(&fp.cache_key).unwrap();
        assert_eq!(entry.version, "14.2.0");
        assert_eq!(entry.features.get("concepts:20"), Some(&true));

        // Changing compiler binary contents changes fingerprint length -> cache invalidated
        std::fs::write(&fake_compiler, b"binary-v2-upgraded").unwrap();
        let fp2 = binary_fingerprint(&fake_compiler).unwrap();
        assert_ne!(fp.file_len, fp2.file_len);
    }
}
