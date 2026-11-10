use anyhow::{Context, Result};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

type ToolCacheKey = (&'static str, Option<OsString>);
static TOOL_PATH_CACHE: OnceLock<Mutex<HashMap<ToolCacheKey, PathBuf>>> = OnceLock::new();

/// Resolve a tool binary on `PATH` and cache the resolved [`PathBuf`] in memory
/// keyed by `(binary_name, current PATH)` to skip repeated `PATH` directory walks.
pub fn which_cached(binary: &'static str) -> Result<PathBuf> {
    let key = (binary, std::env::var_os("PATH"));
    let cache = TOOL_PATH_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(cached) = guard.get(&key)
    {
        return Ok(cached.clone());
    }

    let resolved = which::which(binary).with_context(|| format!("{binary} not found on PATH"))?;
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, resolved.clone());
    }
    Ok(resolved)
}

/// Encode a byte slice as a lowercase hexadecimal string with a single allocation.
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX_DIGITS[(b >> 4) as usize] as char);
        out.push(HEX_DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

/// Write `content` to `path` only if the destination does not exist or its bytes differ.
///
/// Preserves the existing file modification timestamp when contents are unchanged,
/// preventing build tools like CMake from re-running configure passes needlessly.
/// Returns `Ok(true)` if the file was written, `Ok(false)` if unchanged.
pub fn write_if_changed(path: &Path, content: &[u8]) -> Result<bool> {
    if let Ok(meta) = std::fs::metadata(path)
        && meta.len() == content.len() as u64
        && let Ok(existing) = std::fs::read(path)
        && existing == content
    {
        return Ok(false);
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    std::fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(true)
}

/// Convenience wrapper over [`write_if_changed`] for UTF-8 string contents.
pub fn write_str_if_changed(path: &Path, content: &str) -> Result<bool> {
    write_if_changed(path, content.as_bytes())
}

/// Check if `haystack` contains an ASCII `needle` byte sequence ignoring ASCII case,
/// without heap allocation.
pub fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[]), "");
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    }

    #[test]
    fn test_write_if_changed_skips_identical() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("nested").join("test.txt");

        assert!(write_str_if_changed(&file, "hello world").unwrap());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello world");

        assert!(!write_str_if_changed(&file, "hello world").unwrap());

        assert!(write_str_if_changed(&file, "hello beru!").unwrap());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello beru!");
    }

    #[test]
    fn test_contains_ascii_case_insensitive() {
        assert!(contains_ascii_case_insensitive(
            b"SET(FOO)\nAdd_Library(bar STATIC IMPORTED)",
            b"add_library("
        ));
        assert!(!contains_ascii_case_insensitive(
            b"project(foo)",
            b"add_library("
        ));
        assert!(contains_ascii_case_insensitive(b"abc", b""));
    }
}
