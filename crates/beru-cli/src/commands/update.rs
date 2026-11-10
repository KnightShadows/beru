use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::collections::HashMap;

/// Arguments for `beru update`.
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Optional specific package to update.
    pub package: Option<String>,
}

pub fn exec(args: UpdateArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;
    let manifest_path = project_dir.join("Beru.toml");
    if !manifest_path.exists() {
        bail!("No Beru.toml found in the current directory.");
    }

    let manifest =
        beru_manifest::BeruManifest::from_dir(&project_dir).context("failed to parse Beru.toml")?;

    if let Some(ref target_pkg) = args.package {
        if !manifest.dependencies.contains_key(target_pkg) {
            bail!("Package '{}' is not a dependency in Beru.toml", target_pkg);
        }
        println!(
            "{} {}",
            style("Updating").green().bold(),
            style(target_pkg).cyan().bold()
        );
    } else {
        println!("{} dependencies...", style("Updating").green().bold());
    }

    let lock_path = project_dir.join("Beru.lock");
    let old_versions: HashMap<String, String> = if lock_path.exists() {
        if let Ok(lock) = beru_manifest::BeruLock::from_dir(&project_dir) {
            lock.packages
                .into_iter()
                .map(|p| (p.name, p.version))
                .collect()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    let cache = beru_core::cache::BeruCache::default_location()?;
    let beru_exe = std::env::current_exe().ok();
    let new_lock = beru_resolve::resolve_graph(&manifest, &cache, &project_dir, beru_exe)?;

    let mut updated_any = false;
    for pkg in &new_lock.packages {
        if let Some(target) = &args.package
            && &pkg.name != target
        {
            continue;
        }
        let old_ver = old_versions.get(&pkg.name);
        match old_ver {
            Some(old) if old != &pkg.version => {
                println!(
                    "  {} {} v{} -> v{}",
                    style("Updated").green().bold(),
                    style(&pkg.name).cyan().bold(),
                    style(old).yellow(),
                    style(&pkg.version).green()
                );
                updated_any = true;
            }
            None => {
                println!(
                    "  {} {} v{}",
                    style("Added").green().bold(),
                    style(&pkg.name).cyan().bold(),
                    style(&pkg.version).green()
                );
                updated_any = true;
            }
            _ => {}
        }
    }

    if !updated_any {
        if let Some(target) = &args.package {
            println!(
                "  {} is already at the latest compatible version.",
                style(target).cyan()
            );
        } else {
            println!("  All dependencies are up to date.");
        }
    }

    beru_core::fs::write_str_if_changed(&lock_path, &new_lock.to_string()?)
        .context("failed to write Beru.lock")?;

    Ok(())
}
