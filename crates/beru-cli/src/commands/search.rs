use anyhow::Result;
use beru_recipe::{Recipe, RecipePackage};
use clap::Args;
use console::style;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Arguments for `beru search`.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search term to match against package names and descriptions
    pub query: String,

    /// Maximum number of search results to display
    #[arg(long, default_value_t = 25)]
    pub limit: usize,
}

#[derive(Debug, Clone)]
struct SearchEntry {
    name: String,
    latest_version: String,
    package_type: String,
    description: String,
}

/// Execute the `beru search` command.
pub fn exec(args: SearchArgs) -> Result<()> {
    let query_lower = if args.query.is_ascii() {
        String::new()
    } else {
        args.query.to_lowercase()
    };
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let mut catalog: BTreeMap<String, SearchEntry> = BTreeMap::new();

    // 1. Ingest embedded recipes
    for (name, content) in beru_recipe::EMBEDDED_RECIPES {
        if let Ok(pkg) = Recipe::parse_package_metadata(content) {
            record_stub(&mut catalog, pkg);
        } else {
            catalog.insert(
                (*name).to_string(),
                SearchEntry {
                    name: (*name).to_string(),
                    latest_version: "embedded".to_string(),
                    package_type: "library".to_string(),
                    description: String::new(),
                },
            );
        }
    }

    // 2. Ingest bundled recipes adjacent to executable
    if let Some(exe_dir) = beru_recipe::beru_exe_dir() {
        let bundled_dir = exe_dir.join("recipes");
        scan_recipes_dir(&bundled_dir, &mut catalog);
    }

    // 3. Ingest global cache recipes & index
    if let Ok(cache) = beru_core::cache::BeruCache::default_location() {
        scan_recipes_dir(&cache.recipes_dir(), &mut catalog);
        scan_index_dir(&cache.index_dir(), &mut catalog);
    }

    // 4. Ingest project-local recipes
    let local_recipes = current_dir.join("recipes");
    scan_recipes_dir(&local_recipes, &mut catalog);

    let mut matches: Vec<&SearchEntry> = catalog
        .values()
        .filter(|entry| {
            contains_case_insensitive(&entry.name, &args.query, &query_lower)
                || contains_case_insensitive(&entry.description, &args.query, &query_lower)
        })
        .collect();

    matches.sort_by(|a, b| {
        let a_exact = equals_case_insensitive(&a.name, &args.query, &query_lower);
        let b_exact = equals_case_insensitive(&b.name, &args.query, &query_lower);
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    if matches.is_empty() {
        println!(
            "{} No packages matching '{}' found in registry index or recipes.",
            style("Notice:").yellow().bold(),
            args.query
        );
        println!("Tip: Run 'beru index update' to fetch or refresh the package catalog.");
        return Ok(());
    }

    let total = matches.len();
    let display_count = total.min(args.limit);
    let displayed_matches = &matches[..display_count];

    println!(
        "\n{} {} package(s) matching '{}':\n",
        style("Found").green().bold(),
        total,
        args.query
    );

    let max_name_len = displayed_matches
        .iter()
        .map(|e| e.name.len())
        .max()
        .unwrap_or(7)
        .max(7);
    let max_ver_len = displayed_matches
        .iter()
        .map(|e| e.latest_version.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let max_type_len = displayed_matches
        .iter()
        .map(|e| e.package_type.len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!(
        "  {:<name_w$}  {:<ver_w$}  {:<type_w$}  {}",
        style("Package").bold(),
        style("Latest").bold(),
        style("Type").bold(),
        style("Description").bold(),
        name_w = max_name_len,
        ver_w = max_ver_len,
        type_w = max_type_len
    );

    println!(
        "  {}  {}  {}  {}",
        "-".repeat(max_name_len),
        "-".repeat(max_ver_len),
        "-".repeat(max_type_len),
        "-".repeat(35)
    );

    for entry in displayed_matches {
        let desc = if entry.description.is_empty() {
            "-"
        } else {
            entry.description.as_str()
        };

        println!(
            "  {:<name_w$}  {:<ver_w$}  {:<type_w$}  {}",
            style(&entry.name).cyan().bold(),
            style(&entry.latest_version).green(),
            style(&entry.package_type).dim(),
            desc,
            name_w = max_name_len,
            ver_w = max_ver_len,
            type_w = max_type_len
        );
    }

    if total > display_count {
        println!(
            "\n... and {} more. Use --limit to show more results.",
            total - display_count
        );
    }
    println!();

    Ok(())
}

fn contains_case_insensitive(haystack: &str, query: &str, query_unicode_lower: &str) -> bool {
    if query.is_ascii() && haystack.is_ascii() {
        beru_core::fs::contains_ascii_case_insensitive(haystack.as_bytes(), query.as_bytes())
    } else if query_unicode_lower.is_empty() {
        haystack.to_lowercase().contains(&query.to_lowercase())
    } else {
        haystack.to_lowercase().contains(query_unicode_lower)
    }
}

fn equals_case_insensitive(text: &str, query: &str, query_unicode_lower: &str) -> bool {
    if query.is_ascii() && text.is_ascii() {
        text.eq_ignore_ascii_case(query)
    } else if query_unicode_lower.is_empty() {
        text.to_lowercase() == query.to_lowercase()
    } else {
        text.to_lowercase() == query_unicode_lower
    }
}

fn record_stub(catalog: &mut BTreeMap<String, SearchEntry>, pkg: RecipePackage) {
    let desc = pkg.description.unwrap_or_default();

    if let Some(existing) = catalog.get_mut(&pkg.name) {
        if is_version_newer(&pkg.version, &existing.latest_version) {
            existing.latest_version = pkg.version;
            if !desc.is_empty() {
                existing.description = desc;
            }
            existing.package_type = pkg.package_type;
        }
    } else {
        catalog.insert(
            pkg.name.clone(),
            SearchEntry {
                name: pkg.name,
                latest_version: pkg.version,
                package_type: pkg.package_type,
                description: desc,
            },
        );
    }
}

fn is_version_newer(new_ver: &str, old_ver: &str) -> bool {
    parse_semver(new_ver) >= parse_semver(old_ver)
}

fn parse_semver(s: &str) -> (u64, u64, u64) {
    let clean = s.trim_start_matches('v');
    let mut parts = clean.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|p| p.split('-').next().unwrap_or("0").parse().ok())
        .unwrap_or(0);
    (major, minor, patch)
}

fn scan_recipes_dir(dir: &Path, catalog: &mut BTreeMap<String, SearchEntry>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let recipe_file = entry.path().join("recipe.toml");
        if let Ok(meta) = Recipe::read_package_metadata(&recipe_file) {
            record_stub(catalog, meta);
        }
    }
}

fn scan_index_dir(dir: &Path, catalog: &mut BTreeMap<String, SearchEntry>) {
    let Ok(pkg_entries) = fs::read_dir(dir) else {
        return;
    };
    for pkg_entry in pkg_entries.flatten() {
        if !pkg_entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let pkg_name_os = pkg_entry.file_name();
        if pkg_name_os.to_string_lossy().starts_with('.') {
            continue;
        }

        let Ok(ver_entries) = fs::read_dir(pkg_entry.path()) else {
            continue;
        };

        // Check only the newest versions first instead of parsing every historical recipe.toml.
        let mut versions: Vec<((u64, u64, u64), std::path::PathBuf)> = ver_entries
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|ft| ft.is_dir()))
            .map(|e| {
                let ver_name = e.file_name();
                let semver = parse_semver(&ver_name.to_string_lossy());
                (semver, e.path())
            })
            .collect();

        versions.sort_unstable_by_key(|a| std::cmp::Reverse(a.0));

        for (_, ver_dir) in versions {
            let recipe_file = ver_dir.join("recipe.toml");
            if let Ok(meta) = Recipe::read_package_metadata(&recipe_file) {
                record_stub(catalog, meta);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_index_dir_picks_latest_version_first() {
        let dir = tempfile::tempdir().unwrap();
        for (ver, desc) in [("1.0.0", "old"), ("2.5.1", "latest"), ("1.9.0", "mid")] {
            let ver_dir = dir.path().join("coolpkg").join(ver);
            fs::create_dir_all(&ver_dir).unwrap();
            fs::write(
                ver_dir.join("recipe.toml"),
                format!(
                    "[package]\nname = \"coolpkg\"\nversion = \"{ver}\"\ndescription = \"{desc}\"\n[source]\nurl = \"https://example.com/a.tar.gz\"\n"
                ),
            )
            .unwrap();
        }

        let mut catalog = BTreeMap::new();
        scan_index_dir(dir.path(), &mut catalog);
        let entry = catalog.get("coolpkg").unwrap();
        assert_eq!(entry.latest_version, "2.5.1");
        assert_eq!(entry.description, "latest");
    }

    #[test]
    fn test_contains_case_insensitive() {
        assert!(contains_case_insensitive("Nlohmann_JSON", "json", ""));
        assert!(!contains_case_insensitive("fmt", "spdlog", ""));
    }
}
