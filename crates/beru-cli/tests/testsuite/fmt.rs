use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_fmt_help_displays_flags() {
    let p = project("fmt-help").build();

    p.beru("fmt")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--package"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_beru_fmt_formats_project() {
    let p = project("fmt-run")
        .file(
            "src/main.cpp",
            r#"
            int   main(   ) {
                return    0;
            }
            "#,
        )
        .build();

    // Check formatting - should fail since it's unformatted
    p.beru("fmt").arg("--check").assert().failure();

    // Run format in place
    p.beru("fmt").assert().success();

    // Now check formatting should succeed
    p.beru("fmt").arg("--check").assert().success();
}
