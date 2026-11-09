use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::process::Command;
use tracing::{debug, info};

use crate::commands::build::{BuildArgs, exec as build_exec};

/// Arguments for `beru test`.
#[derive(Debug, Args)]
pub struct TestArgs {
    /// Build profile to use
    #[arg(long, default_value = "debug")]
    pub profile: String,
}

pub fn exec(args: TestArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;

    // First, build the project
    build_exec(BuildArgs {
        profile: args.profile.clone(),
        target: None,
        package: None,
    })?;

    let build_dir = project_dir.join("build");
    if !build_dir.exists() {
        bail!("Build directory not found. Build failed?");
    }

    println!("{} tests...", style("Running").cyan().bold());

    let ctest =
        which::which("ctest").context("ctest not found on PATH. Install CMake to run tests.")?;

    let mut cmd = Command::new(&ctest);
    cmd.current_dir(&build_dir);
    cmd.arg("--output-on-failure");

    // Capitalize profile for CTest config (e.g., "debug" -> "Debug")
    let profile_capitalized = {
        let mut chars = args.profile.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    };
    cmd.arg("-C").arg(&profile_capitalized);

    // Run tests in parallel based on available parallelism
    if let Ok(parallelism) = std::thread::available_parallelism() {
        cmd.arg("-j").arg(parallelism.get().to_string());
    } else {
        cmd.arg("-j").arg("1");
    }

    info!("running tests: ctest --output-on-failure");
    debug!("full command: {:?}", cmd);

    let output = cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("failed to run ctest")?;

    if !output.status.success() {
        bail!("tests failed (exit code: {:?})", output.status.code());
    }

    Ok(())
}
