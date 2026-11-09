use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::fs;
use toml_edit::DocumentMut;

/// Arguments for `beru remove`.
#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// The name of the package to remove.
    pub package: String,
}

pub fn exec(args: RemoveArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;
    let manifest_path = project_dir.join("Beru.toml");

    if !manifest_path.exists() {
        bail!("No Beru.toml found in the current directory.");
    }

    let content = fs::read_to_string(&manifest_path).context("failed to read Beru.toml")?;
    let mut doc = content
        .parse::<DocumentMut>()
        .context("failed to parse Beru.toml as AST")?;

    let name = &args.package;
    let mut removed = false;

    if let Some(deps) = doc.get_mut("dependencies").and_then(|d| d.as_table_mut())
        && deps.remove(name).is_some()
    {
        removed = true;
    }

    if let Some(dev_deps) = doc
        .get_mut("dev-dependencies")
        .and_then(|d| d.as_table_mut())
        && dev_deps.remove(name).is_some()
    {
        removed = true;
    }

    if !removed {
        bail!("Package '{}' is not present in Beru.toml", name);
    }

    fs::write(&manifest_path, doc.to_string()).context("failed to write Beru.toml")?;

    println!(
        "{} dependency {} from Beru.toml",
        style("Removed").green().bold(),
        style(name).cyan().bold()
    );

    // Update Beru.lock to match updated dependencies
    if let Ok(manifest) = beru_manifest::BeruManifest::from_dir(&project_dir) {
        let lock_path = project_dir.join("Beru.lock");
        if manifest.dependencies.is_empty() {
            let empty_lock = beru_manifest::BeruLock {
                version: 1,
                packages: Vec::new(),
            };
            if let Ok(lock_str) = empty_lock.to_string() {
                let _ = fs::write(&lock_path, lock_str);
            }
        } else if let Ok(cache) = beru_core::cache::BeruCache::default_location()
            && let Ok(lock) = beru_resolve::resolve_graph(
                &manifest,
                &cache,
                &project_dir,
                std::env::current_exe().ok(),
            )
            && let Ok(lock_str) = lock.to_string()
        {
            let _ = fs::write(&lock_path, lock_str);
        }
    }

    Ok(())
}
