use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_init_library() {
    let p = project("init-proj").build();

    p.beru("init")
        .arg("--type")
        .arg("library")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initializing Beru"));

    assert!(p.root().join("Beru.toml").exists());
    assert!(p.root().join("src").join("init-proj.cpp").exists());
}

#[test]
fn test_beru_init_sanitizes_directory_name() {
    let p = project("my_cool_lib_").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Initializing Beru in existing directory (my-cool-lib)",
        ));

    let manifest = std::fs::read_to_string(p.root().join("Beru.toml")).unwrap();
    assert!(manifest.contains("name = \"my-cool-lib\""));
}
