use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_new_executable() {
    let p = project("new-proj").build();

    p.beru("new")
        .arg("test-proj")
        .arg("--type")
        .arg("executable")
        .assert()
        .success()
        .stdout(predicate::str::contains("Creating executable `test-proj`"));

    let proj_dir = p.root().join("test-proj");
    assert!(proj_dir.join("Beru.toml").exists());
    assert!(proj_dir.join("src").join("main.cpp").exists());
}

#[test]
fn test_beru_new_invalid_package_name_with_suggestion() {
    let p = project("new-invalid").build();

    p.beru("new")
        .arg("my_engine")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid package name 'my_engine'"))
        .stderr(predicate::str::contains("did you mean 'my-engine'?"));

    assert!(!p.root().join("my_engine").exists());
}
