#![warn(missing_docs)]
//! Beru CLI — The command line interface for the Beru package manager.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

mod commands;

/// Beru — A modern C++ package manager
#[derive(Debug, Parser)]
#[command(name = "beru", version, about = "A modern C++ package manager")]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,

    /// Number of parallel jobs to use for building
    #[arg(short = 'j', long, global = true)]
    jobs: Option<usize>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("BERU_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    if let Some(jobs) = cli.jobs {
        std::env::set_var("BERU_JOBS", jobs.to_string());
    }

    commands::run(cli.command)
}
