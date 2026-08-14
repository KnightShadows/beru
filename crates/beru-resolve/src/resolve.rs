use anyhow::Result;
use beru_core::cache::BeruCache;
use beru_manifest::{BeruLock, BeruManifest, LockedPackage};
use pubgrub::SemanticVersion;
use pubgrub::resolve;
use std::path::PathBuf;

/// Compute the full dependency graph for a manifest using PubGrub.
pub fn resolve_graph(
    manifest: &BeruManifest,
    cache: &BeruCache,
    project_dir: &std::path::Path,
    beru_exe_dir: Option<PathBuf>,
) -> Result<BeruLock> {
    let provider = crate::BeruProvider::new(cache, project_dir, beru_exe_dir.clone());

    for (name, dep) in &manifest.dependencies {
        provider.add_source(name, dep);
    }

    let root_pkg = "root".to_string();
    let root_version = SemanticVersion::new(0, 0, 0);

    let mut root_deps = Vec::new();
    for (name, dep) in &manifest.dependencies {
        let range = match dep.version_string() {
            Some(vs) => crate::version_req_to_range(vs),
            None => pubgrub::Range::full(),
        };
        root_deps.push((name.clone(), range));
    }

    provider.deps_cache.borrow_mut().insert(
        (root_pkg.clone(), root_version),
        pubgrub::Dependencies::Available(root_deps.into_iter().collect()),
    );

    let solution = resolve(&provider, root_pkg, root_version)
        .map_err(|e| anyhow::anyhow!("Dependency resolution failed: {}", e))?;

    let mut locked_packages = Vec::new();
    for (name, version) in solution {
        if name == "root" {
            continue;
        }

        let source_str = if let Some(dep) = provider.sources.borrow().get(&name) {
            dep.source_display()
        } else {
            "bundled".to_string()
        };

        let deps_cache = provider.deps_cache.borrow();
        let deps_keys: Vec<String> = if let Some(pubgrub::Dependencies::Available(deps)) =
            deps_cache.get(&(name.clone(), version))
        {
            deps.iter().map(|(k, _)| k.clone()).collect()
        } else {
            Vec::new()
        };

        let mut checksum = None;
        if let Some(beru_manifest::Dependency::Git(g)) = provider.sources.borrow().get(&name) {
            let pin = g
                .rev
                .as_deref()
                .or(g.tag.as_deref())
                .or(g.branch.as_deref());
            if let Some(p) = pin {
                if let Ok(sha) = resolve_git_tag_to_sha(&g.git, p) {
                    checksum = Some(sha);
                }
            }
        } else if let Ok(Some((recipe, _))) = beru_recipe::resolve_recipe(
            &name,
            Some(&version.to_string()),
            project_dir,
            beru_exe_dir.as_deref(),
            Some(&cache.recipes_dir()),
            Some(&cache.index_dir()),
        ) {
            if let Some(sha) = &recipe.source.sha256 {
                checksum = Some(sha.clone());
            } else if let (Some(git_url), Some(git_tag)) = (&recipe.source.git, &recipe.source.tag)
            {
                if let Ok(sha) = resolve_git_tag_to_sha(git_url, git_tag) {
                    checksum = Some(sha);
                } else {
                    tracing::warn!(
                        "Failed to resolve git tag {} for {} to a commit SHA",
                        git_tag,
                        name
                    );
                }
            }
        }

        locked_packages.push(LockedPackage {
            name,
            version: version.to_string(),
            source: source_str,
            checksum,
            dependencies: deps_keys,
        });
    }

    locked_packages.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(BeruLock {
        version: 1,
        packages: locked_packages,
    })
}

fn resolve_git_tag_to_sha(url: &str, tag: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("ls-remote")
        .arg(url)
        .arg(tag)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git ls-remote: {}", e))?;

    if !output.status.success() {
        anyhow::bail!(
            "git ls-remote failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // git ls-remote output format: "<sha> \t refs/tags/<tag>"
    // we want the first word.
    let sha = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No sha found for tag {} at {}", tag, url))?;

    Ok(sha.to_string())
}
