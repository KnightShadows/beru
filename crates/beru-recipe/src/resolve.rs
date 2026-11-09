use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::Recipe;

/// The location where bundled recipes live (inside the Beru binary's crate).
/// At runtime, these are read from the `recipes/` directory relative to the
/// binary, or embedded as a fallback.
const BUNDLED_RECIPES_DIR: &str = "recipes";

/// Embedded fallback recipes shipped inside the binary.
pub const EMBEDDED_RECIPES: &[(&str, &str)] = &[
    (
        "fmt",
        include_str!("../../beru-cli/recipes/fmt/recipe.toml"),
    ),
    (
        "gtest",
        include_str!("../../beru-cli/recipes/gtest/recipe.toml"),
    ),
    (
        "nlohmann_json",
        include_str!("../../beru-cli/recipes/nlohmann_json/recipe.toml"),
    ),
    (
        "spdlog",
        include_str!("../../beru-cli/recipes/spdlog/recipe.toml"),
    ),
];

/// Get an embedded recipe string by package name if it exists.
pub fn get_embedded_recipe(package_name: &str) -> Option<&'static str> {
    EMBEDDED_RECIPES
        .iter()
        .find(|(name, _)| *name == package_name)
        .map(|(_, content)| *content)
}

/// Embedded fallback recipes shipped inside the binary.
pub const EMBEDDED_RECIPES: &[(&str, &str)] = &[
    ("fmt", include_str!("../../beru-cli/recipes/fmt/recipe.toml")),
    (
        "gtest",
        include_str!("../../beru-cli/recipes/gtest/recipe.toml"),
    ),
    (
        "nlohmann_json",
        include_str!("../../beru-cli/recipes/nlohmann_json/recipe.toml"),
    ),
    (
        "spdlog",
        include_str!("../../beru-cli/recipes/spdlog/recipe.toml"),
    ),
];

/// Get an embedded recipe string by package name if it exists.
pub fn get_embedded_recipe(package_name: &str) -> Option<&'static str> {
    EMBEDDED_RECIPES
        .iter()
        .find(|(name, _)| *name == package_name)
        .map(|(_, content)| *content)
}

/// Resolve a recipe for a given package name.
///
/// Resolution order:
/// 1. Project-local `recipes/<name>/recipe.toml`
/// 2. Bundled recipes shipped adjacent to the Beru binary
/// 3. User's global recipes at `~/.beru/recipes/<name>/recipe.toml`
/// 4. Index recipes at `~/.beru/index/<name>/<ver>/recipe.toml`
/// 5. Embedded fallback recipes compiled directly into the binary
///
/// Returns `None` if no recipe is found.
pub fn resolve_recipe(
    package_name: &str,
    version: Option<&str>,
    project_dir: &Path,
    beru_exe_dir: Option<&Path>,
    global_recipes_dir: Option<&Path>,
    index_dir: Option<&Path>,
) -> Result<Option<(Recipe, PathBuf)>> {
    let local_recipe = project_dir
        .join("recipes")
        .join(package_name)
        .join("recipe.toml");
    if local_recipe.exists() {
        debug!("found project-local recipe at {}", local_recipe.display());
        let recipe = Recipe::from_file(&local_recipe)
            .with_context(|| format!("failed to parse {}", local_recipe.display()))?;
        return Ok(Some((recipe, local_recipe)));
    }

    if let Some(exe_dir) = beru_exe_dir {
        let bundled_recipe = exe_dir
            .join(BUNDLED_RECIPES_DIR)
            .join(package_name)
            .join("recipe.toml");
        if bundled_recipe.exists() {
            debug!("found bundled recipe at {}", bundled_recipe.display());
            let recipe = Recipe::from_file(&bundled_recipe)
                .with_context(|| format!("failed to parse {}", bundled_recipe.display()))?;
            if version.is_none() || version == Some(recipe.package.version.as_str()) {
                return Ok(Some((recipe, bundled_recipe)));
            }
        }
    }

    if let Some(global_dir) = global_recipes_dir {
        let global_recipe = global_dir.join(package_name).join("recipe.toml");
        if global_recipe.exists() {
            debug!("found global recipe at {}", global_recipe.display());
            let recipe = Recipe::from_file(&global_recipe)
                .with_context(|| format!("failed to parse {}", global_recipe.display()))?;
            if version.is_none() || version == Some(recipe.package.version.as_str()) {
                return Ok(Some((recipe, global_recipe)));
            }
        }
    }

    if let (Some(idx), Some(ver)) = (index_dir, version) {
        let index_recipe = idx.join(package_name).join(ver).join("recipe.toml");
        if index_recipe.exists() {
            debug!("found index recipe at {}", index_recipe.display());
            let recipe = Recipe::from_file(&index_recipe)
                .with_context(|| format!("failed to parse {}", index_recipe.display()))?;
            return Ok(Some((recipe, index_recipe)));
        }
    }

    if let Some(content) = get_embedded_recipe(package_name) {
        let recipe = Recipe::parse_toml(content)
            .with_context(|| format!("failed to parse embedded recipe for {}", package_name))?;
        if version.is_none() || version == Some(recipe.package.version.as_str()) {
            debug!("found embedded recipe for {}", package_name);
            return Ok(Some((
                recipe,
                PathBuf::from(format!("<embedded>/{}/recipe.toml", package_name)),
            )));
        }
    }

    Ok(None)
}

/// Find the Beru executable's directory (for locating bundled recipes).
pub fn beru_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_recipe_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();

        let resolved =
            resolve_recipe("fmt", Some("11.0.2"), project_dir, None, None, None).unwrap();
        assert!(resolved.is_some());
        let (recipe, path) = resolved.unwrap();
        assert_eq!(recipe.package.name, "fmt");
        assert_eq!(recipe.package.version, "11.0.2");
        assert!(path.to_string_lossy().contains("<embedded>"));
    }

    #[test]
    fn test_embedded_recipe_wrong_version() {
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();

        let resolved = resolve_recipe("fmt", Some("9.0.0"), project_dir, None, None, None).unwrap();
        assert!(resolved.is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_recipe_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();

        let resolved =
            resolve_recipe("fmt", Some("11.0.2"), project_dir, None, None, None).unwrap();
        assert!(resolved.is_some());
        let (recipe, path) = resolved.unwrap();
        assert_eq!(recipe.package.name, "fmt");
        assert_eq!(recipe.package.version, "11.0.2");
        assert!(path.to_string_lossy().contains("<embedded>"));
    }

    #[test]
    fn test_embedded_recipe_wrong_version() {
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path();

        let resolved =
            resolve_recipe("fmt", Some("9.0.0"), project_dir, None, None, None).unwrap();
        assert!(resolved.is_none());
    }
}
