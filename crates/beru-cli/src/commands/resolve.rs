use anyhow::{Context, Result};
use beru_core::cache::BeruCache;
use beru_manifest::BeruManifest;
use clap::Parser;
use std::env;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Parser)]
pub struct ResolveArgs {
    /// Directory containing Beru.toml (defaults to current directory)
    #[arg(short, long)]
    pub dir: Option<PathBuf>,
}

pub fn exec(args: ResolveArgs) -> Result<()> {
    let current_dir = env::current_dir()?;
    let project_dir = args.dir.unwrap_or(current_dir);

    let manifest = BeruManifest::from_dir(&project_dir).context("Failed to parse Beru.toml")?;

    let cache = BeruCache::default_location().context("Failed to initialize Beru cache")?;

    let beru_exe = env::current_exe().ok();

    info!("Resolving dependencies...");
    let lockfile = beru_resolve::resolve_graph(&manifest, &cache, &project_dir, beru_exe)?;

    let lock_path = project_dir.join("Beru.lock");
    beru_core::fs::write_str_if_changed(&lock_path, &lockfile.to_string()?)
        .context("Failed to write Beru.lock")?;

    info!(
        "Successfully wrote Beru.lock with {} dependencies locked",
        lockfile.packages.len()
    );

    Ok(())
}
