use anyhow::{Result, bail};
use console::style;
use std::path::{Path, PathBuf};

use beru_core::cache::BeruCache;
use beru_manifest::{BeruLock, BeruManifest, Dependency, LockedPackage};
use beru_recipe::{Recipe, beru_exe_dir, resolve_recipe};

struct PackagePrep<'a> {
    pkg: &'a LockedPackage,
    opt_dep: Option<&'a Dependency>,
    install_prefix: PathBuf,
    recipe: Option<(Recipe, PathBuf)>,
    is_cached: bool,
    vendored_dir: Option<PathBuf>,
}

/// Resolve, build (if not cached), and inspect all dependencies in a lockfile.
///
/// Concurrent phases overlap network/git fetches for uncached packages and
/// prefix introspection for cached packages while keeping deterministic order
/// and clean build output.
///
/// Returns `(prefix_paths, cmake_dependencies)` ready to pass into toolchain
/// generation.
pub fn prepare_locked_dependencies(
    lockfile: &BeruLock,
    manifest: &BeruManifest,
    cache: &BeruCache,
    abi_hash: &str,
    project_dir: &Path,
    profile: &str,
) -> Result<(Vec<PathBuf>, Vec<crate::CMakeDependency>)> {
    let n = lockfile.packages.len();
    if n == 0 {
        return Ok((Vec::new(), Vec::new()));
    }

    let exe_dir = beru_exe_dir();
    let mut preps = Vec::with_capacity(n);
    let mut cached_indices = Vec::new();
    let mut fetch_indices = Vec::new();

    for (idx, pkg) in lockfile.packages.iter().enumerate() {
        let opt_dep = manifest.dependencies.get(&pkg.name);
        let install_prefix = cache.build_dir(abi_hash, &pkg.name, &pkg.version);
        let is_cached = cache.has_build(abi_hash, &pkg.name, &pkg.version);
        let recipe = find_dep_recipe_with_exe(
            &pkg.name,
            &pkg.version,
            project_dir,
            cache,
            exe_dir.as_deref(),
        )?;

        let vendored_dir = if is_cached {
            println!(
                "  {} {} ({}) — cached",
                style("Using").blue().bold(),
                pkg.name,
                pkg.version,
            );
            cached_indices.push(idx);
            None
        } else {
            println!(
                "  {} {} ({})",
                style("Fetching").yellow().bold(),
                pkg.name,
                pkg.source,
            );
            let vend = project_dir
                .join("vendor")
                .join(format!("{}-{}", pkg.name, pkg.version));
            if vend.exists() {
                Some(vend)
            } else {
                fetch_indices.push(idx);
                None
            }
        };

        preps.push(PackagePrep {
            pkg,
            opt_dep,
            install_prefix,
            recipe,
            is_cached,
            vendored_dir,
        });
    }

    let mut fetched_sources: Vec<Option<PathBuf>> = vec![None; n];
    if fetch_indices.len() == 1 {
        let idx = fetch_indices[0];
        let p = &preps[idx];
        let src = resolve_package_source(
            &p.pkg.name,
            &p.pkg.version,
            p.opt_dep,
            p.pkg,
            p.recipe.as_ref(),
            cache,
            project_dir,
        )?;
        fetched_sources[idx] = Some(src);
    } else if fetch_indices.len() > 1 {
        let results: Vec<(usize, Result<PathBuf>)> = std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(fetch_indices.len());
            for &idx in &fetch_indices {
                let p = &preps[idx];
                handles.push(s.spawn(move || {
                    let res = resolve_package_source(
                        &p.pkg.name,
                        &p.pkg.version,
                        p.opt_dep,
                        p.pkg,
                        p.recipe.as_ref(),
                        cache,
                        project_dir,
                    );
                    (idx, res)
                }));
            }
            handles
                .into_iter()
                .map(|h| h.join().expect("dependency fetch worker panicked"))
                .collect()
        });

        for (idx, res) in results {
            fetched_sources[idx] = Some(res?);
        }
    }

    let mut cmake_deps_slots: Vec<Option<crate::CMakeDependency>> = vec![None; n];

    if cached_indices.len() == 1 {
        let idx = cached_indices[0];
        let p = &preps[idx];
        cmake_deps_slots[idx] = Some(crate::inspect_installed_dependency(
            &p.pkg.name,
            &p.install_prefix,
            p.recipe.as_ref().map(|(r, _)| r),
        ));
    } else if cached_indices.len() > 1 {
        let inspected: Vec<(usize, crate::CMakeDependency)> = std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(cached_indices.len());
            for &idx in &cached_indices {
                let p = &preps[idx];
                handles.push(s.spawn(move || {
                    let dep = crate::inspect_installed_dependency(
                        &p.pkg.name,
                        &p.install_prefix,
                        p.recipe.as_ref().map(|(r, _)| r),
                    );
                    (idx, dep)
                }));
            }
            handles
                .into_iter()
                .map(|h| h.join().expect("dependency inspect worker panicked"))
                .collect()
        });
        for (idx, dep) in inspected {
            cmake_deps_slots[idx] = Some(dep);
        }
    }

    for (idx, p) in preps.iter().enumerate() {
        if p.is_cached {
            continue;
        }
        let source_dir = if let Some(ref vend) = p.vendored_dir {
            println!(
                "  {} {} ({}) — vendored",
                style("Using").cyan().bold(),
                p.pkg.name,
                p.pkg.version
            );
            vend.clone()
        } else {
            fetched_sources[idx]
                .take()
                .expect("uncached non-vendored dependency must have resolved source")
        };

        println!("  {} {}...", style("Building").green().bold(), p.pkg.name);
        build_resolved_dep(
            &source_dir,
            &p.install_prefix,
            p.recipe.as_ref().map(|(r, _)| r),
            profile,
        )?;

        cmake_deps_slots[idx] = Some(crate::inspect_installed_dependency(
            &p.pkg.name,
            &p.install_prefix,
            p.recipe.as_ref().map(|(r, _)| r),
        ));
    }

    let mut prefix_paths = Vec::with_capacity(n);
    let mut cmake_deps = Vec::with_capacity(n);
    for (p, dep_opt) in preps.into_iter().zip(cmake_deps_slots) {
        prefix_paths.push(p.install_prefix);
        cmake_deps.push(dep_opt.expect("every dependency must have inspected metadata"));
    }

    Ok((prefix_paths, cmake_deps))
}

fn find_dep_recipe_with_exe(
    name: &str,
    version: &str,
    project_dir: &Path,
    cache: &BeruCache,
    exe_dir: Option<&Path>,
) -> Result<Option<(Recipe, PathBuf)>> {
    let mut recipe = resolve_recipe(
        name,
        Some(version),
        project_dir,
        exe_dir,
        Some(&cache.recipes_dir()),
        Some(&cache.index_dir()),
    )?;

    let vendored_recipe = project_dir
        .join("vendor")
        .join(format!("{name}-{version}"))
        .join("recipe.toml");
    if recipe.is_none()
        && vendored_recipe.exists()
        && let Ok(r) = Recipe::from_file(&vendored_recipe)
    {
        recipe = Some((r, vendored_recipe));
    }

    Ok(recipe)
}

fn build_resolved_dep(
    source_dir: &Path,
    install_prefix: &Path,
    recipe: Option<&Recipe>,
    profile: &str,
) -> Result<()> {
    std::fs::create_dir_all(install_prefix)?;

    if let Some(r) = recipe {
        if r.build.system == "custom" {
            crate::build_dependency_custom(source_dir, install_prefix, &r.build.commands)?;
        } else {
            crate::build_dependency_cmake(
                source_dir,
                install_prefix,
                &r.build.cmake_args,
                None,
                profile,
            )?;
        }
    } else {
        crate::build_dependency_cmake(source_dir, install_prefix, &[], None, profile)?;
    }

    Ok(())
}

/// Resolve a single dependency: find its source, build it, cache the result.
/// Returns the install prefix path for the built dependency.
pub fn resolve_and_build_locked_dep(
    pkg: &LockedPackage,
    opt_dep: Option<&Dependency>,
    cache: &BeruCache,
    abi_hash: &str,
    project_dir: &Path,
    profile: &str,
) -> Result<PathBuf> {
    let name = &pkg.name;
    let version = &pkg.version;
    let install_prefix = cache.build_dir(abi_hash, name, version);

    if cache.has_build(abi_hash, name, version) {
        println!(
            "  {} {} ({}) — cached",
            style("Using").blue().bold(),
            name,
            version,
        );
        return Ok(install_prefix);
    }

    println!(
        "  {} {} ({})",
        style("Fetching").yellow().bold(),
        name,
        pkg.source,
    );

    let exe_dir = beru_exe_dir();
    let recipe = find_dep_recipe_with_exe(name, version, project_dir, cache, exe_dir.as_deref())?;
    let vendored_dir = project_dir.join("vendor").join(format!("{name}-{version}"));

    let source_dir = if vendored_dir.exists() {
        println!(
            "  {} {} ({}) — vendored",
            style("Using").cyan().bold(),
            name,
            version
        );
        vendored_dir
    } else {
        resolve_package_source(
            name,
            version,
            opt_dep,
            pkg,
            recipe.as_ref(),
            cache,
            project_dir,
        )?
    };

    println!("  {} {}...", style("Building").green().bold(), name);
    build_resolved_dep(
        &source_dir,
        &install_prefix,
        recipe.as_ref().map(|(r, _)| r),
        profile,
    )?;

    Ok(install_prefix)
}

/// Resolve the source directory for a locked dependency package.
pub fn resolve_package_source(
    name: &str,
    version: &str,
    opt_dep: Option<&Dependency>,
    pkg: &LockedPackage,
    recipe: Option<&(Recipe, PathBuf)>,
    cache: &BeruCache,
    project_dir: &Path,
) -> Result<PathBuf> {
    match opt_dep {
        Some(Dependency::Git(_)) | Some(Dependency::Path(_)) => {
            fetch_dependency_source(name, opt_dep.unwrap(), pkg, cache, project_dir)
        }
        Some(Dependency::Registry(_)) | Some(Dependency::Version(_)) | None => {
            if let Some((r, _)) = recipe {
                let src = &r.source;
                if let Some(url) = &src.url {
                    if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
                        let sha = pkg
                            .checksum
                            .clone()
                            .or_else(|| src.sha256.clone())
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "sha256 checksum missing for tarball of package '{}' ({})",
                                    name,
                                    url
                                )
                            })?;
                        let extracted = beru_recipe::fetch_tarball(cache, url, &sha)?;
                        beru_recipe::find_source_root(&extracted)
                    } else {
                        let pin = pkg
                            .checksum
                            .as_deref()
                            .or(src.tag.as_deref())
                            .or(Some(version));
                        beru_recipe::fetch_git(cache, url, pin)
                    }
                } else if let Some(git) = &src.git {
                    let pin = pkg
                        .checksum
                        .as_deref()
                        .or(src.tag.as_deref())
                        .or(Some(version));
                    beru_recipe::fetch_git(cache, git, pin)
                } else {
                    bail!("Recipe for {} has no source", name);
                }
            } else {
                bail!(
                    "Could not find source or recipe for transitive dependency {}",
                    name
                );
            }
        }
    }
}

/// Fetch a dependency's source code (from Git or a local path).
pub fn fetch_dependency_source(
    name: &str,
    dep: &Dependency,
    pkg: &LockedPackage,
    cache: &BeruCache,
    project_dir: &Path,
) -> Result<PathBuf> {
    match dep {
        Dependency::Git(g) => {
            let pin = pkg
                .checksum
                .as_deref()
                .or(g.rev.as_deref())
                .or(g.tag.as_deref())
                .or(g.branch.as_deref());
            let repo_dir = beru_recipe::fetch_git(cache, &g.git, pin)?;
            Ok(repo_dir)
        }
        Dependency::Path(p) => {
            let resolved = if p.path.is_absolute() {
                p.path.clone()
            } else {
                project_dir.join(&p.path)
            };
            if !resolved.exists() {
                bail!(
                    "path dependency '{}' does not exist: {}",
                    name,
                    resolved.display()
                );
            }
            Ok(resolved)
        }
        Dependency::Registry(_) | Dependency::Version(_) => {
            bail!(
                "Registry dependencies cannot be fetched via fetch_dependency_source. Use recipes instead."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_missing_sha256_returns_error_not_panic() {
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path().join("proj");
        let recipe_dir = project_dir.join("recipes").join("no-sha-pkg");
        fs::create_dir_all(&recipe_dir).unwrap();

        fs::write(
            recipe_dir.join("recipe.toml"),
            r#"[package]
name = "no-sha-pkg"
version = "1.0.0"
type = "header-only"

[source]
url = "https://example.com/no-sha-pkg-1.0.0.tar.gz"

[export]
include-dirs = ["include"]
"#,
        )
        .unwrap();

        let cache = BeruCache::with_root(temp.path().join(".beru"));
        cache.ensure_dirs().unwrap();

        let pkg = LockedPackage {
            name: "no-sha-pkg".to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            checksum: None,
            dependencies: vec![],
        };

        let result =
            resolve_and_build_locked_dep(&pkg, None, &cache, "dummy_abi", &project_dir, "debug");

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("sha256 checksum missing for tarball"));
    }

    #[test]
    fn test_prepare_locked_dependencies_multiple_cached_preserves_order() {
        let temp = tempfile::tempdir().unwrap();
        let project_dir = temp.path().join("proj");
        fs::create_dir_all(&project_dir).unwrap();

        let cache = BeruCache::with_root(temp.path().join(".beru"));
        cache.ensure_dirs().unwrap();

        let abi_hash = "abi123";
        for (pkg, ver) in [("alpha", "1.0.0"), ("beta", "2.0.0")] {
            let prefix = cache.build_dir(abi_hash, pkg, ver);
            fs::create_dir_all(prefix.join("include").join(pkg)).unwrap();
            fs::write(prefix.join("include").join(pkg).join("h.hpp"), "// hdr").unwrap();
        }

        let lock = BeruLock {
            version: 1,
            packages: vec![
                LockedPackage {
                    name: "alpha".to_string(),
                    version: "1.0.0".to_string(),
                    source: "registry".to_string(),
                    checksum: None,
                    dependencies: vec![],
                },
                LockedPackage {
                    name: "beta".to_string(),
                    version: "2.0.0".to_string(),
                    source: "registry".to_string(),
                    checksum: None,
                    dependencies: vec![],
                },
            ],
        };

        let manifest =
            BeruManifest::parse_toml("[package]\nname = \"app\"\nversion = \"0.1.0\"\n").unwrap();

        let (prefixes, deps) =
            prepare_locked_dependencies(&lock, &manifest, &cache, abi_hash, &project_dir, "debug")
                .unwrap();

        assert_eq!(prefixes.len(), 2);
        assert_eq!(deps.len(), 2);
        assert!(prefixes[0].ends_with("alpha-1.0.0"));
        assert!(prefixes[1].ends_with("beta-2.0.0"));
    }
}
