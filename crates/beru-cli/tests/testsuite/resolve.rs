use crate::support::project;
use std::fs;

#[test]
fn test_beru_resolve_dependencies() {
    let p = project("resolve-proj")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "resolve-proj"
            version = "0.1.0"

            [dependencies]
            fmt = "11.0.2"
            "#,
        )
        .build();

    // First update the index in our isolated sandbox
    p.beru("index").arg("update").assert().success();

    // Now resolve dependencies
    p.beru("resolve").assert().success();

    let lockfile = fs::read_to_string(p.root().join("Beru.lock")).expect("failed to read lockfile");
    assert!(lockfile.contains("fmt"));
    assert!(lockfile.contains("11.0.2"));
}

/// Helper: create a minimal recipe.toml for a header-only test package.
fn test_recipe(name: &str, version: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "{version}"
type = "header-only"

[source]
url = "https://example.com/{name}-{version}.tar.gz"
sha256 = "0000000000000000000000000000000000000000000000000000000000000000"

[export]
include-dirs = ["include"]
"#
    )
}

/// Seed the sandbox's index directory with a recipe for a given package+version.
fn seed_index_recipe(p: &crate::support::Project, name: &str, version: &str) {
    let recipe_dir = p.beru_home().join("index").join(name).join(version);
    fs::create_dir_all(&recipe_dir).expect("failed to create index recipe dir");
    fs::write(recipe_dir.join("recipe.toml"), test_recipe(name, version))
        .expect("failed to write index recipe");
}

#[test]
fn test_beru_resolve_respects_version_pin() {
    let p = project("resolve-pin")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "resolve-pin"
            version = "0.1.0"

            [dependencies]
            test-lib = "1.0.0"
            "#,
        )
        .build();

    // Seed two versions in the sandbox index
    seed_index_recipe(&p, "test-lib", "1.0.0");
    seed_index_recipe(&p, "test-lib", "2.0.0");

    // Resolve — should pick 1.0.0 because bare version = exact pin
    p.beru("resolve").assert().success();

    let lockfile = fs::read_to_string(p.root().join("Beru.lock")).expect("failed to read lockfile");
    assert!(
        lockfile.contains("version = \"1.0.0\""),
        "lockfile should contain version 1.0.0, got:\n{}",
        lockfile
    );
    assert!(
        !lockfile.contains("version = \"2.0.0\""),
        "lockfile should NOT contain version 2.0.0, got:\n{}",
        lockfile
    );
}

#[test]
fn test_beru_resolve_wildcard_gets_latest() {
    let p = project("resolve-wildcard")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "resolve-wildcard"
            version = "0.1.0"

            [dependencies]
            test-lib = "*"
            "#,
        )
        .build();

    // Seed two versions in the sandbox index
    seed_index_recipe(&p, "test-lib", "1.0.0");
    seed_index_recipe(&p, "test-lib", "2.0.0");

    // Resolve — should pick 2.0.0 because "*" = any version, highest wins
    p.beru("resolve").assert().success();

    let lockfile = fs::read_to_string(p.root().join("Beru.lock")).expect("failed to read lockfile");
    assert!(
        lockfile.contains("version = \"2.0.0\""),
        "lockfile should contain version 2.0.0 (latest), got:\n{}",
        lockfile
    );
}
