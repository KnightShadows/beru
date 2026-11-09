use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Arguments for `beru fmt`.
#[derive(Debug, Args)]
pub struct FmtArgs {
    /// Check if files are formatted without modifying them (exits with code 1 if unformatted)
    #[arg(long)]
    pub check: bool,

    /// Format a specific package in a workspace
    #[arg(short, long)]
    pub package: Option<String>,

    /// Print details about formatted files
    #[arg(short, long)]
    pub verbose: bool,
}

const CXX_EXTENSIONS: &[&str] = &["cpp", "hpp", "cc", "cxx", "h", "c", "hxx", "ixx"];

/// Execute the `beru fmt` command.
pub fn exec(args: FmtArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;

    let clang_format = which::which("clang-format").map_err(|_| {
        anyhow::anyhow!(
            "clang-format not found on PATH.\nPlease install LLVM / clang-format via your package manager:\n  Windows: winget install LLVM.LLVM\n  macOS:   brew install clang-format\n  Linux:   sudo apt-get install -y clang-format\nOr run 'beru setup --install-prereqs'."
        )
    })?;

    let mut target_dirs = Vec::new();

    if let Ok(manifest) = beru_manifest::BeruManifest::from_dir(&project_dir) {
        if let Some(ws) = manifest.workspace {
            if let Some(ref pkg_name) = args.package {
                let mut found = false;
                for member in &ws.members {
                    let member_dir = project_dir.join(member);
                    if let Ok(m) = beru_manifest::BeruManifest::from_dir(&member_dir)
                        && &m.package.name == pkg_name
                    {
                        target_dirs.push(member_dir);
                        found = true;
                        break;
                    }
                }
                if !found {
                    bail!("Package '{}' not found in workspace members", pkg_name);
                }
            } else {
                target_dirs.push(project_dir.clone());
                for member in &ws.members {
                    target_dirs.push(project_dir.join(member));
                }
            }
        } else {
            target_dirs.push(project_dir.clone());
        }
    } else {
        target_dirs.push(project_dir.clone());
    }

    let mut files = Vec::new();
    for dir in &target_dirs {
        collect_cxx_files(dir, &mut files)?;
    }

    // Deduplicate files
    files.sort();
    files.dedup();

    if files.is_empty() {
        println!(
            "{} No C++ files found to format.",
            style("Notice:").yellow().bold()
        );
        return Ok(());
    }

    if args.check {
        println!(
            "{} Checking formatting for {} C++ file(s)...",
            style("Checking").cyan().bold(),
            files.len()
        );

        let mut unformatted = Vec::new();

        for file in &files {
            let status = Command::new(&clang_format)
                .arg("--dry-run")
                .arg("--Werror")
                .arg(file)
                .output()
                .with_context(|| format!("failed to check formatting of {}", file.display()))?;

            if !status.status.success() {
                unformatted.push(file.clone());
                if args.verbose {
                    eprintln!(
                        "  {} Needs formatting: {}",
                        style("FAIL").red().bold(),
                        file.display()
                    );
                }
            } else if args.verbose {
                println!("  {} Formatted: {}", style("OK").green(), file.display());
            }
        }

        if !unformatted.is_empty() {
            eprintln!(
                "\n{} {} file(s) require formatting:",
                style("Error:").red().bold(),
                unformatted.len()
            );
            for f in &unformatted {
                eprintln!("  {}", f.display());
            }
            eprintln!(
                "\nRun {} to format all files in place.",
                style("beru fmt").cyan().bold()
            );
            bail!("Code formatting check failed");
        }

        println!(
            "{} All {} file(s) are properly formatted!",
            style("Success:").green().bold(),
            files.len()
        );
        return Ok(());
    }

    println!(
        "{} {} C++ file(s)...",
        style("Formatting").green().bold(),
        files.len()
    );

    let mut modified_count = 0;

    for file in &files {
        let status = Command::new(&clang_format)
            .arg("-i")
            .arg(file)
            .status()
            .with_context(|| format!("failed to format {}", file.display()))?;

        if status.success() {
            modified_count += 1;
            if args.verbose {
                println!("  {} {}", style("Formatted").green(), file.display());
            }
        } else {
            eprintln!(
                "  {} Failed to format: {}",
                style("WARN").yellow().bold(),
                file.display()
            );
        }
    }

    println!(
        "{} Successfully processed {} file(s).",
        style("Done:").green().bold(),
        modified_count
    );

    Ok(())
}

fn collect_cxx_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if name.starts_with('.')
                || name == "build"
                || name == "_beru_build"
                || name == "target"
                || name == "recipes"
                || name == "vendor"
                || name == "node_modules"
            {
                continue;
            }
            collect_cxx_files(&path, files)?;
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && CXX_EXTENSIONS.contains(&ext.to_lowercase().as_str())
        {
            files.push(path);
        }
    }

    Ok(())
}
