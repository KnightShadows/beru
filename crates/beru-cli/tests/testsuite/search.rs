use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_search_embedded_packages() {
    let p = project("search-test").build();

    p.beru("search")
        .arg("fmt")
        .assert()
        .success()
        .stdout(predicate::str::contains("fmt"))
        .stdout(predicate::str::contains("11.0.2"));
}

#[test]
fn test_beru_search_no_match() {
    let p = project("search-no-match").build();

    p.beru("search")
        .arg("nonexistent_package_xyz123")
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages matching"));
}
