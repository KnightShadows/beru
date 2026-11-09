use crate::support::project;
use predicates::prelude::*;
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

    // Now resolve dependencies without index update (using embedded fallback)
    p.beru("resolve").assert().success();

    let lockfile = fs::read_to_string(p.root().join("Beru.lock")).expect("failed to read lockfile");
    assert!(lockfile.contains("fmt"));
    assert!(lockfile.contains("11.0.2"));
}

#[test]
fn test_resolve_missing_package_empty_index_error() {
    let p = project("missing-pkg")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "missing-pkg"
            version = "0.1.0"

            [dependencies]
            unknown-package = "1.0.0"
            "#,
        )
        .build();

    p.beru("resolve")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Could not find package 'unknown-package' in index, sources, or bundled recipes",
        ))
        .stderr(predicates::str::contains("run `beru index update`"));
}

#[test]
fn test_resolve_missing_package_with_dotgit_index_error() {
    let p = project("missing-pkg-git-index")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "missing-pkg-git-index"
            version = "0.1.0"

            [dependencies]
            unknown-package = "1.0.0"
            "#,
        )
        .build();

    // Create an empty .git directory in index (e.g. from interrupted git init/clone)
    fs::create_dir_all(p.beru_home().join("index").join(".git")).unwrap();

    p.beru("resolve")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Could not find package 'unknown-package' in index, sources, or bundled recipes",
        ))
        .stderr(predicates::str::contains("run `beru index update`"));
}

#[test]
fn test_fresh_install_resolve_reaches_build() {
    let p = project("fresh-install").build();

    p.beru("new")
        .arg("my-engine")
        .arg("--type")
        .arg("executable")
        .arg("--cxx-std")
        .arg("c++20")
        .assert()
        .success();

    let engine_dir = p.root().join("my-engine");

    // Add fmt 11.0.2 (which is embedded in binary)
    let mut cmd = assert_cmd::Command::cargo_bin("beru").unwrap();
    cmd.current_dir(&engine_dir)
        .env("BERU_HOME", p.beru_home())
        .arg("add")
        .arg("fmt")
        .arg("--version")
        .arg("11.0.2")
        .assert()
        .success();

    // Resolve dependencies (hermetic, completely offline, with empty BERU_HOME)
    let mut cmd = assert_cmd::Command::cargo_bin("beru").unwrap();
    cmd.current_dir(&engine_dir)
        .env("BERU_HOME", p.beru_home())
        .arg("resolve")
        .assert()
        .success();

    let lock = fs::read_to_string(engine_dir.join("Beru.lock")).unwrap();
    assert!(lock.contains("fmt"));
    assert!(lock.contains("11.0.2"));
}

#[test]
fn test_beru_resolve_conflict_explanation() {
    let p = project("conflict-proj")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "conflict-proj"
            version = "0.1.0"

            [dependencies]
            a = "1.0.0"
            b = "1.0.0"
            "#,
        )
        .file(
            "recipes/a/recipe.toml",
            r#"
            [package]
            name = "a"
            version = "1.0.0"
            type = "header-only"

            [source]
            url = "https://example.com/a-1.0.0.tar.gz"
            sha256 = "0000000000000000000000000000000000000000000000000000000000000000"

            [dependencies]
            libx = "1.0.0"
            "#,
        )
        .file(
            "recipes/b/recipe.toml",
            r#"
            [package]
            name = "b"
            version = "1.0.0"
            type = "header-only"

            [source]
            url = "https://example.com/b-1.0.0.tar.gz"
            sha256 = "0000000000000000000000000000000000000000000000000000000000000000"

            [dependencies]
            libx = "2.0.0"
            "#,
        )
        .file(
            "recipes/libx/recipe.toml",
            r#"
            [package]
            name = "libx"
            version = "1.0.0"
            type = "header-only"

            [source]
            url = "https://example.com/libx-1.0.0.tar.gz"
            sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
            "#,
        )
        .build();

    let output = p.beru("resolve").assert().failure();
    output
        .stderr(predicates::str::contains("a"))
        .stderr(predicates::str::contains("b"))
        .stderr(predicates::str::contains("libx"))
        .stdout(predicates::str::contains("unit_propagation").not())
        .stdout(predicates::str::contains("DP chose").not())
        .stdout(predicates::str::contains("prior cause").not())
        .stderr(predicates::str::contains("unit_propagation").not())
        .stderr(predicates::str::contains("DP chose").not())
        .stderr(predicates::str::contains("prior cause").not());
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
