use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_build_offline_uncached_fails_cleanly() {
    let p = project("offline-uncached")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "offline-uncached"
            version = "0.1.0"

            [dependencies]
            fmt = "11.0.2"
            "#,
        )
        .file(
            "src/main.cpp",
            r#"
            int main() { return 0; }
            "#,
        )
        .build();

    // Since fmt tarball is not pre-cached in our fresh test BERU_HOME, --offline must fail
    // immediately with our actionable message rather than hanging or attempting DNS/HTTP.
    p.beru("build")
        .arg("--offline")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot download"))
        .stderr(predicate::str::contains(
            "in offline mode: package archive is not cached",
        ));
}

#[test]
fn test_beru_build_offline_zero_deps_succeeds() {
    let p = project("offline-zero-deps").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    p.beru("build")
        .arg("--offline")
        .assert()
        .success()
        .stdout(predicate::str::contains("built successfully"));
}
