use std::path::PathBuf;

/// Errors that can occur when parsing or validating a `Beru.toml` manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// An I/O error occurred while reading the manifest file.
    #[error("Failed to read {path}")]
    Io {
        /// The path that failed to be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A parsing error occurred (e.g. invalid TOML syntax).
    #[error("Failed to parse TOML: {0}")]
    Parse(String),

    /// An invalid C++ standard was requested (e.g. `c++99`).
    #[error("Invalid cxx-std: {0}")]
    InvalidCxxStd(String),

    /// The package name is invalid.
    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    /// The package version is not valid SemVer.
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    /// A general manifest invariant or workspace validation error.
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
}
