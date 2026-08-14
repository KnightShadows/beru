use serde::Deserialize;
use std::path::PathBuf;

/// A single dependency entry in `[dependencies]` or `[dev-dependencies]`.
///
/// Phase 1 supports two source types:
/// - `git`: clone from a git URL with an optional tag/branch/rev pin
/// - `path`: local filesystem path (for monorepo-style dev)
///
/// Phase 2 will add registry-based dependencies with version ranges.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// A git-sourced dependency.
    ///
    /// ```toml
    /// fmt = { git = "https://github.com/fmtlib/fmt", tag = "11.0.2" }
    /// ```
    Git(GitDependency),

    /// A local path dependency.
    ///
    /// ```toml
    /// my-dep = { path = "../my-dep" }
    /// ```
    Path(PathDependency),

    /// A registry dependency with explicit registry URL and version.
    ///
    /// ```toml
    /// catch2 = { version = "3.7.1", registry = "https://my-registry.com" }
    /// ```
    Registry(RegistryDependency),

    /// A simple version string for the default registry.
    ///
    /// ```toml
    /// catch2 = "3.7.1"
    /// ```
    Version(String),
}

/// Registry source for a dependency.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryDependency {
    /// The version constraint.
    pub version: String,

    /// Optional registry URL.
    pub registry: Option<String>,
}

/// Git source for a dependency.
#[derive(Debug, Clone, Deserialize)]
pub struct GitDependency {
    /// The git repository URL.
    pub git: String,

    /// Pin to a specific tag.
    #[serde(default)]
    pub tag: Option<String>,

    /// Pin to a specific branch.
    #[serde(default)]
    pub branch: Option<String>,

    /// Pin to a specific commit revision.
    #[serde(default)]
    pub rev: Option<String>,

    /// Override the package type for this dependency.
    #[serde(rename = "type", default)]
    pub package_type: Option<crate::PackageType>,
}

/// Local path source for a dependency.
#[derive(Debug, Clone, Deserialize)]
pub struct PathDependency {
    /// Path to the dependency (relative to the manifest directory).
    pub path: PathBuf,

    /// Override the package type for this dependency.
    #[serde(rename = "type", default)]
    pub package_type: Option<crate::PackageType>,
}

impl Dependency {
    /// Returns the raw version constraint string, if any.
    ///
    /// - `Version("11.0.2")` → `Some("11.0.2")`
    /// - `Registry { version: "3.7.1", .. }` → `Some("3.7.1")`
    /// - `Git { tag: Some("v1.0"), .. }` → `Some("v1.0")`
    /// - `Git { tag: None, .. }` / `Path(..)` → `None`
    pub fn version_string(&self) -> Option<&str> {
        match self {
            Dependency::Version(v) => Some(v.as_str()),
            Dependency::Registry(r) => Some(r.version.as_str()),
            Dependency::Git(g) => g.tag.as_deref(),
            Dependency::Path(_) => None,
        }
    }

    /// Returns a human-readable description of the source.
    pub fn source_display(&self) -> String {
        match self {
            Dependency::Git(g) => {
                let pin = g
                    .tag
                    .as_deref()
                    .or(g.branch.as_deref())
                    .or(g.rev.as_deref())
                    .unwrap_or("HEAD");
                format!("{} @ {}", g.git, pin)
            }
            Dependency::Path(p) => format!("path: {}", p.path.display()),
            Dependency::Registry(r) => {
                if let Some(reg) = &r.registry {
                    format!("registry: {} @ {}", reg, r.version)
                } else {
                    format!("registry @ {}", r.version)
                }
            }
            Dependency::Version(v) => format!("registry @ {}", v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_dep_with_tag() {
        let toml = r#"git = "https://github.com/fmtlib/fmt"
tag = "11.0.2""#;
        let dep: Dependency = toml::from_str(toml).unwrap();
        match dep {
            Dependency::Git(g) => {
                assert_eq!(g.git, "https://github.com/fmtlib/fmt");
                assert_eq!(g.tag.as_deref(), Some("11.0.2"));
            }
            _ => panic!("expected Git dependency"),
        }
    }

    #[test]
    fn test_parse_path_dep() {
        let toml = r#"path = "../my-dep""#;
        let dep: Dependency = toml::from_str(toml).unwrap();
        match dep {
            Dependency::Path(p) => {
                assert_eq!(p.path, PathBuf::from("../my-dep"));
            }
            _ => panic!("expected Path dependency"),
        }
    }

    #[test]
    fn test_source_display() {
        let git_dep = Dependency::Git(GitDependency {
            git: "https://github.com/fmtlib/fmt".to_string(),
            tag: Some("11.0.2".to_string()),
            branch: None,
            rev: None,
            package_type: None,
        });
        assert_eq!(
            git_dep.source_display(),
            "https://github.com/fmtlib/fmt @ 11.0.2"
        );

        let path_dep = Dependency::Path(PathDependency {
            path: PathBuf::from("../my-dep"),
            package_type: None,
        });
        assert_eq!(path_dep.source_display(), "path: ../my-dep");
    }
}
