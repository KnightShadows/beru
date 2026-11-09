use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use console::style;
use std::path::{Path, PathBuf};

use beru_manifest::{is_valid_package_name, suggest_package_name};
use beru_recipe::Recipe;
use beru_resolve::parse_version;

/// Manage and author recipes.
#[derive(Debug, Args)]
pub struct RecipeArgs {
    #[command(subcommand)]
    pub command: RecipeSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum RecipeSubcommand {
    /// Scaffold a new recipe.toml for a package
    Init(RecipeInitArgs),

    /// Validate a recipe.toml for schema and packaging correctness
    Lint(RecipeLintArgs),
}

#[derive(Debug, Args)]
pub struct RecipeInitArgs {
    /// Package name (lowercase alphanumeric and hyphens)
    pub name: String,

    /// Package version (SemVer)
    #[arg(short = 'v', long, default_value = "1.0.0")]
    pub version: String,

    /// Git repository URL
    #[arg(long)]
    pub git: Option<String>,

    /// Git tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Release tarball URL
    #[arg(long)]
    pub url: Option<String>,

    /// SHA-256 hash of release tarball
    #[arg(long)]
    pub sha256: Option<String>,

    /// Whether this is a header-only library
    #[arg(long)]
    pub header_only: bool,

    /// CMake package name for find_package
    #[arg(long)]
    pub cmake_package: Option<String>,

    /// CMake target to link (e.g. "foo::foo")
    #[arg(long)]
    pub cmake_target: Option<String>,

    /// Output path for recipe.toml (defaults to ./recipe.toml)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RecipeLintArgs {
    /// Path to recipe.toml to validate
    #[arg(default_value = "recipe.toml")]
    pub path: PathBuf,
}

pub fn exec(args: RecipeArgs) -> Result<()> {
    match args.command {
        RecipeSubcommand::Init(init_args) => exec_init(init_args),
        RecipeSubcommand::Lint(lint_args) => exec_lint(lint_args),
    }
}

fn exec_init(args: RecipeInitArgs) -> Result<()> {
    if !is_valid_package_name(&args.name) {
        if let Some(suggestion) = suggest_package_name(&args.name) {
            bail!(
                "Invalid package name '{}': package names must contain only lowercase letters, digits, and hyphens.\n  Did you mean: '{}'?",
                args.name,
                suggestion
            );
        } else {
            bail!(
                "Invalid package name '{}': package names must contain only lowercase letters, digits, and hyphens.",
                args.name
            );
        }
    }

    if let Err(e) = parse_version(&args.version) {
        bail!("Invalid version '{}': {}", args.version, e);
    }

    let output_path = args.output.unwrap_or_else(|| {
        let recipes_dir = Path::new("recipes");
        if recipes_dir.exists() && recipes_dir.is_dir() {
            recipes_dir
                .join(&args.name)
                .join(&args.version)
                .join("recipe.toml")
        } else {
            PathBuf::from("recipe.toml")
        }
    });

    if output_path.exists() {
        bail!(
            "Destination file '{}' already exists. Refusing to overwrite.",
            output_path.display()
        );
    }

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let pkg_type = if args.header_only {
        "header-only"
    } else {
        "library"
    };
    let cmake_pkg = args
        .cmake_package
        .unwrap_or_else(|| args.name.replace('-', "_"));
    let cmake_tgt = args
        .cmake_target
        .unwrap_or_else(|| format!("{}::{}", cmake_pkg, cmake_pkg));

    let mut content = String::new();
    content.push_str("# Beru Recipe Definition\n\n");
    content.push_str("[package]\n");
    content.push_str(&format!("name = \"{}\"\n", args.name));
    content.push_str(&format!("version = \"{}\"\n", args.version));
    content.push_str(&format!("type = \"{}\"\n", pkg_type));
    content.push_str(&format!(
        "description = \"{} library for C++\"\n",
        args.name
    ));
    content.push_str("license = \"MIT\"\n\n");

    content.push_str("[source]\n");
    if let Some(ref tarball_url) = args.url {
        content.push_str(&format!("url = \"{}\"\n", tarball_url));
        let sha = args.sha256.unwrap_or_else(|| {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        });
        content.push_str(&format!("sha256 = \"{}\"\n\n", sha));
    } else if let Some(ref git_url) = args.git {
        content.push_str(&format!("git = \"{}\"\n", git_url));
        let tag = args.tag.unwrap_or_else(|| format!("v{}", args.version));
        content.push_str(&format!("tag = \"{}\"\n\n", tag));
    } else {
        content.push_str(&format!(
            "git = \"https://github.com/example/{}.git\"\ntag = \"v{}\"\n\n",
            args.name, args.version
        ));
    }

    content.push_str("[build]\nsystem = \"cmake\"\ncmake-args = []\n\n");

    content.push_str("[export]\n");
    content.push_str("include-dirs = [\"include\"]\n");
    content.push_str(&format!("cmake-package = \"{}\"\n", cmake_pkg));
    content.push_str(&format!("cmake-targets = [\"{}\"]\n", cmake_tgt));

    std::fs::write(&output_path, content)?;

    println!(
        "  {} scaffolded recipe at {}",
        style("Created").green().bold(),
        output_path.display()
    );

    Ok(())
}

fn exec_lint(args: RecipeLintArgs) -> Result<()> {
    if !args.path.exists() {
        bail!("Recipe file '{}' not found", args.path.display());
    }

    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("Failed to read recipe at '{}'", args.path.display()))?;

    let recipe = Recipe::parse_toml(&content)
        .with_context(|| format!("Failed to parse recipe TOML at '{}'", args.path.display()))?;

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 1. Package name validation
    if !is_valid_package_name(&recipe.package.name) {
        errors.push(format!(
            "Invalid package name '{}': must contain only lowercase letters, digits, and hyphens",
            recipe.package.name
        ));
    }

    // 2. SemVer validation
    if let Err(e) = parse_version(&recipe.package.version) {
        errors.push(format!(
            "Invalid package version '{}': {}",
            recipe.package.version, e
        ));
    }

    // 3. Source validation
    let has_tarball = recipe.source.url.is_some();
    let has_git = recipe.source.git.is_some();

    if !has_tarball && !has_git {
        errors.push("Recipe must specify either [source.url] or [source.git]".to_string());
    } else if has_tarball && has_git {
        errors.push("Recipe cannot specify both [source.url] and [source.git]".to_string());
    }

    if let Some(ref sha) = recipe.source.sha256 {
        if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            errors.push(format!(
                "Invalid sha256 '{}': must be a 64-character hexadecimal string",
                sha
            ));
        }
    } else if has_tarball {
        errors.push("Tarball source requires a 'sha256' checksum".to_string());
    }

    if has_git && recipe.source.tag.is_none() {
        warnings.push("Git source should specify an immutable 'tag'".to_string());
    }

    // 4. Build system validation
    if recipe.build.system != "cmake" && recipe.build.system != "custom" {
        errors.push(format!(
            "Invalid build system '{}': must be 'cmake' or 'custom'",
            recipe.build.system
        ));
    }

    // 5. Export validation
    if recipe.export.cmake_package.is_none()
        && recipe.export.cmake_targets.is_empty()
        && recipe.export.link_libs.is_empty()
    {
        warnings.push(
            "[export] should define 'cmake-package', 'cmake-targets', or 'link-libs'".to_string(),
        );
    }

    println!(
        "{} {} v{} ({})",
        style("Checking").green().bold(),
        recipe.package.name,
        recipe.package.version,
        args.path.display()
    );

    for w in &warnings {
        println!("  {} {}", style("Warning:").yellow().bold(), w);
    }

    if !errors.is_empty() {
        for err in &errors {
            println!("  {} {}", style("Error:").red().bold(), err);
        }
        bail!("Recipe validation failed with {} error(s)", errors.len());
    }

    println!("  {} Recipe is valid", style("Success:").green().bold());
    Ok(())
}
