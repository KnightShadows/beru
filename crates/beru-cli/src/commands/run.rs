use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::process::Command;

use beru_manifest::BeruManifest;

/// Arguments for `beru run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Build profile to use
    #[arg(long, default_value = "debug")]
    pub profile: String,

    /// Arguments to pass to the executable
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

pub fn exec(args: RunArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;
    let mut run_args = args.args.clone();

    // Determine if the first trailing arg is a source file (root-relative or bare).
    if let Some(first) = run_args.first().cloned() {
        let candidate = resolve_script_path(&project_dir, &first);
        if let Some(script_path) = candidate {
            run_args.remove(0);
            let source = std::fs::read_to_string(&script_path)
                .with_context(|| format!("failed to read {}", script_path.display()))?;

            let cache = beru_core::cache::BeruCache::default_location()?;
            cache.ensure_dirs()?;

            let effective_manifest = match beru_manifest::extract_inline_manifest(&source)? {
                Some(inline) => inline,
                None => match BeruManifest::from_dir(&project_dir) {
                    Ok(project_manifest) => {
                        eprintln!(
                            "{} running script using surrounding project's dependencies (no inline `/// beru` block found)",
                            style("Warning:").yellow().bold()
                        );
                        project_manifest
                    }
                    Err(_) => beru_manifest::default_adhoc_manifest(),
                },
            };

            let binary =
                beru_build::build_adhoc(&script_path, &effective_manifest, &args.profile, &cache)?;

            println!(
                "  {} `{}`\n",
                style("Running").green().bold(),
                script_path.display()
            );
            let status = Command::new(&binary)
                .args(&run_args)
                .status()
                .with_context(|| format!("failed to run {}", binary.display()))?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
            return Ok(());
        } else if first.ends_with(".cpp") || first.starts_with("src/") || first.starts_with("src\\")
        {
            bail!("Source file '{}' not found in project.", first);
        }
    }

    let manifest = BeruManifest::from_dir(&project_dir).context("failed to parse Beru.toml")?;

    if manifest.package.package_type != beru_manifest::PackageType::Executable {
        bail!(
            "`beru run` is only for executable projects. This project is type '{}'.",
            manifest.package.package_type
        );
    }

    let target = None;

    let (resolved_target, _) = super::build::resolve_target(&project_dir, target.as_deref())?;
    let mut actual_target_name = resolved_target.clone();
    if resolved_target == "main" {
        actual_target_name = manifest.package.name.clone();
    }

    let build_args = super::build::BuildArgs {
        profile: args.profile.clone(),
        target: target.clone(),
    };
    super::build::exec(build_args)?;

    let build_dir = project_dir.join("build");

    let exe_path = find_executable(&build_dir, &actual_target_name)?;

    println!(
        "  {} `{}`\n",
        style("Running").green().bold(),
        actual_target_name,
    );

    let status = Command::new(&exe_path)
        .args(&run_args)
        .status()
        .with_context(|| format!("failed to run {}", exe_path.display()))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Search for the built executable in common CMake output locations.
fn find_executable(build_dir: &std::path::Path, name: &str) -> Result<std::path::PathBuf> {
    let candidates = [
        build_dir.join(name),
        build_dir.join(format!("{name}.exe")),
        build_dir.join("Debug").join(name),
        build_dir.join("Debug").join(format!("{name}.exe")),
        build_dir.join("Release").join(name),
        build_dir.join("Release").join(format!("{name}.exe")),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    if let Some(found) = find_file_recursive(build_dir, name) {
        return Ok(found);
    }

    bail!(
        "could not find executable '{}' in {}. Was the build successful?",
        name,
        build_dir.display()
    )
}

/// Recursively search for a file by name in a directory.
fn find_file_recursive(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                && (file_name == name || file_name == format!("{name}.exe"))
            {
                return Some(path);
            }
        } else if path.is_dir()
            && let Some(found) = find_file_recursive(&path, name)
        {
            return Some(found);
        }
    }
    None
}

fn resolve_script_path(
    project_dir: &std::path::Path,
    first_arg: &str,
) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(first_arg);
    if path.extension().and_then(|s| s.to_str()) == Some("cpp") && path.exists() {
        return Some(path.to_path_buf());
    }

    let stem = first_arg.strip_suffix(".cpp").unwrap_or(first_arg);
    let root_file = project_dir.join(format!("{}.cpp", stem));
    if root_file.exists() {
        return Some(root_file);
    }

    let src_file = project_dir.join("src").join(format!("{}.cpp", stem));
    if src_file.exists() {
        return Some(src_file);
    }

    None
}
