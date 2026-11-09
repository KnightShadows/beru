use anyhow::{Result, bail};
use console::style;
use std::path::{Path, PathBuf};

use beru_core::cache::BeruCache;
use beru_manifest::Dependency;
use beru_recipe::{beru_exe_dir, resolve_recipe};

/// Resolve a single dependency: find its source, build it, cache the result.
/// Returns the install prefix path for the built dependency.
pub fn resolve_and_build_locked_dep(
    pkg: &beru_manifest::LockedPackage,
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

    let source_dir = match opt_dep {
        Some(Dependency::Git(_)) | Some(Dependency::Path(_)) => {
            fetch_dependency_source(name, opt_dep.unwrap(), pkg, cache, project_dir)?
        }
        Some(Dependency::Registry(_)) | Some(Dependency::Version(_)) | None => {
            let recipe = resolve_recipe(
                name,
                Some(version),
                project_dir,
                beru_exe_dir().as_deref(),
                Some(&cache.recipes_dir()),
                Some(&cache.index_dir()),
            )?;

            if let Some((r, _)) = recipe {
                let src = r.source;
                if let Some(url) = src.url {
                    if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
                        let sha = pkg.checksum.clone().or(src.sha256).ok_or_else(|| {
                            anyhow::anyhow!(
                                "sha256 checksum missing for tarball of package '{}' ({})",
                                name,
                                url
                            )
                        })?;
                        let extracted = beru_recipe::fetch_tarball(cache, &url, &sha)?;
                        beru_recipe::find_source_root(&extracted)?
                    } else {
                        let pin = pkg
                            .checksum
                            .as_deref()
                            .or(src.tag.as_deref())
                            .or(Some(version));
                        beru_recipe::fetch_git(cache, &url, pin)?
                    }
                } else if let Some(git) = src.git {
                    let pin = pkg
                        .checksum
                        .as_deref()
                        .or(src.tag.as_deref())
                        .or(Some(version));
                    beru_recipe::fetch_git(cache, &git, pin)?
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
    };

    let recipe = resolve_recipe(
        name,
        Some(version),
        project_dir,
        beru_exe_dir().as_deref(),
        Some(&cache.recipes_dir()),
        Some(&cache.index_dir()),
    )?;

    println!("  {} {}...", style("Building").green().bold(), name,);

    std::fs::create_dir_all(&install_prefix)?;

    if let Some((ref r, _)) = recipe {
        if r.build.system == "custom" {
            crate::build_dependency_custom(&source_dir, &install_prefix, &r.build.commands)?;
        } else {
            crate::build_dependency_cmake(
                &source_dir,
                &install_prefix,
                &r.build.cmake_args,
                None,
                profile,
            )?;
        }
    } else {
        crate::build_dependency_cmake(&source_dir, &install_prefix, &[], None, profile)?;
    }

    Ok(install_prefix)
}

/// Fetch a dependency's source code (from Git or a local path).
pub fn fetch_dependency_source(
    name: &str,
    dep: &Dependency,
    pkg: &beru_manifest::LockedPackage,
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

        let pkg = beru_manifest::LockedPackage {
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
}
