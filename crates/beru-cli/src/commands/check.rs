use anyhow::{Context, Result};
use clap::Args;
use console::style;

use crate::commands::build::{prepare_project_toolchain, resolve_target};
use beru_build::build_project;
use beru_core::cache::BeruCache;
use beru_manifest::BeruManifest;

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
                "cmake_minimum_required(VERSION 3.20)\nproject({} LANGUAGES CXX)\n\nadd_executable({} src/{}.cpp)\nif(COMMAND beru_link_dependencies)\n    beru_link_dependencies({})\nendif()\n",
                resolved_target_name, resolved_target_name, target_stem, resolved_target_name
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

    let toolchain_path =
        prepare_project_toolchain(&project_dir, &manifest, &cache, &args.profile, false)?;

    let build_dir = project_dir.join("build");

    let override_path = project_dir.join("beru-override.cmake");
    beru_core::fs::write_str_if_changed(
        &override_path,
        "set(CMAKE_CXX_LINK_EXECUTABLE \"cmake -E echo\" CACHE STRING \"\" FORCE)\nset(CMAKE_C_LINK_EXECUTABLE \"cmake -E echo\" CACHE STRING \"\" FORCE)\n",
    )
    .context("failed to write override.cmake")?;

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
