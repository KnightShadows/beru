use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_setup_help_displays_flags() {
    let p = project("setup-help").build();

    p.beru("setup")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--install-prereqs"))
        .stdout(predicate::str::contains("--add-to-path"));
}

#[test]
fn test_beru_setup_prints_diagnostics_table() {
    let p = project("setup-run").build();

    p.beru("setup")
        .assert()
        .stdout(predicate::str::contains(
            "Beru Environment & Toolchain Setup",
        ))
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("cmake"))
        .stdout(predicate::str::contains("C++ compiler"))
        .stdout(predicate::str::contains("ninja"))
        .stdout(predicate::str::contains("PATH"));
}
