use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

/// Build a dependency using a custom script or set of commands.
pub fn build_dependency_custom(
    source_dir: &Path,
    install_prefix: &Path,
    commands: &[String],
) -> Result<()> {
    info!("building with custom commands in {}", source_dir.display());

    let jobs = std::env::var("BERU_JOBS").unwrap_or_else(|_| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .to_string()
    });

    let install_dir_str = install_prefix.to_string_lossy();

    for cmd_str in commands {
        let expanded = cmd_str
            .replace("{install_dir}", &install_dir_str)
            .replace("{jobs}", &jobs);

        debug!("running custom command: {}", expanded);

        let sh = which::which("sh").context("sh not found on PATH for custom build")?;

        let output = Command::new(&sh)
            .arg("-c")
            .arg(&expanded)
            .current_dir(source_dir)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .output()
            .with_context(|| format!("failed to run custom command: {}", expanded))?;

        if !output.status.success() {
            bail!(
                "custom command failed (exit code: {:?}): {}",
                output.status.code(),
                expanded
            );
        }
    }

    Ok(())
}
