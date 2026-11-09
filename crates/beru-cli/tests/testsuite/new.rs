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
    assert!(proj_dir.join("CMakePresets.json").exists());

    let presets_content = std::fs::read_to_string(proj_dir.join("CMakePresets.json")).unwrap();
    assert!(presets_content.contains("CMAKE_TOOLCHAIN_FILE"));
    assert!(presets_content.contains("CMAKE_EXPORT_COMPILE_COMMANDS"));
}

#[test]
fn test_scaffolded_cmakelists_standalone() {
    let p = project("standalone-cmake").build();

    p.beru("new")
        .arg("standalone-app")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    let proj_dir = p.root().join("standalone-app");
    let cmake_file = std::fs::read_to_string(proj_dir.join("CMakeLists.txt")).unwrap();
    assert!(cmake_file.contains("if(COMMAND beru_link_dependencies)"));
    assert!(cmake_file.contains("CMAKE_EXPORT_COMPILE_COMMANDS ON"));

    // Verify plain CMake configuration works standalone without unknown command error
    let status = std::process::Command::new("cmake")
        .args(["-S", ".", "-B", "build_test"])
        .current_dir(&proj_dir)
        .status();

    if let Ok(s) = status {
        assert!(s.success(), "Plain cmake configuration failed");
    }
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
