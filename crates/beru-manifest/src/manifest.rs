use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::dependency::Dependency;
use crate::error::ManifestError;

/// The top-level `Beru.toml` manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct BeruManifest {
    /// The core package definition.
    pub package: Package,

    /// `[dependencies]` — packages required for building.
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,

    /// `[dev-dependencies]` — packages required only for testing.
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<String, Dependency>,

    /// `[build]` — build system configuration.
    #[serde(default)]
    pub build: BuildConfig,

    /// `[profile.*]` — build profiles (debug, release, etc.).
    #[serde(default)]
    pub profile: BTreeMap<String, Profile>,
}

/// `[package]` table.
#[derive(Debug, Clone, Deserialize)]
pub struct Package {
    /// Package name (used for cache keys, recipe lookup, etc.).
    pub name: String,

    /// SemVer version string.
    pub version: String,

    /// C++ standard: `c++11`, `c++14`, `c++17`, `c++20`, `c++23`, `c++26`.
    #[serde(rename = "cxx-std", default = "default_cxx_std")]
    pub cxx_std: String,

    /// Package type.
    #[serde(rename = "type", default = "default_package_type")]
    pub package_type: PackageType,

    /// One-line description.
    #[serde(default)]
    pub description: Option<String>,

    /// SPDX license identifier.
    #[serde(default)]
    pub license: Option<String>,

    /// Author list.
    #[serde(default)]
    pub authors: Vec<String>,

    /// Repository URL.
    #[serde(default)]
    pub repository: Option<String>,
}

/// What kind of artifact this package produces.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageType {
    /// A library package (default).
    #[default]
    Library,
    /// An executable binary package.
    Executable,
    /// A header-only C++ library.
    HeaderOnly,
}

impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageType::Library => write!(f, "library"),
            PackageType::Executable => write!(f, "executable"),
            PackageType::HeaderOnly => write!(f, "header-only"),
        }
    }
}

/// `[build]` table.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    /// Build system to use.
    #[serde(default = "default_build_system")]
    pub system: BuildSystem,

    /// Minimum CMake version required.
    #[serde(rename = "cmake-minimum", default)]
    pub cmake_minimum: Option<String>,

    /// Whether to build shared libraries (default: false = static).
    #[serde(rename = "shared-libs", default)]
    pub shared_libs: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            system: BuildSystem::Cmake,
            cmake_minimum: None,
            shared_libs: false,
        }
    }
}

/// Supported build systems.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildSystem {
    /// The CMake build system (default).
    #[default]
    Cmake,
    /// Custom shell scripts for building.
    Custom,
}

impl std::fmt::Display for BuildSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildSystem::Cmake => write!(f, "cmake"),
            BuildSystem::Custom => write!(f, "custom"),
        }
    }
}

/// `[profile.<name>]` table (e.g. `[profile.release]`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Profile {
    /// Optimization level: `"0"`, `"1"`, `"2"`, `"3"`, `"s"`, `"z"`.
    #[serde(default)]
    pub optimization: Option<String>,

    /// Enable link-time optimization.
    #[serde(default)]
    pub lto: bool,

    /// Sanitizers to enable (e.g. `["address", "undefined"]`).
    #[serde(default)]
    pub sanitizers: Vec<String>,
}

fn default_cxx_std() -> String {
    "c++17".to_string()
}

fn default_package_type() -> PackageType {
    PackageType::Library
}

fn default_build_system() -> BuildSystem {
    BuildSystem::Cmake
}

impl BeruManifest {
    /// Load and parse a `Beru.toml` from the given directory.
    pub fn from_dir(dir: &Path) -> Result<Self, ManifestError> {
        let path = dir.join("Beru.toml");
        Self::from_file(&path)
    }

    /// Load and parse a `Beru.toml` from an explicit file path.
    pub fn from_file(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path).map_err(|e| ManifestError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::parse_toml(&content)
    }

    /// Parse a `Beru.toml` from a string.
    pub fn parse_toml(content: &str) -> Result<Self, ManifestError> {
        let manifest: BeruManifest =
            toml::from_str(content).map_err(|e| ManifestError::Parse(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate manifest invariants beyond what serde can check.
    fn validate(&self) -> Result<(), ManifestError> {
        let valid_stds = ["c++11", "c++14", "c++17", "c++20", "c++23", "c++26"];
        if !valid_stds.contains(&self.package.cxx_std.as_str()) {
            return Err(ManifestError::InvalidCxxStd(self.package.cxx_std.clone()));
        }

        if !is_valid_package_name(&self.package.name) {
            return Err(ManifestError::InvalidPackageName(self.package.name.clone()));
        }

        semver::Version::parse(&self.package.version)
            .map_err(|_| ManifestError::InvalidVersion(self.package.version.clone()))?;

        Ok(())
    }
}

/// Package names: lowercase ASCII letters, digits, and hyphens.
/// Must start with a letter and be at least 2 characters.
pub fn is_valid_package_name(name: &str) -> bool {
    if name.len() < 2 {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Suggests a valid package name from an invalid one if possible.
///
/// Converts underscores and spaces to hyphens, converts uppercase to lowercase,
/// and ensures the name starts with a valid prefix and meets length requirements.
pub fn suggest_package_name(name: &str) -> Option<String> {
    let sanitized: String = name
        .to_lowercase()
        .chars()
        .map(|c| match c {
            '_' | ' ' => '-',
            c if c.is_ascii_alphanumeric() || c == '-' => c,
            _ => '-',
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    let candidate = if trimmed.starts_with(|c: char| c.is_ascii_lowercase()) && trimmed.len() >= 2 {
        trimmed.to_string()
    } else {
        format!("pkg-{}", trimmed)
    };
    if is_valid_package_name(&candidate) && candidate != name {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_manifest() {
        let toml = r#"
[package]
name = "hello-world"
version = "0.1.0"
"#;
        let manifest = BeruManifest::parse_toml(toml).unwrap();
        assert_eq!(manifest.package.name, "hello-world");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.cxx_std, "c++17");
        assert_eq!(manifest.package.package_type, PackageType::Library);
    }

    #[test]
    fn test_parse_full_manifest() {
        let toml = r#"
[package]
name = "my-lib"
version = "1.2.3"
cxx-std = "c++20"
type = "executable"
description = "A test project"
license = "MIT"
authors = ["Test Author <test@example.com>"]
repository = "https://github.com/test/my-lib"

[dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "11.0.2" }
my-dep = { path = "../my-dep" }

[dev-dependencies]
gtest = { git = "https://github.com/google/googletest", tag = "v1.15.0" }

[build]
system = "cmake"
cmake-minimum = "3.20"
shared-libs = false

[profile.release]
optimization = "3"
lto = true

[profile.debug]
optimization = "0"
sanitizers = ["address", "undefined"]
"#;
        let manifest = BeruManifest::parse_toml(toml).unwrap();
        assert_eq!(manifest.package.cxx_std, "c++20");
        assert_eq!(manifest.package.package_type, PackageType::Executable);
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert!(manifest.profile.contains_key("release"));
        assert!(manifest.profile.contains_key("debug"));
    }

    #[test]
    fn test_invalid_cxx_std() {
        let toml = r#"
[package]
name = "bad-std"
version = "0.1.0"
cxx-std = "c++99"
"#;
        let result = BeruManifest::parse_toml(toml);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ManifestError::InvalidCxxStd(_)
        ));
    }

    #[test]
    fn test_invalid_package_name() {
        let toml = r#"
[package]
name = "X"
version = "0.1.0"
"#;
        let result = BeruManifest::parse_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_package_names() {
        assert!(is_valid_package_name("fmt"));
        assert!(is_valid_package_name("my-lib"));
        assert!(is_valid_package_name("boost-asio"));
        assert!(is_valid_package_name("lib2"));
        assert!(!is_valid_package_name("X"));
        assert!(!is_valid_package_name("123"));
        assert!(!is_valid_package_name("My_Lib"));
        assert!(!is_valid_package_name(""));
    }

    #[test]
    fn test_suggest_package_name() {
        assert_eq!(
            suggest_package_name("my_engine"),
            Some("my-engine".to_string())
        );
        assert_eq!(
            suggest_package_name("MyEngine"),
            Some("myengine".to_string())
        );
        assert_eq!(suggest_package_name("my-engine"), None);
    }
}
