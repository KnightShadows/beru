use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::debug;

/// The global Beru cache directory layout.
///
/// ```text
/// ~/.beru/
/// ├── cache/
/// │   ├── sources/         # downloaded tarballs + git clones, by sha256 or git url
/// │   └── builds/          # compiled artifacts, keyed by ABI profile hash
/// │       └── <abi-hash>/
/// │           └── <pkg>-<version>/
/// │               ├── include/
/// │               ├── lib/
/// │               └── .beru-meta.json
/// ├── recipes/             # git clone of beru-recipes (Phase 2)
/// ├── bin/                 # globally installed tools
/// └── config.toml
/// ```
#[derive(Debug, Clone)]
pub struct BeruCache {
    /// Root directory, typically `~/.beru`.
    root: PathBuf,
}

impl BeruCache {
    /// Create a cache rooted at the platform-standard location (`~/.beru`).
    pub fn default_location() -> Result<Self> {
        let root = if let Ok(custom) = std::env::var("BERU_HOME") {
            PathBuf::from(custom)
        } else {
            dirs::home_dir()
                .context("could not determine home directory")?
                .join(".beru")
        };
        Ok(Self { root })
    }

    /// Create a cache rooted at a custom directory (for testing).
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// The root cache directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory for downloaded source archives (`~/.beru/cache/sources/`).
    pub fn sources_dir(&self) -> PathBuf {
        self.root.join("cache").join("sources")
    }

    /// Directory for a specific source archive, keyed by SHA-256.
    /// Uses the first 16 characters of the hash to avoid exceeding Windows MAX_PATH limits.
    pub fn source_dir(&self, sha256: &str) -> PathBuf {
        let key = if sha256.len() > 16 {
            &sha256[..16]
        } else {
            sha256
        };
        self.sources_dir().join(key)
    }

    /// Directory for compiled build artifacts (`~/.beru/cache/builds/`).
    pub fn builds_dir(&self) -> PathBuf {
        self.root.join("cache").join("builds")
    }

    /// Directory for a specific built package under a given ABI profile hash.
    pub fn build_dir(&self, abi_hash: &str, package: &str, version: &str) -> PathBuf {
        self.builds_dir()
            .join(abi_hash)
            .join(format!("{package}-{version}"))
    }

    /// Check if a built artifact exists in the cache.
    pub fn has_build(&self, abi_hash: &str, package: &str, version: &str) -> bool {
        let dir = self.build_dir(abi_hash, package, version);
        dir.exists() && dir.join("include").exists()
    }

    /// The `include/` directory for a cached build.
    pub fn build_include_dir(&self, abi_hash: &str, package: &str, version: &str) -> PathBuf {
        self.build_dir(abi_hash, package, version).join("include")
    }

    /// The `lib/` directory for a cached build.
    pub fn build_lib_dir(&self, abi_hash: &str, package: &str, version: &str) -> PathBuf {
        self.build_dir(abi_hash, package, version).join("lib")
    }

    /// Directory for ad-hoc script build artifacts (`~/.beru/cache/adhoc/`).
    pub fn adhoc_dir(&self) -> PathBuf {
        self.root.join("cache").join("adhoc")
    }

    /// Directory for one specific ad-hoc build, keyed by content+ABI hash.
    pub fn adhoc_build_dir(&self, hash: &str) -> PathBuf {
        self.adhoc_dir().join(hash)
    }

    /// Directory for git clones (`~/.beru/cache/git/`).
    pub fn git_dir(&self) -> PathBuf {
        self.root.join("cache").join("git")
    }

    /// Directory for a specific git clone, keyed by a sanitized URL.
    pub fn git_repo_dir(&self, url: &str) -> PathBuf {
        let sanitized = sanitize_url(url);
        self.git_dir().join(sanitized)
    }

    /// Directory for bundled/local recipes.
    pub fn recipes_dir(&self) -> PathBuf {
        self.root.join("recipes")
    }

    /// Directory for globally installed tools.
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// Get the path to the global registry index repository
    pub fn index_dir(&self) -> PathBuf {
        self.root.join("index")
    }

    /// Ensure all cache directories exist.
    pub fn ensure_dirs(&self) -> Result<()> {
        let dirs = [
            self.sources_dir(),
            self.builds_dir(),
            self.git_dir(),
            self.recipes_dir(),
            self.bin_dir(),
            self.index_dir(),
            self.adhoc_dir(),
        ];
        for dir in &dirs {
            if !dir.exists() {
                debug!("creating cache directory: {}", dir.display());
                std::fs::create_dir_all(dir)
                    .with_context(|| format!("failed to create {}", dir.display()))?;
            }
        }
        Ok(())
    }
}

/// Sanitize a URL into a filesystem-safe directory name.
///
/// `https://github.com/fmtlib/fmt` → `github.com-fmtlib-fmt`
fn sanitize_url(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .replace(['/', ':'], "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_url() {
        assert_eq!(
            sanitize_url("https://github.com/fmtlib/fmt"),
            "github.com-fmtlib-fmt"
        );
        assert_eq!(
            sanitize_url("https://github.com/google/googletest"),
            "github.com-google-googletest"
        );
    }

    #[test]
    fn test_cache_paths() {
        let cache = BeruCache::with_root(PathBuf::from("/tmp/beru-test"));

        assert_eq!(
            cache.sources_dir(),
            PathBuf::from("/tmp/beru-test/cache/sources")
        );
        assert_eq!(
            cache.source_dir("abc123"),
            PathBuf::from("/tmp/beru-test/cache/sources/abc123")
        );
        assert_eq!(
            cache.build_dir("deadbeef", "fmt", "11.0.2"),
            PathBuf::from("/tmp/beru-test/cache/builds/deadbeef/fmt-11.0.2")
        );
        assert_eq!(
            cache.git_repo_dir("https://github.com/fmtlib/fmt"),
            PathBuf::from("/tmp/beru-test/cache/git/github.com-fmtlib-fmt")
        );
    }
}
