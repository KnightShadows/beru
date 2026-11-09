use crate::support::project;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_beru_add_dependency() {
    let p = project("add-proj").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("add")
        .arg("fmt@11.0.2")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added dependency fmt v11.0.2"));

    let manifest = fs::read_to_string(p.root().join("Beru.toml")).unwrap();
    assert!(manifest.contains("fmt = \"11.0.2\""));

    let lock = fs::read_to_string(p.root().join("Beru.lock")).unwrap();
    assert!(lock.contains("name = \"fmt\""));
    assert!(lock.contains("version = \"11.0.2\""));
}

#[test]
fn test_beru_add_dependency_no_version() {
    let p = project("add-proj-no-ver").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("add")
        .arg("spdlog")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added dependency spdlog v*"));

    let manifest = fs::read_to_string(p.root().join("Beru.toml")).unwrap();
    assert!(manifest.contains("spdlog = \"*\""));
}
