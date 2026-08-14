#![warn(missing_docs)]
//! Beru dependency resolver bridging recipes to the pubgrub solver.

use anyhow::Result;
use beru_core::cache::BeruCache;
use beru_manifest::Dependency;
use beru_recipe::resolve_recipe;
use pubgrub::DependencyProvider;
use pubgrub::SemanticVersion;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// A PubGrub dependency provider for Beru packages.
pub struct BeruProvider<'a> {
    /// Reference to the global Beru cache.
    pub cache: &'a BeruCache,
    /// The project directory for resolving local path dependencies.
    pub project_dir: &'a Path,
    /// The directory of the running Beru executable for bundled recipes.
    pub beru_exe_dir: Option<PathBuf>,

    /// Maps a package name to the known `Dependency` source declaration.
    /// This is populated dynamically as we read manifests.
    pub sources: RefCell<HashMap<String, Dependency>>,

    /// Caches the dependencies for a specific package version to avoid re-fetching.
    #[allow(clippy::type_complexity)]
    pub deps_cache: RefCell<
        HashMap<
            (String, SemanticVersion),
            pubgrub::Dependencies<String, pubgrub::Range<SemanticVersion>, String>,
        >,
    >,

    /// Available versions for a given package name.
    pub available_versions: RefCell<HashMap<String, Vec<SemanticVersion>>>,
}

impl<'a> BeruProvider<'a> {
    /// Construct a new `BeruProvider`.
    pub fn new(cache: &'a BeruCache, project_dir: &'a Path, beru_exe_dir: Option<PathBuf>) -> Self {
        Self {
            cache,
            project_dir,
            beru_exe_dir,
            sources: RefCell::new(HashMap::new()),
            deps_cache: RefCell::new(HashMap::new()),
            available_versions: RefCell::new(HashMap::new()),
        }
    }

    /// Add a source declaration (e.g., from a parsed Beru.toml or recipe).
    pub fn add_source(&self, name: &str, dep: &Dependency) {
        let mut sources = self.sources.borrow_mut();
        sources.insert(name.to_string(), dep.clone());
    }

    /// Ensure we know about the available versions of a package.
    fn ensure_versions(&self, package: &str) -> anyhow::Result<()> {
        if self.available_versions.borrow().contains_key(package) {
            return Ok(());
        }

        debug!("Resolving available versions for {}", package);
        let versions = self.fetch_available_versions(package)?;

        self.available_versions
            .borrow_mut()
            .insert(package.to_string(), versions);
        Ok(())
    }

    fn fetch_available_versions(&self, package: &str) -> anyhow::Result<Vec<SemanticVersion>> {
        if package == "root" {
            return Ok(vec![SemanticVersion::new(0, 0, 0)]);
        }

        let sources = self.sources.borrow();
        if let Some(dep) = sources.get(package) {
            match dep {
                Dependency::Git(g) => {
                    if let Some(tag) = &g.tag {
                        match parse_version(tag) {
                            Ok(v) => return Ok(vec![v]),
                            Err(e) => {
                                warn!(
                                    "Failed to parse git tag '{}' as version for {}: {}. Falling back to 0.0.0",
                                    tag, package, e
                                );
                            }
                        }
                    }
                    return Ok(vec![SemanticVersion::new(0, 0, 0)]);
                }
                Dependency::Path(_) => {
                    return Ok(vec![SemanticVersion::new(0, 0, 0)]);
                }
                Dependency::Registry(_) | Dependency::Version(_) => {}
            }
        }

        let mut versions = Vec::new();
        let index_pkg_dir = self.cache.index_dir().join(package);
        if index_pkg_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&index_pkg_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        let version_str = entry.file_name().to_string_lossy().into_owned();
                        match parse_version(&version_str) {
                            Ok(v) => {
                                let recipe_path = entry.path().join("recipe.toml");
                                if recipe_path.exists() {
                                    versions.push(v);
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Skipping index entry '{}' for {}: {}",
                                    version_str, package, e
                                );
                            }
                        }
                    }
                }
            }
        }

        if !versions.is_empty() {
            return Ok(versions);
        }

        let recipe = resolve_recipe(
            package,
            None,
            self.project_dir,
            self.beru_exe_dir.as_deref(),
            Some(&self.cache.recipes_dir()),
            Some(&self.cache.index_dir()),
        )?;

        if let Some((r, _)) = recipe {
            match parse_version(&r.package.version) {
                Ok(v) => return Ok(vec![v]),
                Err(e) => {
                    warn!(
                        "Failed to parse recipe version '{}' for {}: {}",
                        r.package.version, package, e
                    );
                }
            }
        }

        anyhow::bail!(
            "Could not find package '{}' in index, sources, or bundled recipes",
            package
        );
    }
}

impl<'a> DependencyProvider for BeruProvider<'a> {
    type P = String;
    type V = SemanticVersion;
    type VS = pubgrub::Range<SemanticVersion>;
    type Priority = usize;
    type M = String;
    type Err = std::io::Error;

    fn prioritize(
        &self,
        _package: &Self::P,
        _range: &Self::VS,
        _package_conflicts_counts: &pubgrub::PackageResolutionStatistics,
    ) -> Self::Priority {
        0
    }

    fn choose_version(
        &self,
        package: &Self::P,
        range: &Self::VS,
    ) -> Result<Option<Self::V>, Self::Err> {
        self.ensure_versions(package)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let versions = self.available_versions.borrow();
        if let Some(versions) = versions.get(package) {
            let mut valid_versions: Vec<SemanticVersion> = versions
                .iter()
                .filter(|v| range.contains(v))
                .cloned()
                .collect();
            valid_versions.sort();
            Ok(valid_versions.pop())
        } else {
            Ok(None)
        }
    }

    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<pubgrub::Dependencies<Self::P, Self::VS, Self::M>, Self::Err> {
        let key = (package.clone(), *version);
        if let Some(deps) = self.deps_cache.borrow().get(&key) {
            return Ok(deps.clone());
        }

        info!("Fetching dependencies for {} v{}", package, version);

        let mut deps_map = pubgrub::Map::default();

        let version_str = version.to_string();
        let index_recipe_path = self
            .cache
            .index_dir()
            .join(package)
            .join(&version_str)
            .join("recipe.toml");

        let mut recipe = None;
        if index_recipe_path.exists() {
            if let Ok(r) = beru_recipe::Recipe::from_file(&index_recipe_path) {
                recipe = Some(r);
            }
        }

        if recipe.is_none() {
            if let Ok(Some((r, _))) = resolve_recipe(
                package,
                Some(&version_str),
                self.project_dir,
                self.beru_exe_dir.as_deref(),
                Some(&self.cache.recipes_dir()),
                Some(&self.cache.index_dir()),
            ) {
                recipe = Some(r);
            }
        }

        if let Some(r) = recipe {
            for (dep_name, dep_value) in &r.dependencies {
                let range = parse_recipe_dep_range(dep_name, dep_value);
                deps_map.insert(dep_name.clone(), range);
            }
        }

        let deps = pubgrub::Dependencies::Available(deps_map.into_iter().collect());
        self.deps_cache.borrow_mut().insert(key, deps.clone());
        Ok(deps)
    }
}

/// Parse a version string into its (major, minor, patch) components.
fn parse_version_parts(s: &str) -> anyhow::Result<(u32, u32, u32)> {
    let clean = s.trim_start_matches('v');
    let parts: Vec<&str> = clean.split('.').collect();
    let major: u32 = parts
        .first()
        .ok_or_else(|| anyhow::anyhow!("missing major version in '{}'", s))?
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid major version in '{}'", s))?;
    let minor: u32 = parts
        .get(1)
        .unwrap_or(&"0")
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid minor version in '{}'", s))?;
    let patch: u32 = parts
        .get(2)
        .unwrap_or(&"0")
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid patch version in '{}'", s))?;
    Ok((major, minor, patch))
}

fn parse_version(s: &str) -> anyhow::Result<SemanticVersion> {
    let (major, minor, patch) = parse_version_parts(s)?;
    Ok(SemanticVersion::new(major, minor, patch))
}

/// Convert a version requirement string into a `pubgrub::Range<SemanticVersion>`.
///
/// Beru defaults to **exact pin** semantics:
/// - `"11.0.2"` or `"=11.0.2"` → `Range::singleton(11.0.2)` (exact pin)
/// - `"*"` → `Range::full()` (any version)
/// - `"^1.2.3"` → caret range: `>=1.2.3, <2.0.0`
/// - `"^0.2.3"` → caret range: `>=0.2.3, <0.3.0` (0.x special case)
/// - `"^0.0.3"` → caret range: `>=0.0.3, <0.0.4` (0.0.x special case)
/// - `"~1.2.3"` → tilde range: `>=1.2.3, <1.3.0`
/// - `">=1.2.3"` → `Range::higher_than(1.2.3)`
/// - `">=1.0.0, <2.0.0"` → `Range::between(1.0.0, 2.0.0)`
pub fn version_req_to_range(req: &str) -> pubgrub::Range<SemanticVersion> {
    let req = req.trim();

    // Wildcard: any version
    if req == "*" || req.is_empty() {
        return pubgrub::Range::full();
    }

    // Compound: ">=X.Y.Z, <A.B.C"
    if req.contains(',') {
        let parts: Vec<&str> = req.split(',').map(|s| s.trim()).collect();
        let mut range = pubgrub::Range::full();
        for part in parts {
            range = range.intersection(&version_req_to_range(part));
        }
        return range;
    }

    // Exact pin with "=" prefix
    if let Some(rest) = req.strip_prefix("=") {
        let rest = rest.trim();
        if let Ok(v) = parse_version(rest) {
            return pubgrub::Range::singleton(v);
        }
        warn!(
            "Failed to parse version in '{}', falling back to full range",
            req
        );
        return pubgrub::Range::full();
    }

    // Caret range: "^X.Y.Z"
    if let Some(rest) = req.strip_prefix('^') {
        let rest = rest.trim();
        if let Ok((major, minor, patch)) = parse_version_parts(rest) {
            let lower = SemanticVersion::new(major, minor, patch);
            let upper = caret_upper_bound(major, minor, patch);
            return pubgrub::Range::between(lower, upper);
        }
        warn!(
            "Failed to parse version in '{}', falling back to full range",
            req
        );
        return pubgrub::Range::full();
    }

    // Tilde range: "~X.Y.Z" → >=X.Y.Z, <X.(Y+1).0
    if let Some(rest) = req.strip_prefix('~') {
        let rest = rest.trim();
        if let Ok((major, minor, patch)) = parse_version_parts(rest) {
            let lower = SemanticVersion::new(major, minor, patch);
            let upper = SemanticVersion::new(major, minor + 1, 0);
            return pubgrub::Range::between(lower, upper);
        }
        warn!(
            "Failed to parse version in '{}', falling back to full range",
            req
        );
        return pubgrub::Range::full();
    }

    // Greater-than-or-equal: ">=X.Y.Z"
    if let Some(rest) = req.strip_prefix(">=") {
        let rest = rest.trim();
        if let Ok(v) = parse_version(rest) {
            return pubgrub::Range::higher_than(v);
        }
        warn!(
            "Failed to parse version in '{}', falling back to full range",
            req
        );
        return pubgrub::Range::full();
    }

    // Less-than: "<X.Y.Z" (used in compound expressions)
    if let Some(rest) = req.strip_prefix('<') {
        let rest = rest.trim();
        if let Ok(v) = parse_version(rest) {
            return pubgrub::Range::strictly_lower_than(v);
        }
        warn!(
            "Failed to parse version in '{}', falling back to full range",
            req
        );
        return pubgrub::Range::full();
    }

    // Bare version string: exact pin (Beru default)
    // "11.0.2" → =11.0.2
    if let Ok(v) = parse_version(req) {
        return pubgrub::Range::singleton(v);
    }

    warn!(
        "Could not parse version requirement '{}', falling back to full range",
        req
    );
    pubgrub::Range::full()
}

/// Compute the upper bound for a caret range.
///
/// - `^1.2.3` → `2.0.0` (bump major)
/// - `^0.2.3` → `0.3.0` (bump minor, 0.x special case)
/// - `^0.0.3` → `0.0.4` (bump patch, 0.0.x special case)
fn caret_upper_bound(major: u32, minor: u32, patch: u32) -> SemanticVersion {
    if major > 0 {
        SemanticVersion::new(major + 1, 0, 0)
    } else if minor > 0 {
        SemanticVersion::new(0, minor + 1, 0)
    } else {
        SemanticVersion::new(0, 0, patch + 1)
    }
}

/// Extract a version range from a recipe dependency's `toml::Value`.
///
/// Recipes declare dependencies as either:
/// - A bare string: `fmt = "11.0.2"`
/// - A table with a version key: `fmt = { version = "11.0.2" }`
///
/// Returns the parsed range, or `Range::full()` if no version is specified.
fn parse_recipe_dep_range(dep_name: &str, value: &toml::Value) -> pubgrub::Range<SemanticVersion> {
    let version_str = match value {
        toml::Value::String(s) => Some(s.as_str()),
        toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()),
        _ => None,
    };

    match version_str {
        Some(vs) => version_req_to_range(vs),
        None => {
            debug!(
                "No version constraint for recipe dependency '{}', using full range",
                dep_name
            );
            pubgrub::Range::full()
        }
    }
}

mod resolve;
pub use resolve::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_valid() {
        assert_eq!(
            parse_version("1.2.3").unwrap(),
            SemanticVersion::new(1, 2, 3)
        );
        assert_eq!(
            parse_version("v10.0.1").unwrap(),
            SemanticVersion::new(10, 0, 1)
        );
        assert_eq!(parse_version("0.1").unwrap(), SemanticVersion::new(0, 1, 0));
        assert_eq!(parse_version("5").unwrap(), SemanticVersion::new(5, 0, 0));
    }

    #[test]
    fn test_parse_version_invalid() {
        assert!(parse_version("").is_err());
        assert!(parse_version("abc").is_err());
        assert!(parse_version("1.abc.3").is_err());
    }

    #[test]
    fn test_version_req_exact_pin() {
        // Bare version = exact pin (Beru default)
        let range = version_req_to_range("11.0.2");
        assert!(range.contains(&SemanticVersion::new(11, 0, 2)));
        assert!(!range.contains(&SemanticVersion::new(11, 0, 3)));
        assert!(!range.contains(&SemanticVersion::new(11, 1, 0)));
    }

    #[test]
    fn test_version_req_explicit_exact() {
        let range = version_req_to_range("=3.7.1");
        assert!(range.contains(&SemanticVersion::new(3, 7, 1)));
        assert!(!range.contains(&SemanticVersion::new(3, 7, 2)));
    }

    #[test]
    fn test_version_req_wildcard() {
        let range = version_req_to_range("*");
        assert!(range.contains(&SemanticVersion::new(0, 0, 0)));
        assert!(range.contains(&SemanticVersion::new(99, 99, 99)));
    }

    #[test]
    fn test_version_req_caret() {
        // ^1.2.3 → >=1.2.3, <2.0.0
        let range = version_req_to_range("^1.2.3");
        assert!(range.contains(&SemanticVersion::new(1, 2, 3)));
        assert!(range.contains(&SemanticVersion::new(1, 9, 0)));
        assert!(!range.contains(&SemanticVersion::new(2, 0, 0)));
        assert!(!range.contains(&SemanticVersion::new(1, 2, 2)));
    }

    #[test]
    fn test_version_req_caret_zero_major() {
        // ^0.2.3 → >=0.2.3, <0.3.0
        let range = version_req_to_range("^0.2.3");
        assert!(range.contains(&SemanticVersion::new(0, 2, 3)));
        assert!(range.contains(&SemanticVersion::new(0, 2, 9)));
        assert!(!range.contains(&SemanticVersion::new(0, 3, 0)));
    }

    #[test]
    fn test_version_req_caret_zero_minor() {
        // ^0.0.3 → >=0.0.3, <0.0.4
        let range = version_req_to_range("^0.0.3");
        assert!(range.contains(&SemanticVersion::new(0, 0, 3)));
        assert!(!range.contains(&SemanticVersion::new(0, 0, 4)));
    }

    #[test]
    fn test_version_req_tilde() {
        // ~1.2.3 → >=1.2.3, <1.3.0
        let range = version_req_to_range("~1.2.3");
        assert!(range.contains(&SemanticVersion::new(1, 2, 3)));
        assert!(range.contains(&SemanticVersion::new(1, 2, 9)));
        assert!(!range.contains(&SemanticVersion::new(1, 3, 0)));
    }

    #[test]
    fn test_version_req_gte() {
        let range = version_req_to_range(">=2.0.0");
        assert!(range.contains(&SemanticVersion::new(2, 0, 0)));
        assert!(range.contains(&SemanticVersion::new(99, 0, 0)));
        assert!(!range.contains(&SemanticVersion::new(1, 9, 9)));
    }

    #[test]
    fn test_version_req_compound() {
        // >=1.0.0, <2.0.0
        let range = version_req_to_range(">=1.0.0, <2.0.0");
        assert!(range.contains(&SemanticVersion::new(1, 0, 0)));
        assert!(range.contains(&SemanticVersion::new(1, 9, 9)));
        assert!(!range.contains(&SemanticVersion::new(2, 0, 0)));
        assert!(!range.contains(&SemanticVersion::new(0, 9, 9)));
    }

    #[test]
    fn test_parse_recipe_dep_range_string() {
        let val = toml::Value::String("3.7.1".to_string());
        let range = parse_recipe_dep_range("catch2", &val);
        assert!(range.contains(&SemanticVersion::new(3, 7, 1)));
        assert!(!range.contains(&SemanticVersion::new(3, 7, 2)));
    }

    #[test]
    fn test_parse_recipe_dep_range_table() {
        let mut table = toml::map::Map::new();
        table.insert(
            "version".to_string(),
            toml::Value::String("^1.0.0".to_string()),
        );
        let val = toml::Value::Table(table);
        let range = parse_recipe_dep_range("fmt", &val);
        assert!(range.contains(&SemanticVersion::new(1, 0, 0)));
        assert!(range.contains(&SemanticVersion::new(1, 9, 0)));
        assert!(!range.contains(&SemanticVersion::new(2, 0, 0)));
    }

    #[test]
    fn test_parse_recipe_dep_range_no_version() {
        let mut table = toml::map::Map::new();
        table.insert(
            "registry".to_string(),
            toml::Value::String("https://example.com".to_string()),
        );
        let val = toml::Value::Table(table);
        let range = parse_recipe_dep_range("other", &val);
        // No version key → full range
        assert!(range.contains(&SemanticVersion::new(0, 0, 0)));
        assert!(range.contains(&SemanticVersion::new(99, 99, 99)));
    }
}
