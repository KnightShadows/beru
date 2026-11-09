use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::path::PathBuf;

/// Arguments for `beru install`.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// The name of the package to install (omit to install current project)
    pub package: Option<String>,

    /// Install from a local directory path
    #[arg(long)]
    pub path: Option<String>,
}

/// Arguments for `beru uninstall`.
#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// The name of the package or binary to uninstall
    pub package: String,
}

pub fn exec_install(args: InstallArgs) -> Result<()> {
    let current_dir = std::env::current_dir().context("failed to get current directory")?;
    let project_dir = if let Some(p) = args.path {
        let path = PathBuf::from(&p);
        if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        }
    } else {
        current_dir
    };

    let manifest = beru_manifest::BeruManifest::from_dir(&project_dir).with_context(|| {
        format!(
            "failed to find or parse Beru.toml in {}",
            project_dir.display()
        )
    })?;

    if manifest.package.package_type != beru_manifest::PackageType::Executable {
        bail!(
            "`beru install` is only supported for executable projects (found '{}')",
            manifest.package.package_type
        );
    }

    let target_name = manifest.package.name.clone();

    println!(
        "{} {} v{} ({})",
        style("Installing").green().bold(),
        target_name,
        manifest.package.version,
        project_dir.display()
    );

    // Build project in release mode
    let old_dir = std::env::current_dir()?;
    std::env::set_current_dir(&project_dir)?;
    let build_res = super::build::exec(super::build::BuildArgs {
        profile: "release".to_string(),
        target: None,
        package: None,
    });
    let _ = std::env::set_current_dir(old_dir);
    build_res?;

    let build_dir = project_dir.join("build");
    let built_exe = super::run::find_executable(&build_dir, &target_name)?;

    let cache = beru_core::cache::BeruCache::default_location()?;
    cache.ensure_dirs()?;
    let bin_dir = cache.bin_dir();

    let exe_name = built_exe
        .file_name()
        .context("invalid executable filename")?;
    let dest_path = bin_dir.join(exe_name);

    std::fs::copy(&built_exe, &dest_path).with_context(|| {
        format!(
            "failed to copy binary from {} to {}",
            built_exe.display(),
            dest_path.display()
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest_path, perms)?;
    }

    println!(
        "  {} `{}` (in {})",
        style("Installed").green().bold(),
        target_name,
        dest_path.display()
    );

    // Check if bin_dir is in PATH
    let path_var = std::env::var("PATH").unwrap_or_default();
    let bin_dir_str = bin_dir.to_string_lossy();
    if !path_var.contains(&*bin_dir_str) {
        println!(
            "{} be sure to add `{}` to your PATH to run installed binaries.",
            style("warning:").yellow().bold(),
            bin_dir.display()
        );
    }

    Ok(())
}

pub fn exec_uninstall(args: UninstallArgs) -> Result<()> {
    let cache = beru_core::cache::BeruCache::default_location()?;
    let bin_dir = cache.bin_dir();

    let candidates = [
        bin_dir.join(&args.package),
        bin_dir.join(format!("{}.exe", args.package)),
    ];

    let mut removed = false;
    for candidate in &candidates {
        if candidate.exists() {
            std::fs::remove_file(candidate)
                .with_context(|| format!("failed to remove binary {}", candidate.display()))?;
            removed = true;
        }
    }

    if !removed {
        bail!(
            "Binary '{}' is not installed in {}",
            args.package,
            bin_dir.display()
        );
    }

    println!(
        "{} binary `{}` from {}",
        style("Uninstalled").green().bold(),
        args.package,
        bin_dir.display()
    );

    Ok(())
}
