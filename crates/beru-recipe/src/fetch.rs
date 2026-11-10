use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{debug, info};

use beru_core::cache::BeruCache;

static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
static DEST_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();

fn http_client() -> Result<&'static reqwest::blocking::Client> {
    if let Some(client) = HTTP_CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    let _ = HTTP_CLIENT.set(client);
    Ok(HTTP_CLIENT.get().expect("HTTP_CLIENT initialized"))
}

fn dest_lock(path: &Path) -> Arc<Mutex<()>> {
    let map = DEST_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().unwrap_or_else(|e| e.into_inner());
    Arc::clone(
        guard
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    )
}

/// Download and verify a source tarball from a URL.
///
/// 1. If the tarball is already cached (by SHA-256), skip the download.
/// 2. Download to a temporary buffer (reusing pooled HTTP connections).
/// 3. Verify the SHA-256 checksum.
/// 4. Extract into the cache.
///
/// Returns the path to the extracted source directory.
pub fn fetch_tarball(cache: &BeruCache, url: &str, expected_sha256: &str) -> Result<PathBuf> {
    let dest_dir = cache.source_dir(expected_sha256);

    if dest_dir.exists() {
        debug!("source already cached at {}", dest_dir.display());
        return Ok(dest_dir);
    }

    let lock = dest_lock(&dest_dir);
    let _dest_guard = lock.lock().unwrap_or_else(|e| e.into_inner());

    // Re-check inside lock in case another worker just completed extraction.
    if dest_dir.exists() {
        debug!("source already cached at {}", dest_dir.display());
        return Ok(dest_dir);
    }

    if std::env::var("BERU_OFFLINE").is_ok() {
        bail!(
            "cannot download {} in offline mode: package archive is not cached",
            url
        );
    }

    info!("downloading {}", url);

    let client = http_client()?;
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("failed to download {url}"))?;

    if !response.status().is_success() {
        bail!("download failed for {}: HTTP {}", url, response.status());
    }

    let bytes = response
        .bytes()
        .with_context(|| format!("failed to read response body from {url}"))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual_hash = beru_core::fs::hex_encode(&hasher.finalize());

    if actual_hash != expected_sha256 {
        bail!(
            "SHA-256 mismatch for {}:\n  expected: {}\n  actual:   {}",
            url,
            expected_sha256,
            actual_hash
        );
    }

    debug!("SHA-256 verified: {}", expected_sha256);

    let extract_res = (|| -> Result<()> {
        std::fs::create_dir_all(&dest_dir)
            .with_context(|| format!("failed to create {}", dest_dir.display()))?;
        extract_tarball(&bytes, &dest_dir)
            .with_context(|| format!("failed to extract tarball from {url}"))?;
        Ok(())
    })();

    if let Err(e) = extract_res {
        let _ = std::fs::remove_dir_all(&dest_dir);
        return Err(e);
    }

    info!("extracted to {}", dest_dir.display());

    Ok(dest_dir)
}

/// Clone a git repository into the cache.
///
/// If the repo is already cloned, does a fetch + checkout instead.
/// Returns the path to the cloned repository.
pub fn fetch_git(cache: &BeruCache, url: &str, pin: Option<&str>) -> Result<PathBuf> {
    let repo_dir = cache.git_repo_dir(url);
    let lock = dest_lock(&repo_dir);
    let _repo_guard = lock.lock().unwrap_or_else(|e| e.into_inner());

    if !repo_dir.exists() {
        if std::env::var("BERU_OFFLINE").is_ok() {
            bail!(
                "cannot clone {} in offline mode: repository is not cached",
                url
            );
        }

        info!("cloning {}", url);
        if let Some(parent) = repo_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let output = std::process::Command::new("git")
            .args(["init"])
            .arg(&repo_dir)
            .output()
            .context("failed to init git repo")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git init failed for {}:\n{}", url, stderr);
        }

        let output = std::process::Command::new("git")
            .current_dir(&repo_dir)
            .args(["remote", "add", "origin", url])
            .output()
            .context("failed to add git remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git remote add failed for {}:\n{}", url, stderr);
        }
    } else {
        debug!("git repo already cloned at {}", repo_dir.display());
        if let Some(p) = pin
            && is_git_commit_sha(p)
        {
            if let Ok(head_out) = std::process::Command::new("git")
                .args(["rev-parse", "--verify", "HEAD"])
                .current_dir(&repo_dir)
                .output()
                && head_out.status.success()
                && String::from_utf8_lossy(&head_out.stdout)
                    .trim()
                    .eq_ignore_ascii_case(p)
            {
                debug!("cached git repo already checked out at commit {}", p);
                return Ok(repo_dir);
            }
            if let Ok(co_out) = std::process::Command::new("git")
                .args(["checkout", "--force", p])
                .current_dir(&repo_dir)
                .output()
                && co_out.status.success()
            {
                debug!("checked out cached git commit {} without network fetch", p);
                return Ok(repo_dir);
            }
        }
    }

    if std::env::var("BERU_OFFLINE").is_ok() {
        if let Some(pin) = pin {
            let _ = std::process::Command::new("git")
                .args(["checkout", "--force", pin])
                .current_dir(&repo_dir)
                .output();
        }
        return Ok(repo_dir);
    }

    if let Some(pin) = pin {
        git_fetch_and_checkout(&repo_dir, pin)?;
    } else {
        // Just pull default branch
        let output = std::process::Command::new("git")
            .current_dir(&repo_dir)
            .args(["pull", "--depth", "1", "origin"])
            .output()
            .context("failed to pull default branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git pull failed:\n{stderr}");
        }
    }

    Ok(repo_dir)
}

/// Git fetch + checkout a specific ref.
fn git_fetch_and_checkout(repo_dir: &Path, refspec: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["fetch", "--depth", "1", "origin", refspec])
        .current_dir(repo_dir)
        .output()
        .context("failed to run git fetch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git fetch failed for {}:\n{}", refspec, stderr);
    }

    // `FETCH_HEAD` will contain the fetched commit, whether refspec was a tag or SHA
    let output = std::process::Command::new("git")
        .args(["checkout", "--force", "FETCH_HEAD"])
        .current_dir(repo_dir)
        .output()
        .context("failed to run git checkout FETCH_HEAD")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git checkout failed:\n{stderr}");
    }

    Ok(())
}

/// Extract a `.tar.gz` archive into a destination directory.
fn extract_tarball(data: &[u8], dest: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(data);
    let gz = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(dest)?;
    Ok(())
}

/// Find the actual source root inside an extracted tarball.
///
/// Many GitHub tarballs extract into a single top-level directory
/// (e.g. `fmt-11.0.2/`). This finds that directory.
pub fn find_source_root(extracted_dir: &Path) -> Result<PathBuf> {
    let entries: Vec<_> = std::fs::read_dir(extracted_dir)?
        .filter_map(|e| e.ok())
        .collect();

    if entries.len() == 1 && entries[0].file_type().is_ok_and(|ft| ft.is_dir()) {
        return Ok(entries[0].path());
    }

    Ok(extracted_dir.to_path_buf())
}

/// Check whether a Git ref string is a full 40-character hexadecimal commit SHA.
pub fn is_git_commit_sha(refspec: &str) -> bool {
    refspec.len() == 40 && refspec.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_source_root_single_subdir() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("mypkg-1.0.0");
        std::fs::create_dir_all(&nested).unwrap();

        let root = find_source_root(temp.path()).unwrap();
        assert_eq!(root, nested);
    }

    #[test]
    fn test_extract_tarball_corrupt_data_fails_cleanly() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("corrupt_dest");
        let corrupt_data = b"not a valid tarball";
        let res = extract_tarball(corrupt_data, &dest);
        assert!(res.is_err());
    }
}
