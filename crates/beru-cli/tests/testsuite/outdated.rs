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
fn test_beru_outdated_up_to_date() {
    let p = project("outdated-up-to-date").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("add").arg("fmt@11.0.2").assert().success();

    p.beru("outdated")
        .assert()
        .success()
        .stdout(predicate::str::contains("Package"))
        .stdout(predicate::str::contains("Current"))
        .stdout(predicate::str::contains("Latest"))
        .stdout(predicate::str::contains("fmt"))
        .stdout(predicate::str::contains("11.0.2"))
        .stdout(predicate::str::contains("Up to date"));
}

#[test]
fn test_beru_outdated_shows_newer_version() {
    let p = project("outdated-shows-newer")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "outdated-shows-newer"
            version = "0.1.0"

            [dependencies]
            test-lib = "1.0.0"
            "#,
        )
        .build();

    // Resolve initially at 1.0.0
    seed_index_recipe(&p, "test-lib", "1.0.0");
    p.beru("resolve").assert().success();

    // Add 1.2.0 to index
    seed_index_recipe(&p, "test-lib", "1.2.0");

    p.beru("outdated")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-lib"))
        .stdout(predicate::str::contains("1.0.0"))
        .stdout(predicate::str::contains("1.2.0"))
        .stdout(predicate::str::contains("Update available (minor)"))
        .stdout(predicate::str::contains(
            "Run `beru update` to update dependencies.",
        ));
}

#[test]
fn test_beru_outdated_no_lockfile() {
    let p = project("outdated-no-lock").build();

    p.beru("outdated")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "No Beru.lock found in the current directory.",
        ));
}
