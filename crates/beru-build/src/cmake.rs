use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

fn cached_cmake_path() -> Result<std::path::PathBuf> {
    beru_core::fs::which_cached("cmake")
        .context("cmake not found on PATH. Install CMake (https://cmake.org) to use Beru.")
}

fn run_inherited(mut cmd: Command, action: &str) -> Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("failed to run {action}"))?;

    if !status.success() {
        bail!("{action} failed (exit code: {:?})", status.code());
    }
    Ok(())
}

/// Invoke CMake to configure a project.
///
/// Runs: `cmake -S <source_dir> -B <build_dir> [extra_args...]`
pub fn cmake_configure(
    source_dir: &Path,
    build_dir: &Path,
    install_prefix: &Path,
    extra_args: &[String],
    toolchain_file: Option<&Path>,
) -> Result<()> {
    let cmake = cached_cmake_path()?;

    let mut pass_toolchain_flag = toolchain_file.is_some();
    let cache_file = build_dir.join("CMakeCache.txt");
    if cache_file.exists() {
        if is_cmake_cache_valid(&cache_file, source_dir, toolchain_file) {
            pass_toolchain_flag = false;
        } else {
            let _ = std::fs::remove_file(&cache_file);
            let cmake_files = build_dir.join("CMakeFiles");
            if cmake_files.exists() {
                let _ = std::fs::remove_dir_all(&cmake_files);
            }
        }
    }

    let mut cmd = Command::new(&cmake);
    cmd.arg("-S").arg(source_dir);
    cmd.arg("-B").arg(build_dir);
    cmd.arg(format!(
        "-DCMAKE_INSTALL_PREFIX={}",
        install_prefix.display()
    ));

    if pass_toolchain_flag && let Some(tc) = toolchain_file {
        cmd.arg(format!("-DCMAKE_TOOLCHAIN_FILE={}", tc.display()));
    }

    for arg in extra_args {
        cmd.arg(arg);
    }

    info!(
        "configuring: cmake -S {} -B {}",
        source_dir.display(),
        build_dir.display()
    );
    debug!("full command: {:?}", cmd);

    run_inherited(cmd, "cmake configure")
}

/// Invoke CMake to build a configured project.
///
/// Runs: `cmake --build <build_dir> --parallel [--config <build_type>]`
pub fn cmake_build(build_dir: &Path, target: Option<&str>, build_type: Option<&str>) -> Result<()> {
    let cmake = cached_cmake_path()?;

    info!("building: cmake --build {}", build_dir.display());

    let mut cmd = Command::new(&cmake);
    cmd.arg("--build").arg(build_dir);

    if let Ok(jobs) = std::env::var("BERU_JOBS") {
        cmd.arg("--parallel").arg(jobs);
    } else {
        cmd.arg("--parallel");
    }

    if let Some(t) = target {
        cmd.arg("--target").arg(t);
    }
    if let Some(bt) = build_type {
        cmd.arg("--config").arg(bt);
    }

    run_inherited(cmd, "cmake build")
}

/// Invoke CMake to install built artifacts.
///
/// Runs: `cmake --install <build_dir> [--config <build_type>]`
pub fn cmake_install(build_dir: &Path, build_type: Option<&str>) -> Result<()> {
    let cmake = cached_cmake_path()?;

    info!("installing: cmake --install {}", build_dir.display());

    let mut cmd = Command::new(&cmake);
    cmd.arg("--install").arg(build_dir);
    if let Some(bt) = build_type {
        cmd.arg("--config").arg(bt);
    }

    run_inherited(cmd, "cmake install")
}

/// Full build pipeline for a dependency using CMake:
/// configure → build → install into the cache.
pub fn build_dependency_cmake(
    source_dir: &Path,
    install_prefix: &Path,
    cmake_args: &[String],
    toolchain_file: Option<&Path>,
    profile: &str,
) -> Result<()> {
    let build_dir = source_dir.join("_beru_build");

    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)
            .with_context(|| format!("failed to clean {}", build_dir.display()))?;
    }

    let cmake_build_type = match profile {
        "debug" => "Debug",
        "release" => "Release",
        "relwithdebinfo" => "RelWithDebInfo",
        "minsizerel" => "MinSizeRel",
        other => other,
    };

    let mut extra_args = cmake_args.to_vec();
    extra_args.push(format!("-DCMAKE_BUILD_TYPE={}", cmake_build_type));

    // Disable test suites, examples, and docs for dependencies by default
    let default_disables = [
        "-DBUILD_TESTING=OFF",
        "-DBUILD_TESTS=OFF",
        "-DFMT_TEST=OFF",
        "-DFMT_DOC=OFF",
        "-DSPDLOG_BUILD_TESTS=OFF",
        "-DSPDLOG_BUILD_EXAMPLE=OFF",
        "-DCATCH_BUILD_TESTING=OFF",
        "-Dgtest_build_tests=OFF",
        "-Dgmock_build_tests=OFF",
    ];
    for flag in &default_disables {
        let key = flag.split('=').next().unwrap();
        if !extra_args.iter().any(|arg| arg.starts_with(key)) {
            extra_args.push(flag.to_string());
        }
    }

    cmake_configure(
        source_dir,
        &build_dir,
        install_prefix,
        &extra_args,
        toolchain_file,
    )?;

    cmake_build(&build_dir, None, Some(cmake_build_type))?;
    cmake_install(&build_dir, Some(cmake_build_type))?;

    Ok(())
}

/// Build the user's project using CMake.
///
/// This generates a toolchain file in the project's build directory,
/// then runs configure + build.
pub fn build_project(
    project_dir: &Path,
    build_dir: &Path,
    toolchain_file: &Path,
    profile: &str,
    target: Option<&str>,
    extra_args: &[String],
) -> Result<()> {
    cmake_configure(
        project_dir,
        build_dir,
        build_dir,
        extra_args,
        Some(toolchain_file),
    )?;
    let cmake_build_type = match profile {
        "debug" => "Debug",
        "release" => "Release",
        "relwithdebinfo" => "RelWithDebInfo",
        "minsizerel" => "MinSizeRel",
        other => other,
    };
    cmake_build(build_dir, target, Some(cmake_build_type))?;

    // Export compile_commands.json to project root for clangd / LSP editors
    let compile_commands = build_dir.join("compile_commands.json");
    if compile_commands.exists() {
        let dest = project_dir.join("compile_commands.json");
        let _ = std::fs::copy(&compile_commands, &dest);
    }

    Ok(())
}

fn is_cmake_cache_valid(
    cache_file: &Path,
    source_dir: &Path,
    toolchain_file: Option<&Path>,
) -> bool {
    let Ok(content) = std::fs::read_to_string(cache_file) else {
        return false;
    };

    let mut home_matches = false;
    let mut toolchain_matches = toolchain_file.is_none();

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("CMAKE_HOME_DIRECTORY:INTERNAL=") {
            home_matches = cmake_paths_match(Path::new(val.trim()), source_dir);
        } else if let Some(tc) = toolchain_file
            && let Some(val) = line
                .strip_prefix("CMAKE_TOOLCHAIN_FILE:FILEPATH=")
                .or_else(|| line.strip_prefix("CMAKE_TOOLCHAIN_FILE:PATH="))
                .or_else(|| line.strip_prefix("CMAKE_TOOLCHAIN_FILE:STRING="))
        {
            toolchain_matches = cmake_paths_match(Path::new(val.trim()), tc);
        }
    }

    home_matches && toolchain_matches
}

fn cmake_paths_match(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| -> String {
        let resolved = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        let raw = resolved.to_string_lossy();
        let stripped = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
        let fwd = stripped
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_string();
        if cfg!(windows) {
            fwd.to_ascii_lowercase()
        } else {
            fwd
        }
    };
    norm(a) == norm(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmake_cache_validation() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src_proj");
        std::fs::create_dir_all(&src).unwrap();
        let tc = src.join("beru-toolchain.cmake");
        std::fs::write(&tc, "# tc").unwrap();
        let cache = dir.path().join("CMakeCache.txt");

        let norm_src = src.canonicalize().unwrap();
        let norm_src_s = norm_src
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/");
        let norm_tc_s = tc
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/");

        std::fs::write(
            &cache,
            format!(
                "CMAKE_HOME_DIRECTORY:INTERNAL={}\nCMAKE_TOOLCHAIN_FILE:FILEPATH={}\n",
                norm_src_s, norm_tc_s
            ),
        )
        .unwrap();
        assert!(is_cmake_cache_valid(&cache, &src, Some(&tc)));

        std::fs::write(
            &cache,
            format!("CMAKE_HOME_DIRECTORY:INTERNAL={}\n", norm_src_s),
        )
        .unwrap();
        assert!(!is_cmake_cache_valid(&cache, &src, Some(&tc)));
    }
}
