use serde::Deserialize;

/// A recipe describes how to fetch and build a package that doesn't
/// natively support Beru.
///
/// Corresponds to `recipe.toml` files.
#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    /// Package metadata.
    pub package: RecipePackage,

    /// Where to download the source.
    pub source: RecipeSource,

    /// How to build the package.
    #[serde(default)]
    pub build: RecipeBuild,

    /// What the package exports to consumers.
    #[serde(default)]
    pub export: RecipeExport,

    /// Dependencies this recipe requires.
    #[serde(default)]
    pub dependencies: std::collections::BTreeMap<String, toml::Value>,
}

/// `[package]` table in a recipe.
#[derive(Debug, Clone, Deserialize)]
pub struct RecipePackage {
    /// The name of the package.
    pub name: String,
    /// The version of the package.
    pub version: String,
    /// The type of package.
    #[serde(rename = "type", default = "default_recipe_type")]
    pub package_type: String,
    /// A short description of the package.
    #[serde(default)]
    pub description: Option<String>,
    /// The SPDX license identifier.
    #[serde(default)]
    pub license: Option<String>,
    /// The URL to the homepage.
    #[serde(default)]
    pub homepage: Option<String>,
}

/// `[source]` table — where to download the package source.
#[derive(Debug, Clone, Deserialize)]
pub struct RecipeSource {
    /// URL to download a source tarball.
    #[serde(default)]
    pub url: Option<String>,

    /// SHA-256 checksum of the tarball.
    #[serde(default)]
    pub sha256: Option<String>,

    /// Git repository URL (alternative to tarball).
    #[serde(default)]
    pub git: Option<String>,

    /// Git tag to checkout.
    #[serde(default)]
    pub tag: Option<String>,
}

/// `[build]` table — how to build the package.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RecipeBuild {
    /// Build system: `cmake` or `custom`.
    #[serde(default = "default_build_system")]
    pub system: String,

    /// Extra CMake arguments (e.g. `-DFMT_TEST=OFF`).
    #[serde(rename = "cmake-args", alias = "args", alias = "cmake_args", default)]
    pub cmake_args: Vec<String>,

    /// Shell commands to execute if system = "custom".
    /// Supports {install_dir} and {jobs} templating.
    #[serde(default)]
    pub commands: Vec<String>,
}

/// `[export]` table — what the package provides to consumers.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RecipeExport {
    /// Relative paths to include directories.
    #[serde(rename = "include-dirs", alias = "include_dirs", default)]
    pub include_dirs: Vec<String>,

    /// Library names to link against (without `lib` prefix or extension).
    #[serde(rename = "link-libs", alias = "link_libs", default)]
    pub link_libs: Vec<String>,

    /// CMake package name for `find_package()` interop.
    #[serde(rename = "cmake-package", alias = "cmake_package", default)]
    pub cmake_package: Option<String>,

    /// CMake targets (e.g. `fmt::fmt`) for `target_link_libraries()`.
    #[serde(rename = "cmake-targets", alias = "cmake_targets", default)]
    pub cmake_targets: Vec<String>,
}

fn default_recipe_type() -> String {
    "library".to_string()
}

fn default_build_system() -> String {
    "cmake".to_string()
}

impl Recipe {
    /// Parse a `recipe.toml` from a string.
    pub fn parse_toml(content: &str) -> Result<Self, anyhow::Error> {
        let recipe: Self = toml::from_str(content)?;
        if recipe.build.system == "custom"
            && recipe.export.link_libs.is_empty()
            && recipe.export.include_dirs.is_empty()
            && recipe.export.cmake_targets.is_empty()
            && !recipe.is_header_only()
        {
            anyhow::bail!(
                "Recipes with system = 'custom' must explicitly define an [export] section (e.g. link-libs or include-dirs)."
            );
        }
        Ok(recipe)
    }

    /// Parse a `recipe.toml` from a file path.
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_toml(&content)
    }

    /// Whether this recipe describes a header-only package.
    pub fn is_header_only(&self) -> bool {
        self.package.package_type == "header-only"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_recipe() {
        let toml = r#"
[package]
name = "fmt"
version = "11.0.2"
type = "library"

[source]
url = "https://github.com/fmtlib/fmt/archive/refs/tags/11.0.2.tar.gz"
sha256 = "6cb1e6d37bdcb756dbbe59be438790db409cdb4868c66e888d5df9f13f7c027f"

[build]
system = "cmake"
cmake-args = ["-DFMT_DOC=OFF", "-DFMT_TEST=OFF", "-DFMT_INSTALL=ON"]

[export]
include-dirs = ["include"]
link-libs = ["fmt"]
cmake-package = "fmt"
cmake-targets = ["fmt::fmt"]
"#;
        let recipe = Recipe::parse_toml(toml).unwrap();
        assert_eq!(recipe.package.name, "fmt");
        assert_eq!(recipe.package.version, "11.0.2");
        assert!(!recipe.is_header_only());
        assert_eq!(recipe.build.cmake_args.len(), 3);
        assert_eq!(recipe.export.link_libs, vec!["fmt"]);
    }

    #[test]
    fn test_parse_header_only_recipe() {
        let toml = r#"
[package]
name = "nlohmann-json"
version = "3.11.3"
type = "header-only"

[source]
url = "https://github.com/nlohmann/json/releases/download/v3.11.3/json.tar.xz"
sha256 = "d6c65aca6b1ed68e7a182f4757f21f1be1c753c"

[export]
include-dirs = ["include"]
cmake-package = "nlohmann_json"
cmake-targets = ["nlohmann_json::nlohmann_json"]
"#;
        let recipe = Recipe::parse_toml(toml).unwrap();
        assert!(recipe.is_header_only());
        assert!(recipe.export.link_libs.is_empty());
    }

    #[test]
    fn test_parse_custom_build_system() {
        let toml = r#"
[package]
name = "boost"
version = "1.86.0"

[source]
url = "https://example.com/boost.tar.gz"

[build]
system = "custom"
commands = [
    "./bootstrap.sh --prefix={install_dir}",
    "./b2 install --prefix={install_dir} -j{jobs}"
]

[export]
include-dirs = ["include"]
link-libs = ["boost_system"]
"#;
        let recipe = Recipe::parse_toml(toml).unwrap();
        assert_eq!(recipe.build.system, "custom");
        assert_eq!(recipe.build.commands.len(), 2);
        assert_eq!(
            recipe.build.commands[0],
            "./bootstrap.sh --prefix={install_dir}"
        );
    }

    #[test]
    fn test_parse_custom_build_system_missing_export() {
        let toml = r#"
[package]
name = "boost"
version = "1.86.0"

[source]
url = "https://example.com/boost.tar.gz"

[build]
system = "custom"
commands = ["./do_something.sh"]
"#;
        let err = Recipe::parse_toml(toml).unwrap_err();
        assert!(
            err.to_string()
                .contains("must explicitly define an [export] section")
        );
    }

    #[test]
    fn test_parse_build_args_alias() {
        let toml = r#"
[package]
name = "eigen"
version = "3.4.0"

[source]
git = "https://gitlab.com/libeigen/eigen.git"
tag = "3.4.0"

[build]
system = "cmake"
args = ["-DBUILD_TESTING=OFF", "-DBUILD_EXAMPLES=OFF"]
"#;
        let recipe = Recipe::parse_toml(toml).unwrap();
        assert_eq!(recipe.package.name, "eigen");
        assert_eq!(
            recipe.build.cmake_args,
            vec!["-DBUILD_TESTING=OFF", "-DBUILD_EXAMPLES=OFF"]
        );
    }
}
