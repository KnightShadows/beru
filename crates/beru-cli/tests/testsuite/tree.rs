use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_tree() {
    let p = project("tree-proj")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "tree-proj"
            version = "0.1.0"
            
            [dependencies]
            fmt = "11.0.2"
            "#,
        )
        .build();

    // Must resolve first to generate the lockfile
    p.beru("resolve").assert().success();

    // Now run tree
    p.beru("tree")
        .assert()
        .success()
        .stdout(predicate::str::contains("tree-proj v0.1.0"))
        .stdout(predicate::str::contains("fmt v11.0.2"));
}
