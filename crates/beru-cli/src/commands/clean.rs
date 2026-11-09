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

    let mut cleaned_something = clean_dir(&project_dir)?;

    if let Ok(manifest) = beru_manifest::BeruManifest::from_dir(&project_dir)
        && let Some(ws) = manifest.workspace
    {
        for member in ws.members {
            let member_dir = project_dir.join(member);
            if member_dir.exists() {
                let cleaned = clean_dir(&member_dir)?;
                cleaned_something = cleaned_something || cleaned;
            }
        }
    }

    if cleaned_something {
        println!("{} project", style("Cleaned").green().bold());
    } else {
        println!("{} Nothing to clean", style("Skipped").yellow().bold());
    }

    Ok(())
}

fn clean_dir(dir: &std::path::Path) -> Result<bool> {
    let build_dir = dir.join("build");
    let toolchain_file = dir.join("beru-toolchain.cmake");
    let override_file = dir.join("beru-override.cmake");

    let mut cleaned_something = false;

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).context("failed to remove build directory")?;
        info!("Removed build directory at {}", build_dir.display());
        cleaned_something = true;
    }

    if toolchain_file.exists() {
        fs::remove_file(&toolchain_file).context("failed to remove toolchain file")?;
        info!("Removed toolchain file at {}", toolchain_file.display());
        cleaned_something = true;
    }

    if override_file.exists() {
        fs::remove_file(&override_file).context("failed to remove override file")?;
        info!("Removed override file at {}", override_file.display());
        cleaned_something = true;
    }

    Ok(cleaned_something)
}
