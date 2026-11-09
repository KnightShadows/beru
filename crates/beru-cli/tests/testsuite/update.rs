use crate::support::project;
use predicates::prelude::*;
use std::fs;

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

fn seed_index_recipe(p: &crate::support::Project, name: &str, version: &str) {
    let recipe_dir = p.beru_home().join("index").join(name).join(version);
    fs::create_dir_all(&recipe_dir).expect("failed to create index recipe dir");
    fs::write(recipe_dir.join("recipe.toml"), test_recipe(name, version))
        .expect("failed to write index recipe");
}

#[test]
fn test_beru_update_up_to_date() {
    let p = project("update-up-to-date").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("add").arg("fmt@11.0.2").assert().success();

    p.beru("update")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updating dependencies..."))
        .stdout(predicate::str::contains("All dependencies are up to date."));
}

#[test]
fn test_beru_update_specific_package_up_to_date() {
    let p = project("update-pkg-up-to-date").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("add").arg("fmt@11.0.2").assert().success();

    p.beru("update")
        .arg("fmt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updating fmt"))
        .stdout(predicate::str::contains(
            "fmt is already at the latest compatible version.",
        ));
}

#[test]
fn test_beru_update_nonexistent_package() {
    let p = project("update-nonexistent").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("update")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Package 'nonexistent' is not a dependency in Beru.toml",
        ));
}

#[test]
fn test_beru_update_advances_lockfile_version() {
    let p = project("update-advances")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "update-advances"
            version = "0.1.0"

            [dependencies]
            test-lib = ">=1.0.0"
            "#,
        )
        .build();

    // First, only version 1.0.0 exists
    seed_index_recipe(&p, "test-lib", "1.0.0");
    p.beru("resolve").assert().success();

    let lock_before = fs::read_to_string(p.root().join("Beru.lock")).unwrap();
    assert!(lock_before.contains("version = \"1.0.0\""));

    // Now upstream publishes 1.5.0
    seed_index_recipe(&p, "test-lib", "1.5.0");

    p.beru("update")
        .arg("test-lib")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Updated test-lib v1.0.0 -> v1.5.0",
        ));

    let lock_after = fs::read_to_string(p.root().join("Beru.lock")).unwrap();
    assert!(lock_after.contains("version = \"1.5.0\""));
}
