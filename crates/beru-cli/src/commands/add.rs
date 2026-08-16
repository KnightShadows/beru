use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::fs;
use toml_edit::{DocumentMut, InlineTable, Item, Table, Value, value};

/// Arguments for `beru add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// The name of the package to add.
    pub package: String,

    /// Specifies the exact semantic version of the package.
    #[arg(long)]
    pub version: Option<String>,

    /// Specifies a Git repository URL instead of fetching from the registry.
    #[arg(long)]
    pub git: Option<String>,

    /// Specifies the Git tag to checkout.
    #[arg(long)]
    pub tag: Option<String>,

    /// Specifies the exact Git commit hash to checkout.
    #[arg(long)]
    pub rev: Option<String>,

    /// Specifies the branch to checkout.
    #[arg(long)]
    pub branch: Option<String>,

    /// Specifies a local filesystem path to a dependency.
    #[arg(long)]
    pub path: Option<String>,
}

pub fn exec(args: AddArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;
    let manifest_path = project_dir.join("Beru.toml");

    if !manifest_path.exists() {
        bail!("No Beru.toml found in the current directory.");
    }

    // Read the file content
    let content = fs::read_to_string(&manifest_path).context("failed to read Beru.toml")?;

    // Parse it into a mutable AST
    let mut doc = content
        .parse::<DocumentMut>()
        .context("failed to parse Beru.toml as AST")?;

    // Ensure [dependencies] table exists
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = Item::Table(Table::new());
    }

    let deps = doc["dependencies"]
        .as_table_mut()
        .context("[dependencies] must be a table")?;

    // Determine what to add — support `name@version` shorthand
    let (name, version_from_at) = if let Some(pos) = args.package.find('@') {
        let (n, v) = args.package.split_at(pos);
        (n.to_string(), Some(v[1..].to_string()))
    } else {
        (args.package, None)
    };

    // Explicit --version flag takes precedence over @version shorthand
    let effective_version = args.version.or(version_from_at);

    if args.git.is_some() || args.path.is_some() {
        let mut inline = InlineTable::new();
        if let Some(git) = args.git {
            inline.insert("git", git.into());
            if let Some(tag) = args.tag {
                inline.insert("tag", tag.into());
            }
            if let Some(rev) = args.rev {
                inline.insert("rev", rev.into());
            }
            if let Some(branch) = args.branch {
                inline.insert("branch", branch.into());
            }
        } else if let Some(path) = args.path {
            inline.insert("path", path.into());
        }

        deps.insert(&name, Item::Value(Value::InlineTable(inline)));
        println!(
            "{} dependency {} to Beru.toml",
            style("Added").green().bold(),
            style(&name).cyan().bold()
        );
    } else {
        let version = effective_version.unwrap_or_else(|| "*".to_string());
        deps.insert(&name, value(&version));
        println!(
            "{} dependency {} v{} to Beru.toml",
            style("Added").green().bold(),
            style(&name).cyan().bold(),
            style(&version).yellow()
        );
    }

    // Write it back to disk
    fs::write(&manifest_path, doc.to_string()).context("failed to write Beru.toml")?;

    Ok(())
}
