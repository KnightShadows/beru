use crate::{build_project, generate_toolchain_cmake, prepare_locked_dependencies};
use anyhow::Result;
use beru_core::cache::BeruCache;
use beru_manifest::BeruManifest;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Build (and cache) an ad-hoc single-file script as an isolated CMake project.
/// Never touches any file outside `cache_dir` and the returned build directory within it.
pub fn build_adhoc(
    entry_file: &Path,
    manifest: &BeruManifest,
    profile: &str,
    cache: &BeruCache,
) -> Result<PathBuf> {
    let entry_file_contents = std::fs::read_to_string(entry_file)?;

    let abi_profile = beru_core::toolchain::build_abi_profile(
        &manifest.package.cxx_std,
        profile,
        manifest.build.shared_libs,
        vec![],
    )?;

    let project_dir = entry_file
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                Path::new(".")
            } else {
                p
            }
        })
        .unwrap_or_else(|| Path::new("."));
    let lockfile =
        beru_resolve::resolve_graph(manifest, cache, project_dir, std::env::current_exe().ok())?;

    let mut hasher = Sha256::new();
    hasher.update(b"abi:");
    hasher.update(abi_profile.hash().as_bytes());
    hasher.update(b"\nsource:");
    hasher.update(entry_file_contents.as_bytes());
    hasher.update(b"\ndeps:");
    for pkg in &lockfile.packages {
        hasher.update(pkg.name.as_bytes());
        hasher.update(b"@");
        hasher.update(pkg.version.as_bytes());
    }
    let hash = beru_core::fs::hex_encode(&hasher.finalize());

    let build_dir = cache.adhoc_build_dir(&hash);
    let binary_name = entry_file.file_stem().unwrap().to_string_lossy();
    let sanitized_name = sanitize_cmake_identifier(&binary_name);
    let binary_path = build_dir.join(if cfg!(windows) {
        format!("{}.exe", sanitized_name)
    } else {
        sanitized_name.to_string()
    });

    if binary_path.exists() {
        println!("  cache hit: {}", binary_path.display());
        return Ok(binary_path);
    }

    let abi_hash = abi_profile.hash();
    let (prefix_paths, cmake_deps) =
        prepare_locked_dependencies(&lockfile, manifest, cache, &abi_hash, project_dir, profile)?;

    std::fs::create_dir_all(&build_dir)?;

    let absolute_entry = entry_file
        .canonicalize()
        .unwrap_or_else(|_| entry_file.to_path_buf());
    let mut absolute_entry_str = absolute_entry.to_string_lossy().into_owned();
    if absolute_entry_str.starts_with("\\\\?\\") {
        absolute_entry_str = absolute_entry_str[4..].to_string();
    }
    let absolute_entry_str = absolute_entry_str.replace("\\", "/");

    let cmakelists = format!(
        "cmake_minimum_required(VERSION 3.20)\n\
        project(adhoc-script)\n\
        set(CMAKE_RUNTIME_OUTPUT_DIRECTORY \"${{CMAKE_BINARY_DIR}}\")\n\
        set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_DEBUG \"${{CMAKE_BINARY_DIR}}\")\n\
        set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_RELEASE \"${{CMAKE_BINARY_DIR}}\")\n\
        set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_RELWITHDEBINFO \"${{CMAKE_BINARY_DIR}}\")\n\
        set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_MINSIZEREL \"${{CMAKE_BINARY_DIR}}\")\n\
        add_executable({} \"{}\")\n\
        beru_link_dependencies({})\n",
        sanitized_name, absolute_entry_str, sanitized_name
    );
    beru_core::fs::write_str_if_changed(&build_dir.join("CMakeLists.txt"), &cmakelists)?;

    let toolchain_file = build_dir.join("beru-toolchain.cmake");
    let prefix_refs: Vec<&Path> = prefix_paths.iter().map(|p| p.as_path()).collect();
    generate_toolchain_cmake(
        &toolchain_file,
        &manifest.package.cxx_std,
        profile,
        &prefix_refs,
        &cmake_deps,
    )?;

    build_project(&build_dir, &build_dir, &toolchain_file, profile, None, &[])?;

    println!("  binary: {}", binary_path.display());
    Ok(binary_path)
}

/// A valid CMake target identifier derived from a script's filename stem. CMake target names
/// must start with a letter/underscore and are safest restricted to alphanumerics/underscores —
/// arbitrary script filenames (spaces, dots beyond the extension, unicode) don't guarantee that.
fn sanitize_cmake_identifier(stem: &str) -> String {
    let mut out = String::from("adhoc_");
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    out
}
