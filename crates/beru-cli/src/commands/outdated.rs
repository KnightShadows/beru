use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;

/// Arguments for `beru outdated`.
#[derive(Debug, Args)]
pub struct OutdatedArgs {}

pub fn exec(_args: OutdatedArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;
    let lock_path = project_dir.join("Beru.lock");

    if !lock_path.exists() {
        bail!(
            "No Beru.lock found in the current directory. Run `beru resolve` or `beru build` first."
        );
    }

    let lock =
        beru_manifest::BeruLock::from_dir(&project_dir).context("failed to parse Beru.lock")?;

    if lock.packages.is_empty() {
        println!("No dependencies found in Beru.lock.");
        return Ok(());
    }

    let cache = beru_core::cache::BeruCache::default_location()?;
    let beru_exe = std::env::current_exe().ok();
    let provider = beru_resolve::BeruProvider::new(&cache, &project_dir, beru_exe);

    // If manifest exists, populate sources in provider
    if let Ok(manifest) = beru_manifest::BeruManifest::from_dir(&project_dir) {
        for (name, dep) in &manifest.dependencies {
            provider.add_source(name, dep);
        }
    }

    struct Row {
        name: String,
        current: String,
        latest: String,
        status: String,
        has_update: bool,
    }

    let mut rows: Vec<Row> = Vec::new();

    for pkg in &lock.packages {
        let current_parts = beru_resolve::parse_version_parts(&pkg.version).ok();
        let mut available = provider
            .fetch_available_versions(&pkg.name)
            .unwrap_or_default();
        available.sort();

        let latest_semver = available.last().cloned();
        let latest_str = latest_semver
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| pkg.version.clone());
        let latest_parts = beru_resolve::parse_version_parts(&latest_str).ok();

        let (status, has_update) = match (current_parts, latest_parts) {
            (Some(curr), Some(lat)) if lat > curr => {
                if lat.0 > curr.0 {
                    (
                        style("Update available (major)")
                            .yellow()
                            .bold()
                            .to_string(),
                        true,
                    )
                } else if lat.1 > curr.1 {
                    (
                        style("Update available (minor)").cyan().bold().to_string(),
                        true,
                    )
                } else {
                    (
                        style("Update available (patch)").green().bold().to_string(),
                        true,
                    )
                }
            }
            _ => (style("Up to date").dim().to_string(), false),
        };

        rows.push(Row {
            name: pkg.name.clone(),
            current: pkg.version.clone(),
            latest: latest_str,
            status,
            has_update,
        });
    }

    // Print table with aligned columns
    let name_w = rows
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(7)
        .max("Package".len());
    let curr_w = rows
        .iter()
        .map(|r| r.current.len())
        .max()
        .unwrap_or(7)
        .max("Current".len());
    let lat_w = rows
        .iter()
        .map(|r| r.latest.len())
        .max()
        .unwrap_or(6)
        .max("Latest".len());

    println!(
        "{:<name_w$}  {:<curr_w$}  {:<lat_w$}  Status",
        style("Package").bold(),
        style("Current").bold(),
        style("Latest").bold(),
    );

    let has_any_updates = rows.iter().any(|r| r.has_update);

    for r in rows {
        println!(
            "{:<name_w$}  {:<curr_w$}  {:<lat_w$}  {}",
            style(&r.name).cyan(),
            r.current,
            r.latest,
            r.status,
        );
    }

    if has_any_updates {
        println!();
        println!(
            "Run `{}` to update dependencies.",
            style("beru update").green().bold()
        );
    }

    Ok(())
}
