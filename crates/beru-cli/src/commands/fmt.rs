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
const MAX_CMD_ARG_BYTES: usize = 16_000;

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

    files.sort();
    files.dedup();

    if files.is_empty() {
        println!(
            "{} No C++ files found to format.",
            style("Notice:").yellow().bold()
        );
        return Ok(());
    }

    let batches = chunk_files_by_cmd_len(&files, MAX_CMD_ARG_BYTES);

    if args.check {
        println!(
            "{} Checking formatting for {} C++ file(s)...",
            style("Checking").cyan().bold(),
            files.len()
        );

        let mut unformatted = Vec::new();

        for batch in batches {
            let status = Command::new(&clang_format)
                .arg("--dry-run")
                .arg("--Werror")
                .args(batch)
                .output()
                .context("failed to run clang-format --dry-run")?;

            if status.status.success() {
                if args.verbose {
                    for file in batch {
                        println!("  {} Formatted: {}", style("OK").green(), file.display());
                    }
                }
                continue;
            }

            // Batch contained unformatted file(s); isolate exact offenders per file.
            for file in batch {
                let single = Command::new(&clang_format)
                    .arg("--dry-run")
                    .arg("--Werror")
                    .arg(file)
                    .output()
                    .with_context(|| format!("failed to check formatting of {}", file.display()))?;

                if !single.status.success() {
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

    for batch in batches {
        let status = Command::new(&clang_format)
            .arg("-i")
            .args(batch)
            .status()
            .context("failed to run clang-format -i")?;

        if status.success() {
            modified_count += batch.len();
            if args.verbose {
                for file in batch {
                    println!("  {} {}", style("Formatted").green(), file.display());
                }
            }
            continue;
        }

        // Fall back to per-file formatting only if a batch failed.
        for file in batch {
            let single = Command::new(&clang_format)
                .arg("-i")
                .arg(file)
                .status()
                .with_context(|| format!("failed to format {}", file.display()))?;

            if single.success() {
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
    }

    println!(
        "{} Successfully processed {} file(s).",
        style("Done:").green().bold(),
        modified_count
    );

    Ok(())
}

/// Partition file paths into argument batches bounded cleanly under OS command-line limits.
fn chunk_files_by_cmd_len(files: &[PathBuf], max_bytes: usize) -> Vec<&[PathBuf]> {
    let mut batches = Vec::new();
    let mut start = 0;
    let mut cur_bytes = 0;

    for (idx, path) in files.iter().enumerate() {
        let len = path.as_os_str().len() + 3;
        if idx > start && cur_bytes + len > max_bytes {
            batches.push(&files[start..idx]);
            start = idx;
            cur_bytes = 0;
        }
        cur_bytes += len;
    }

    if start < files.len() {
        batches.push(&files[start..]);
    }

    batches
}

fn is_cxx_extension(ext: &str) -> bool {
    CXX_EXTENSIONS
        .iter()
        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
}

fn collect_cxx_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_dir() {
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };
            if name_str.starts_with('.')
                || matches!(
                    name_str,
                    "build" | "_beru_build" | "target" | "recipes" | "vendor" | "node_modules"
                )
            {
                continue;
            }
            collect_cxx_files(&entry.path(), files)?;
        } else if ft.is_file() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && is_cxx_extension(ext)
            {
                files.push(path);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_files_by_cmd_len() {
        let files: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("src/file_{i}.cpp")))
            .collect();
        let batches = chunk_files_by_cmd_len(&files, 36);
        assert!(batches.len() >= 2);
        let flattened: Vec<&PathBuf> = batches.into_iter().flatten().collect();
        assert_eq!(flattened.len(), 5);
    }

    #[test]
    fn test_is_cxx_extension_case_insensitive() {
        assert!(is_cxx_extension("cpp"));
        assert!(is_cxx_extension("HPP"));
        assert!(!is_cxx_extension("rs"));
    }
}
