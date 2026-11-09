use anyhow::Result;
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
    let query_lower = args.query.to_lowercase();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let mut catalog: BTreeMap<String, SearchEntry> = BTreeMap::new();

    // 1. Ingest embedded recipes
    for (name, content) in beru_recipe::EMBEDDED_RECIPES {
        if let Ok(recipe) = beru_recipe::Recipe::parse_toml(content) {
            record_recipe(&mut catalog, &recipe);
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

    // Filter by query
    let mut matches: Vec<&SearchEntry> = catalog
        .values()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&query_lower)
                || entry.description.to_lowercase().contains(&query_lower)
        })
        .collect();

    // Sort by name match first, then alphabetically
    matches.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == query_lower;
        let b_exact = b.name.to_lowercase() == query_lower;
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

fn record_recipe(catalog: &mut BTreeMap<String, SearchEntry>, recipe: &beru_recipe::Recipe) {
    let name = recipe.package.name.clone();
    let ver = recipe.package.version.clone();
    let pkg_type = recipe.package.package_type.clone();
    let desc = recipe.package.description.clone().unwrap_or_default();

    if let Some(existing) = catalog.get_mut(&name) {
        if is_version_newer(&ver, &existing.latest_version) {
            existing.latest_version = ver;
            if !desc.is_empty() {
                existing.description = desc;
            }
            existing.package_type = pkg_type;
        }
    } else {
        catalog.insert(
            name.clone(),
            SearchEntry {
                name,
                latest_version: ver,
                package_type: pkg_type,
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
    let parts: Vec<&str> = clean.split('.').collect();
    let major = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts
        .get(2)
        .and_then(|p| p.split('-').next().unwrap_or("0").parse().ok())
        .unwrap_or(0);
    (major, minor, patch)
}

fn scan_recipes_dir(dir: &Path, catalog: &mut BTreeMap<String, SearchEntry>) {
    if !dir.exists() {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let recipe_file = p.join("recipe.toml");
                if recipe_file.exists()
                    && let Ok(recipe) = beru_recipe::Recipe::from_file(&recipe_file)
                {
                    record_recipe(catalog, &recipe);
                }
            }
        }
    }
}

fn scan_index_dir(dir: &Path, catalog: &mut BTreeMap<String, SearchEntry>) {
    if !dir.exists() {
        return;
    }
    if let Ok(pkg_entries) = fs::read_dir(dir) {
        for pkg_entry in pkg_entries.flatten() {
            let pkg_dir = pkg_entry.path();
            if !pkg_dir.is_dir() {
                continue;
            }
            let pkg_name = pkg_entry.file_name().to_string_lossy().to_string();
            if pkg_name.starts_with('.') {
                continue;
            }

            if let Ok(ver_entries) = fs::read_dir(&pkg_dir) {
                for ver_entry in ver_entries.flatten() {
                    let ver_dir = ver_entry.path();
                    if ver_dir.is_dir() {
                        let recipe_file = ver_dir.join("recipe.toml");
                        if recipe_file.exists()
                            && let Ok(recipe) = beru_recipe::Recipe::from_file(&recipe_file)
                        {
                            record_recipe(catalog, &recipe);
                        }
                    }
                }
            }
        }
    }
}
