use crate::support::project;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_beru_remove_dependency() {
    let p = project("remove-proj").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("add").arg("fmt@11.0.2").assert().success();

    let manifest_before = fs::read_to_string(p.root().join("Beru.toml")).unwrap();
    assert!(manifest_before.contains("fmt = \"11.0.2\""));

    let lock_before = fs::read_to_string(p.root().join("Beru.lock")).unwrap();
    assert!(lock_before.contains("name = \"fmt\""));

    // Remove the dependency using the rm alias
    p.beru("rm")
        .arg("fmt")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Removed dependency fmt from Beru.toml",
        ));

    let manifest_after = fs::read_to_string(p.root().join("Beru.toml")).unwrap();
    assert!(!manifest_after.contains("fmt"));

    let lock_after = fs::read_to_string(p.root().join("Beru.lock")).unwrap();
    assert!(!lock_after.contains("name = \"fmt\""));
}

#[test]
fn test_beru_remove_nonexistent_dependency() {
    let p = project("remove-nonexistent").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("remove")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Package 'nonexistent' is not present in Beru.toml",
        ));
}
