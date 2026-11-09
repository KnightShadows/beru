use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::fs;
use std::path::{Path, PathBuf};

/// Arguments for `beru vendor`.
#[derive(Debug, Args)]
pub struct VendorArgs {
    /// Directory where dependencies will be vendored (defaults to 'vendor' in project directory)
    pub path: Option<PathBuf>,

    /// Do not delete existing vendored package directories
    #[arg(long)]
    pub no_delete: bool,
}

/// Recursively copy a directory to destination, skipping `.git`, `build`, and `.beru`.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)
        .with_context(|| format!("failed to create directory {}", dst.display()))?;
    for entry in
        fs::read_dir(src).with_context(|| format!("failed to read directory {}", src.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        let file_name = entry.file_name();

        if file_name == ".git" || file_name == "build" || file_name == ".beru" {
            continue;
        }

        let target = dst.join(file_name);
        if file_type.is_dir() {
            copy_dir_all(&path, &target)?;
        } else if file_type.is_file() {
            fs::copy(&path, &target).with_context(|| {
                format!("failed to copy {} to {}", path.display(), target.display())
            })?;
        }
    }
    Ok(())
}

/// Execute the `beru vendor` command.
pub fn exec(args: VendorArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;

    let manifest_path = project_dir.join("Beru.toml");
    if !manifest_path.exists() {
        bail!(
            "Beru.toml not found in current directory. Run 'beru init' or 'beru new' to create a project."
        );
    }

    let lockfile_path = project_dir.join("Beru.lock");
    if !lockfile_path.exists() {
        bail!("Lockfile Beru.lock not found. Please run 'beru resolve' first.");
    }

    let manifest =
        beru_manifest::BeruManifest::from_dir(&project_dir).context("failed to parse Beru.toml")?;
    let lockfile =
        beru_manifest::BeruLock::from_dir(&project_dir).context("failed to parse Beru.lock")?;

    if lockfile.packages.is_empty() {
        println!(
            "{} No dependencies found in Beru.lock to vendor.",
            style("Notice:").yellow().bold()
        );
        return Ok(());
    }

    let vendor_root = match args.path {
        Some(p) => {
            if p.is_absolute() {
                p
            } else {
                project_dir.join(p)
            }
        }
        None => project_dir.join("vendor"),
    };

    let cache = beru_core::cache::BeruCache::default_location()
        .context("failed to initialize global cache")?;

    println!(
        "{} {} dependencies to {}...",
        style("Vendoring").cyan().bold(),
        lockfile.packages.len(),
        vendor_root.display()
    );

    for pkg in &lockfile.packages {
        let target_pkg_dir = vendor_root.join(format!("{}-{}", pkg.name, pkg.version));

        if target_pkg_dir.exists() && !args.no_delete {
            fs::remove_dir_all(&target_pkg_dir).with_context(|| {
                format!(
                    "failed to remove existing vendor directory {}",
                    target_pkg_dir.display()
                )
            })?;
        }

        let opt_dep = manifest.dependencies.get(&pkg.name);

        let recipe = beru_recipe::resolve_recipe(
            &pkg.name,
            Some(&pkg.version),
            &project_dir,
            None,
            Some(&cache.recipes_dir()),
            Some(&cache.index_dir()),
        )?;

        let source_dir = beru_build::resolve_package_source(
            &pkg.name,
            &pkg.version,
            opt_dep,
            pkg,
            recipe.as_ref(),
            &cache,
            &project_dir,
        )
        .with_context(|| {
            format!(
                "failed to resolve source for dependency '{}' ({})",
                pkg.name, pkg.version
            )
        })?;

        copy_dir_all(&source_dir, &target_pkg_dir)?;

        // Ensure recipe.toml is present inside vendored package directory
        if let Some((_, ref recipe_path)) = recipe {
            let target_recipe = target_pkg_dir.join("recipe.toml");
            if recipe_path.to_string_lossy().contains("<embedded>") {
                if let Some(content) = beru_recipe::get_embedded_recipe(&pkg.name) {
                    fs::write(&target_recipe, content).with_context(|| {
                        format!(
                            "failed to write embedded recipe to {}",
                            target_recipe.display()
                        )
                    })?;
                }
            } else if recipe_path.exists() && !target_recipe.exists() {
                fs::copy(recipe_path, &target_recipe).with_context(|| {
                    format!(
                        "failed to copy recipe from {} to {}",
                        recipe_path.display(),
                        target_recipe.display()
                    )
                })?;
            }
        }

        println!(
            "  {} {} ({}) -> {}",
            style("Vendored").green().bold(),
            pkg.name,
            pkg.version,
            target_pkg_dir
                .strip_prefix(&project_dir)
                .unwrap_or(&target_pkg_dir)
                .display()
        );
    }

    println!(
        "\n{} Vendored {} dependencies into {}",
        style("Finished").green().bold(),
        lockfile.packages.len(),
        vendor_root
            .strip_prefix(&project_dir)
            .unwrap_or(&vendor_root)
            .display()
    );
    println!("To build offline using vendored dependencies, run:\n  beru build --offline");

    Ok(())
}
