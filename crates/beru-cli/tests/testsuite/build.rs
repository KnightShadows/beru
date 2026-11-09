use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_build_success() {
    let p = project("build-proj").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("built successfully"));

    // Verify compile_commands.json was exported to project root for clangd / LSP
    assert!(p.root().join("compile_commands.json").exists());

    // Verify beru clean cleans up compile_commands.json
    p.beru("clean").assert().success();
    assert!(!p.root().join("compile_commands.json").exists());
}
