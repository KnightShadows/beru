use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::process::Command;
use tracing::{debug, warn};

use crate::abi::AbiProfile;

/// Detected C++ compiler information.
#[derive(Debug, Clone)]
pub struct CompilerInfo {
    /// Absolute path to the compiler binary.
    pub path: std::path::PathBuf,
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

/// Detect the C++ compiler available on the system.
///
/// Tries, in order: `CXX` env var, `c++`, `g++`, `clang++`.
pub fn detect_compiler() -> Result<CompilerInfo> {
    if let Ok(cxx) = std::env::var("CXX") {
        debug!("using CXX environment variable: {}", cxx);
        if let Ok(info) = probe_compiler(&cxx) {
            return Ok(info);
        }
        warn!("CXX={} did not work, falling back to auto-detection", cxx);
    }

    let candidates = ["c++", "g++", "clang++", "cl", "cl.exe"];
    for candidate in &candidates {
        if let Ok(path) = which::which(candidate) {
            debug!("trying compiler: {}", path.display());
            if let Ok(info) = probe_compiler(path.to_str().unwrap_or(candidate)) {
                return Ok(info);
            }
        }
    }

    bail!("no C++ compiler found. Install g++ or clang++, or set the CXX environment variable.")
}

/// Probe a specific compiler to determine its family, version, and stdlib.
fn probe_compiler(compiler: &str) -> Result<CompilerInfo> {
    let path = which::which(compiler)
        .with_context(|| format!("compiler '{compiler}' not found on PATH"))?;

    let output = Command::new(&path)
        .arg("--version")
        .output()
        .with_context(|| format!("failed to run '{} --version'", path.display()))?;

    let version_output = String::from_utf8_lossy(&output.stdout);
    let stderr_output = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{version_output}\n{stderr_output}");

    let (family, version) = parse_compiler_version(&combined)?;

    let stdlib = detect_stdlib(&family);

    Ok(CompilerInfo {
        path,
        family,
        version,
        stdlib,
    })
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
/// where X, Y, Z are sequences of digits.
fn extract_version_number(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < len && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && chars[i] == '.' {
                i += 1;
                if i < len && chars[i].is_ascii_digit() {
                    while i < len && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i < len && chars[i] == '.' {
                        let dot_pos = i;
                        i += 1;
                        if i < len && chars[i].is_ascii_digit() {
                            while i < len && chars[i].is_ascii_digit() {
                                i += 1;
                            }
                        } else {
                            i = dot_pos;
                        }
                    }
                    return Some(chars[start..i].iter().collect());
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
        let result = hasher.finalize();
        result
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let src_path = dir.join(format!("check_{}.cpp", hash));
    let out_path = dir.join(format!("check_{}.out", hash));

    if std::fs::write(&src_path, source).is_err() {
        return false;
    }

    let mut cmd = Command::new(&compiler.path);
    cmd.arg(format!("-std=c++{}", cxx_std));

    cmd.arg("-c").arg(&src_path).arg("-o").arg(&out_path);

    let result = match cmd.output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&out_path);

    result
}

/// Check if the compiler supports C++20 concepts
pub fn supports_concepts(compiler: &CompilerInfo, cxx_std: &str) -> bool {
    let source = r#"
template<typename T>
concept Integral = requires(T a) {
    { a + a } -> std::same_as<T>;
};
int main() { return 0; }
"#;
    check_compiler_feature(compiler, source, cxx_std)
}

/// Check if the compiler supports C++20 modules
pub fn supports_modules(compiler: &CompilerInfo, cxx_std: &str) -> bool {
    let source = r#"
export module test;
export int foo() { return 42; }
"#;
    check_compiler_feature(compiler, source, cxx_std)
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
}
