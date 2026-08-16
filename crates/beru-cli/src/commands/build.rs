use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use beru_build::{build_project, generate_toolchain_cmake, resolve_and_build_locked_dep};
use beru_core::cache::BeruCache;
use beru_core::toolchain;
use beru_manifest::BeruManifest;
use beru_recipe::{beru_exe_dir, resolve_recipe};

/// Arguments for `beru build`.
#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Build profile to use
    #[arg(long, default_value = "debug")]
    pub profile: String,

    /// Optional target filename (e.g., day1.cpp)
    pub target: Option<String>,
}

pub fn exec(args: BuildArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;

    let manifest = BeruManifest::from_dir(&project_dir)
        .context("failed to parse Beru.toml (are you in a Beru project directory?)")?;

    let mut resolved_target_name = manifest.package.name.clone();

    if manifest.package.package_type == beru_manifest::PackageType::Executable {
        let (target_stem, show_warning) = resolve_target(&project_dir, args.target.as_deref())?;

        if target_stem != "main" {
            resolved_target_name = target_stem.clone();
        }

        if show_warning {
            println!(
                "{} Multiple files found but no 'main.cpp'. Defaulting to '{}.cpp'.",
                style("Warning:").yellow().bold(),
                target_stem
            );
        }

        let cmakelists_path = project_dir.join("CMakeLists.txt");
        if !cmakelists_path.exists() {
            let cmake_content = format!(
                "cmake_minimum_required(VERSION 3.20)\nproject({} LANGUAGES CXX)\n\nadd_executable({} src/{}.cpp)\n",
                target_stem, target_stem, target_stem
            );
            std::fs::write(cmakelists_path, cmake_content)
                .context("failed to write dynamic CMakeLists.txt")?;
        }
    }

    println!(
        "{} {} v{} ({})",
        style("Building").green().bold(),
        resolved_target_name,
        manifest.package.version,
        manifest.package.cxx_std,
    );

    let cache = BeruCache::default_location()?;
    cache.ensure_dirs()?;

    let abi_profile = toolchain::build_abi_profile(
        &manifest.package.cxx_std,
        &args.profile,
        manifest.build.shared_libs,
        vec![],
    )?;

    let abi_hash = abi_profile.hash();
    info!("ABI profile: {}", abi_profile);
    debug!("ABI hash: {}", abi_hash);

    let lock_path = project_dir.join("Beru.lock");
    let lockfile = if lock_path.exists() {
        let existing =
            beru_manifest::BeruLock::from_dir(&project_dir).context("Failed to parse Beru.lock")?;
        let is_stale = manifest
            .dependencies
            .keys()
            .any(|name| !existing.packages.iter().any(|pkg| &pkg.name == name));
        if is_stale {
            info!("Beru.lock is out of date, resolving dependencies...");
            let beru_exe = std::env::current_exe().ok();
            let generated = beru_resolve::resolve_graph(&manifest, &cache, &project_dir, beru_exe)?;
            std::fs::write(&lock_path, generated.to_string()?)
                .context("Failed to write Beru.lock")?;
            generated
        } else {
            existing
        }
    } else {
        info!("Beru.lock not found, resolving dependencies...");
        let beru_exe = std::env::current_exe().ok();
        let generated = beru_resolve::resolve_graph(&manifest, &cache, &project_dir, beru_exe)?;
        std::fs::write(&lock_path, generated.to_string()?).context("Failed to write Beru.lock")?;
        generated
    };

    let mut prefix_paths: Vec<PathBuf> = Vec::new();
    let mut cmake_deps: Vec<beru_build::CMakeDependency> = Vec::new();

    if !lockfile.packages.is_empty() {
        println!("{} dependencies...", style("Building").cyan().bold(),);
    }

    for pkg in &lockfile.packages {
        let opt_dep = manifest.dependencies.get(&pkg.name);

        let install_prefix = resolve_and_build_locked_dep(
            pkg,
            opt_dep,
            &cache,
            &abi_hash,
            &project_dir,
            &args.profile,
        )?;
        prefix_paths.push(install_prefix);

        let recipe = resolve_recipe(
            &pkg.name,
            Some(&pkg.version),
            &project_dir,
            beru_exe_dir().as_deref(),
            Some(&cache.recipes_dir()),
            Some(&cache.index_dir()),
        )?;

        if let Some((r, _)) = recipe {
            let mut targets = r.export.cmake_targets.clone();
            if targets.is_empty() {
                targets = r.export.link_libs.clone();
            }
            cmake_deps.push(beru_build::CMakeDependency {
                package_name: r.export.cmake_package.clone(),
                targets,
            });
        }
    }

    let toolchain_path = project_dir.join("beru-toolchain.cmake");
    let prefix_refs: Vec<&Path> = prefix_paths.iter().map(|p| p.as_path()).collect();
    generate_toolchain_cmake(
        &toolchain_path,
        &manifest.package.cxx_std,
        &args.profile,
        &prefix_refs,
        &cmake_deps,
    )?;

    let build_dir = project_dir.join("build");
    println!("{} project...", style("Compiling").green().bold(),);
    build_project(
        &project_dir,
        &build_dir,
        &toolchain_path,
        &args.profile,
        Some(resolved_target_name.as_str()),
        &[],
    )?;

    println!(
        "  {} {} built successfully",
        style("Finished").green().bold(),
        resolved_target_name,
    );

    Ok(())
}

/// Resolve the correct target executable file from the src/ directory.
pub fn resolve_target(project_dir: &Path, target_arg: Option<&str>) -> Result<(String, bool)> {
    let src_dir = project_dir.join("src");
    if !src_dir.exists() {
        bail!("src/ directory not found in project");
    }

    let mut cpp_files = Vec::new();
    for entry in std::fs::read_dir(&src_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("cpp")
            && let Some(name) = path.file_stem().and_then(|s| s.to_str())
        {
            cpp_files.push(name.to_string());
        }
    }

    if cpp_files.is_empty() {
        bail!("no .cpp files found in src/");
    }

    // Scenario B: User explicitly requested a target
    if let Some(t) = target_arg {
        let stem = t.strip_suffix(".cpp").unwrap_or(t);
        if cpp_files.contains(&stem.to_string()) {
            return Ok((stem.to_string(), false));
        } else {
            bail!("target file '{}.cpp' not found in src/", stem);
        }
    }

    // Scenario A: No arguments
    if cpp_files.len() == 1 {
        // Exactly 1 file
        return Ok((cpp_files[0].clone(), false));
    }

    if cpp_files.contains(&"main".to_string()) {
        // Multiple files, main.cpp exists
        return Ok(("main".to_string(), false));
    }

    // Multiple files, no main.cpp
    cpp_files.sort();
    let default_target = cpp_files[0].clone();

    Ok((default_target, true))
}
