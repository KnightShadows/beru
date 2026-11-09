use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

use crate::commands::build::{BuildArgs, exec as build_exec};

/// Arguments for `beru test`.
#[derive(Debug, Args)]
pub struct TestArgs {
    /// Build profile to use
    #[arg(long, default_value = "debug")]
    pub profile: String,

    /// Specific package to test in a workspace
    #[arg(short, long)]
    pub package: Option<String>,

    /// Filter test names by regex pattern (passed to ctest -R)
    #[arg(short = 'R', long = "filter")]
    pub filter: Option<String>,
}

pub fn exec(args: TestArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;

    // First, build the project or workspace
    build_exec(BuildArgs {
        profile: args.profile.clone(),
        target: None,
        package: args.package.clone(),
    })?;

    let manifest = beru_manifest::BeruManifest::from_dir(&project_dir)?;

    // Handle workspace testing
    if let Some(ref ws) = manifest.workspace {
        if let Some(ref pkg_name) = args.package {
            let mut found = false;
            for member in &ws.members {
                let member_dir = project_dir.join(member);
                if !member_dir.exists() {
                    continue;
                }
                let member_manifest = beru_manifest::BeruManifest::from_dir(&member_dir)?;
                if &member_manifest.package.name == pkg_name {
                    found = true;
                    println!(
                        "{} tests for workspace package '{}'...",
                        style("Running").cyan().bold(),
                        pkg_name
                    );
                    run_tests_in_dir(&member_dir, &args.profile, args.filter.as_deref())?;
                    break;
                }
            }
            if !found {
                bail!("Package '{}' not found in workspace members", pkg_name);
            }
            return Ok(());
        } else if !ws.members.is_empty() {
            println!(
                "{} tests across {} workspace member(s)...",
                style("Running").cyan().bold(),
                ws.members.len()
            );
            for member in &ws.members {
                let member_dir = project_dir.join(member);
                if !member_dir.exists() {
                    continue;
                }
                let member_name = beru_manifest::BeruManifest::from_dir(&member_dir)
                    .map(|m| m.package.name)
                    .unwrap_or_else(|_| member.clone());

                println!(
                    "  {} tests for member '{}'...",
                    style("Running").cyan().bold(),
                    member_name
                );
                run_tests_in_dir(&member_dir, &args.profile, args.filter.as_deref())?;
            }
            return Ok(());
        }
    }

    // Single project testing
    println!("{} tests...", style("Running").cyan().bold());
    run_tests_in_dir(&project_dir, &args.profile, args.filter.as_deref())
}

fn run_tests_in_dir(dir: &Path, profile: &str, filter: Option<&str>) -> Result<()> {
    let build_dir = dir.join("build");
    if !build_dir.exists() {
        bail!(
            "Build directory not found at '{}'. Build failed?",
            build_dir.display()
        );
    }

    let ctest_file = build_dir.join("CTestTestfile.cmake");
    if !ctest_file.exists() {
        println!(
            "  {} No tests configured for {}",
            style("Skipped").yellow().bold(),
            dir.display()
        );
        return Ok(());
    }

    let ctest =
        which::which("ctest").context("ctest not found on PATH. Install CMake to run tests.")?;

    let mut cmd = Command::new(&ctest);
    cmd.current_dir(&build_dir);
    cmd.arg("--output-on-failure");

    let profile_capitalized = {
        let mut chars = profile.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    };
    cmd.arg("-C").arg(&profile_capitalized);

    if let Some(f) = filter {
        cmd.arg("-R").arg(f);
    }

    if let Ok(parallelism) = std::thread::available_parallelism() {
        cmd.arg("-j").arg(parallelism.get().to_string());
    } else {
        cmd.arg("-j").arg("1");
    }

    info!("running tests: ctest in {}", build_dir.display());
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
