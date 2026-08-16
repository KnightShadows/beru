use anyhow::{Context, Result};
use clap::Args;
use console::style;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::commands::build::resolve_target;
use beru_build::resolve_and_build_locked_dep;
use beru_build::{build_project, generate_toolchain_cmake};
use beru_core::cache::BeruCache;
use beru_core::toolchain;
use beru_manifest::BeruManifest;
use beru_recipe::beru_exe_dir;

/// Arguments for `beru check`.
#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Build profile to use
    #[arg(long, default_value = "debug")]
    pub profile: String,

    /// Optional target filename (e.g., day1.cpp)
    pub target: Option<String>,
}

pub fn exec(args: CheckArgs) -> Result<()> {
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
        "{} {} v{} ({}) [syntax-only]",
        style("Checking").blue().bold(),
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

        let recipe = beru_recipe::resolve_recipe(
            &pkg.name,
            Some(&pkg.version),
            &project_dir,
            beru_exe_dir().as_deref(),
            Some(&cache.recipes_dir()),
            Some(&cache.index_dir()),
        )?;

        let package_name = if let Some((ref r, _)) = recipe {
            r.export
                .cmake_package
                .clone()
                .or_else(|| Some(pkg.name.clone()))
        } else {
            Some(pkg.name.clone())
        };

        let mut targets = if let Some((ref r, _)) = recipe {
            let mut t = r.export.cmake_targets.clone();
            if t.is_empty() {
                t = r.export.link_libs.clone();
            }
            t
        } else {
            Vec::new()
        };

        if targets.is_empty() {
            targets.push(format!("{}::{}", pkg.name, pkg.name));
        }

        cmake_deps.push(beru_build::CMakeDependency {
            package_name,
            targets,
        });
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

    let override_path = project_dir.join("beru-override.cmake");
    std::fs::write(
        &override_path,
        "set(CMAKE_CXX_LINK_EXECUTABLE \"cmake -E echo\" CACHE STRING \"\" FORCE)\nset(CMAKE_C_LINK_EXECUTABLE \"cmake -E echo\" CACHE STRING \"\" FORCE)\n",
    ).context("failed to write override.cmake")?;

    let override_arg = format!(
        "-DCMAKE_USER_MAKE_RULES_OVERRIDE={}",
        override_path.display().to_string().replace("\\", "/")
    );

    let extra_args = vec![
        "-DCMAKE_CXX_FLAGS=-fsyntax-only".to_string(),
        "-DCMAKE_C_FLAGS=-fsyntax-only".to_string(),
        "-DCMAKE_CXX_COMPILER_WORKS=1".to_string(),
        "-DCMAKE_C_COMPILER_WORKS=1".to_string(),
        "-DCMAKE_CXX_COMPILER_FORCED=1".to_string(),
        "-DCMAKE_C_COMPILER_FORCED=1".to_string(),
        override_arg,
    ];

    build_project(
        &project_dir,
        &build_dir,
        &toolchain_path,
        &args.profile,
        Some(resolved_target_name.as_str()),
        &extra_args,
    )?;

    println!(
        "  {} {} checked successfully",
        style("Finished").green().bold(),
        resolved_target_name,
    );

    Ok(())
}
