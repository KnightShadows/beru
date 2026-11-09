use anyhow::Result;
use clap::Subcommand;

pub mod add;
pub mod build;
pub mod cache;
pub mod check;
pub mod clean;
pub mod index;
pub mod init;
pub mod install;
pub mod new;
pub mod outdated;
pub mod remove;
pub mod resolve;
pub mod run;
pub mod test;
pub mod tree;
pub mod update;

/// Top-level Beru commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new Beru C++ project
    New(new::NewArgs),

    /// Initialize Beru in an existing directory
    Init(init::InitArgs),

    /// Build the project
    Build(build::BuildArgs),

    /// Build and run the project (executables only)
    Run(run::RunArgs),

    /// Resolve dependencies and generate Beru.lock
    Resolve(resolve::ResolveArgs),

    /// Manage the package registry index
    Index(index::IndexArgs),

    /// Remove build artifacts
    Clean(clean::CleanArgs),

    /// Manage global cache
    Cache(cache::CacheArgs),

    /// Add a dependency to Beru.toml
    Add(add::AddArgs),

    /// Remove a dependency from Beru.toml
    #[command(alias = "rm")]
    Remove(remove::RemoveArgs),

    /// Update dependencies in Beru.lock to their latest compatible versions
    Update(update::UpdateArgs),

    /// Display outdated dependencies compared to the registry
    Outdated(outdated::OutdatedArgs),

    /// Display a tree of dependencies
    Tree(tree::TreeArgs),

    /// Check the project for errors without producing a binary
    Check(check::CheckArgs),

    /// Run the project's tests
    Test(test::TestArgs),

    /// Install a binary tool locally into ~/.beru/bin
    Install(install::InstallArgs),

    /// Uninstall a binary tool from ~/.beru/bin
    Uninstall(install::UninstallArgs),
}

/// Dispatch a command.
pub fn run(cmd: Command) -> Result<()> {
    match cmd {
        Command::New(args) => new::exec(args),
        Command::Init(args) => init::exec(args),
        Command::Build(args) => build::exec(args),
        Command::Run(args) => run::exec(args),
        Command::Resolve(args) => resolve::exec(args),
        Command::Index(args) => index::exec(args),
        Command::Clean(args) => clean::exec(args),
        Command::Cache(args) => cache::exec(args),
        Command::Add(args) => add::exec(args),
        Command::Remove(args) => remove::exec(args),
        Command::Update(args) => update::exec(args),
        Command::Outdated(args) => outdated::exec(args),
        Command::Tree(args) => tree::exec(args),
        Command::Check(args) => check::exec(args),
        Command::Test(args) => test::exec(args),
        Command::Install(args) => install::exec_install(args),
        Command::Uninstall(args) => install::exec_uninstall(args),
    }
}
