use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use std::path::Path;
use tracing::{debug, info};

use beru_build::{
    build_project, generate_toolchain_cmake_with_config, prepare_locked_dependencies,
};
use beru_core::cache::BeruCache;
use beru_core::toolchain;
use beru_manifest::BeruManifest;

/// Arguments for `beru build`.
#[derive(Debug, Args, Clone)]
pub struct BuildArgs {
    /// Build profile to use
    #[arg(long, default_value = "debug")]
    pub profile: String,

    /// Optional target filename (e.g., day1.cpp)
    pub target: Option<String>,

    /// Specific package to build in a workspace
    #[arg(short, long)]
    pub package: Option<String>,
}

pub fn exec(args: BuildArgs) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to get current directory")?;
    exec_in_dir(&project_dir, args)
}

pub fn exec_in_dir(project_dir: &Path, args: BuildArgs) -> Result<()> {
    let cache = BeruCache::default_location()?;
    cache.ensure_dirs()?;
    exec_in_dir_with_cache(project_dir, args, &cache)
}

fn exec_in_dir_with_cache(project_dir: &Path, args: BuildArgs, cache: &BeruCache) -> Result<()> {
    let manifest = BeruManifest::from_dir(project_dir)
        .context("failed to parse Beru.toml (are you in a Beru project directory?)")?;

    // Handle workspace root
    if let Some(ref ws) = manifest.workspace {
        if let Some(ref pkg_name) = args.package {
            let mut found = false;
            for member in &ws.members {
                let member_dir = project_dir.join(member);
                if !member_dir.exists() {
                    continue;
                }
                let member_manifest = BeruManifest::from_dir(&member_dir)?;
                if &member_manifest.package.name == pkg_name {
                    found = true;
                    let mut member_args = args.clone();
                    member_args.package = None;
                    exec_in_dir_with_cache(&member_dir, member_args, cache)?;
                    break;
                }
            }
            if !found {
                bail!("Package '{}' not found in workspace members", pkg_name);
            }
            return Ok(());
        } else if !ws.members.is_empty() {
            println!(
                "{} workspace with {} member(s)...",
                style("Building").green().bold(),
                ws.members.len()
            );
            for member in &ws.members {
                let member_dir = project_dir.join(member);
                if !member_dir.exists() {
                    bail!("Workspace member directory '{}' does not exist", member);
                }
                let mut member_args = args.clone();
                member_args.package = None;
                exec_in_dir_with_cache(&member_dir, member_args, cache)?;
            }
            return Ok(());
        }
    }

    let mut resolved_target_name = manifest.package.name.clone();

    let cmakelists_path = project_dir.join("CMakeLists.txt");
    if !cmakelists_path.exists() {
        let cmake_content = match manifest.package.package_type {
            beru_manifest::PackageType::Executable => {
                let (target_stem, show_warning) =
                    resolve_target(project_dir, args.target.as_deref())?;
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
                format!(
                    "cmake_minimum_required(VERSION 3.20)\nproject({} LANGUAGES CXX)\n\nadd_executable({} src/{}.cpp)\nif(COMMAND beru_link_dependencies)\n    beru_link_dependencies({})\nendif()\n",
                    resolved_target_name, resolved_target_name, target_stem, resolved_target_name
                )
            }
            beru_manifest::PackageType::HeaderOnly => {
                format!(
                    "cmake_minimum_required(VERSION 3.20)\nproject({0} LANGUAGES CXX)\n\nadd_library({0} INTERFACE)\ntarget_include_directories({0} INTERFACE include)\nif(EXISTS \"${{CMAKE_CURRENT_SOURCE_DIR}}/include\")\n    install(DIRECTORY include/ DESTINATION include)\nendif()\n",
                    resolved_target_name
                )
            }
            beru_manifest::PackageType::Library => {
                format!(
                    "cmake_minimum_required(VERSION 3.20)\nproject({0} LANGUAGES CXX)\n\nfile(GLOB_RECURSE SOURCES src/*.cpp src/*.cxx src/*.cc)\nif(SOURCES)\n    add_library({0} ${{SOURCES}})\n    target_include_directories({0} PUBLIC include)\n    if(COMMAND beru_link_dependencies)\n        beru_link_dependencies({0})\n    endif()\n    install(TARGETS {0} ARCHIVE DESTINATION lib LIBRARY DESTINATION lib RUNTIME DESTINATION bin)\nelse()\n    add_library({0} INTERFACE)\n    target_include_directories({0} INTERFACE include)\nendif()\nif(EXISTS \"${{CMAKE_CURRENT_SOURCE_DIR}}/include\")\n    install(DIRECTORY include/ DESTINATION include)\nendif()\n",
                    resolved_target_name
                )
            }
        };
        std::fs::write(cmakelists_path, cmake_content)
            .context("failed to write dynamic CMakeLists.txt")?;
    } else if manifest.package.package_type == beru_manifest::PackageType::Executable {
        let (target_stem, _) = resolve_target(project_dir, args.target.as_deref())?;
        if target_stem != "main" {
            resolved_target_name = target_stem;
        }
    }

    println!(
        "{} {} v{} ({})",
        style("Building").green().bold(),
        resolved_target_name,
        manifest.package.version,
        manifest.package.cxx_std,
    );

    let toolchain_path =
        prepare_project_toolchain(project_dir, &manifest, cache, &args.profile, true)?;

    let build_dir = project_dir.join("build");
    println!("{} project...", style("Compiling").green().bold(),);
    build_project(
        project_dir,
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

/// Resolve lockfile (updating if missing or stale), prepare cached dependencies,
/// and synthesize `beru-toolchain.cmake` in `project_dir`.
pub(crate) fn prepare_project_toolchain(
    project_dir: &Path,
    manifest: &BeruManifest,
    cache: &BeruCache,
    profile: &str,
    print_deps_banner: bool,
) -> Result<std::path::PathBuf> {
    let abi_profile = toolchain::build_abi_profile(
        &manifest.package.cxx_std,
        profile,
        manifest.build.shared_libs,
        vec![],
    )?;

    let abi_hash = abi_profile.hash();
    info!("ABI profile: {}", abi_profile);
    debug!("ABI hash: {}", abi_hash);

    let lock_path = project_dir.join("Beru.lock");
    let lockfile = if lock_path.exists() {
        let existing =
            beru_manifest::BeruLock::from_dir(project_dir).context("Failed to parse Beru.lock")?;
        let is_stale = manifest
            .dependencies
            .keys()
            .any(|name| !existing.packages.iter().any(|pkg| &pkg.name == name));
        if is_stale {
            info!("Beru.lock is out of date, resolving dependencies...");
            let beru_exe = std::env::current_exe().ok();
            let generated = beru_resolve::resolve_graph(manifest, cache, project_dir, beru_exe)?;
            beru_core::fs::write_str_if_changed(&lock_path, &generated.to_string()?)
                .context("Failed to write Beru.lock")?;
            generated
        } else {
            existing
        }
    } else {
        info!("Beru.lock not found, resolving dependencies...");
        let beru_exe = std::env::current_exe().ok();
        let generated = beru_resolve::resolve_graph(manifest, cache, project_dir, beru_exe)?;
        beru_core::fs::write_str_if_changed(&lock_path, &generated.to_string()?)
            .context("Failed to write Beru.lock")?;
        generated
    };

    if print_deps_banner && !lockfile.packages.is_empty() {
        println!("{} dependencies...", style("Building").cyan().bold());
    }

    let (prefix_paths, cmake_deps) =
        prepare_locked_dependencies(&lockfile, manifest, cache, &abi_hash, project_dir, profile)?;

    let profile_cfg = manifest.profile.get(profile);
    let opt_str = profile_cfg.and_then(|p| p.optimization.as_ref().map(|o| o.to_string()));
    let warnings = profile_cfg.and_then(|p| p.warnings.as_deref());
    let lto = profile_cfg.map(|p| p.lto).unwrap_or(false);
    let sanitizers = profile_cfg.map(|p| p.sanitizers.as_slice()).unwrap_or(&[]);
    let flags = profile_cfg.map(|p| p.flags.as_slice()).unwrap_or(&[]);

    let sys_deps: Vec<beru_build::SystemCMakeDependency> = manifest
        .system_dependencies
        .iter()
        .map(|(name, dep)| beru_build::SystemCMakeDependency {
            package_name: dep.cmake_package(),
            targets: dep.cmake_targets(name),
            pkg_config: dep.pkg_config().map(|s| s.to_string()),
            required: dep.is_required(),
        })
        .collect();

    let toolchain_config = beru_build::ToolchainConfig {
        optimization: opt_str.as_deref(),
        warnings,
        lto,
        sanitizers,
        flags,
        system_deps: &sys_deps,
    };

    let toolchain_path = project_dir.join("beru-toolchain.cmake");
    let prefix_refs: Vec<&Path> = prefix_paths.iter().map(|p| p.as_path()).collect();
    generate_toolchain_cmake_with_config(
        &toolchain_path,
        &manifest.package.cxx_std,
        profile,
        &prefix_refs,
        &cmake_deps,
        &toolchain_config,
    )?;

    Ok(toolchain_path)
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
