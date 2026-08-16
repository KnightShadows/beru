use anyhow::{Context, Result};
use clap::Args;
use console::style;
use std::fs;
use tracing::info;

/// Arguments for `beru clean`.
#[derive(Debug, Args)]
pub struct CleanArgs {}

pub fn exec(_args: CleanArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;

    let build_dir = project_dir.join("build");
    let toolchain_file = project_dir.join("beru-toolchain.cmake");
    let override_file = project_dir.join("beru-override.cmake");

    let mut cleaned_something = false;

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).context("failed to remove build directory")?;
        info!("Removed build directory");
        cleaned_something = true;
    }

    if toolchain_file.exists() {
        fs::remove_file(&toolchain_file).context("failed to remove toolchain file")?;
        info!("Removed toolchain file");
        cleaned_something = true;
    }

    if override_file.exists() {
        fs::remove_file(&override_file).context("failed to remove override file")?;
        info!("Removed override file");
        cleaned_something = true;
    }

    if cleaned_something {
        println!("{} project", style("Cleaned").green().bold());
    } else {
        println!("{} Nothing to clean", style("Skipped").yellow().bold());
    }

    Ok(())
}
