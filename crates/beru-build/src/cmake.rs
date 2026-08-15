use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

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
    let cmake = which::which("cmake")
        .context("cmake not found on PATH. Install CMake (https://cmake.org) to use Beru.")?;

    let mut cmd = Command::new(&cmake);
    cmd.arg("-S").arg(source_dir);
    cmd.arg("-B").arg(build_dir);
    cmd.arg(format!(
        "-DCMAKE_INSTALL_PREFIX={}",
        install_prefix.display()
    ));

    if let Some(tc) = toolchain_file {
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

    let output = cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .with_context(|| "failed to run cmake configure")?;

    if !output.status.success() {
        bail!(
            "cmake configure failed (exit code: {:?})",
            output.status.code()
        );
    }

    Ok(())
}

/// Invoke CMake to build a configured project.
///
/// Runs: `cmake --build <build_dir> --parallel [--config <build_type>]`
pub fn cmake_build(build_dir: &Path, target: Option<&str>, build_type: Option<&str>) -> Result<()> {
    let cmake = which::which("cmake").context("cmake not found on PATH")?;

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

    let output = cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("failed to run cmake build")?;

    if !output.status.success() {
        bail!("cmake build failed (exit code: {:?})", output.status.code());
    }

    Ok(())
}

/// Invoke CMake to install built artifacts.
///
/// Runs: `cmake --install <build_dir> [--config <build_type>]`
pub fn cmake_install(build_dir: &Path, build_type: Option<&str>) -> Result<()> {
    let cmake = which::which("cmake").context("cmake not found on PATH")?;

    info!("installing: cmake --install {}", build_dir.display());

    let mut cmd = Command::new(&cmake);
    cmd.arg("--install").arg(build_dir);
    if let Some(bt) = build_type {
        cmd.arg("--config").arg(bt);
    }

    let output = cmd
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("failed to run cmake install")?;

    if !output.status.success() {
        bail!(
            "cmake install failed (exit code: {:?})",
            output.status.code()
        );
    }

    Ok(())
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
    Ok(())
}
