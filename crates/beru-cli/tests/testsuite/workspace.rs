use crate::support::ProjectBuilder;

#[test]
fn test_beru_workspace_build_and_clean() {
    let project = ProjectBuilder::new("my-monorepo")
        .file(
            "Beru.toml",
            r#"[package]
name = "my-monorepo"
version = "0.1.0"
cxx-std = "c++17"
type = "library"

[workspace]
members = [
    "libs/math",
    "apps/calc",
]
"#,
        )
        // Member 1: math library
        .file(
            "libs/math/Beru.toml",
            r#"[package]
name = "math"
version = "0.1.0"
cxx-std = "c++17"
type = "library"
"#,
        )
        .file(
            "libs/math/include/math.hpp",
            r#"#pragma once
inline int add(int a, int b) { return a + b; }
"#,
        )
        .file(
            "libs/math/src/math.cpp",
            r#"#include "../include/math.hpp"
"#,
        )
        // Member 2: calc executable
        .file(
            "apps/calc/Beru.toml",
            r#"[package]
name = "calc"
version = "0.1.0"
cxx-std = "c++17"
type = "executable"

[dependencies]
math = { path = "../../libs/math" }
"#,
        )
        .file(
            "apps/calc/src/main.cpp",
            r#"#include <iostream>
#include "math.hpp"

int main() {
    std::cout << "Calc sum: " << add(20, 22) << std::endl;
    return 0;
}
"#,
        )
        .build();

    // 1. Build entire workspace from root
    project.beru("build").assert().success();

    assert!(project.root().join("libs/math/build").exists());
    assert!(project.root().join("apps/calc/build").exists());

    // 2. Clean workspace from root
    project.beru("clean").assert().success();

    assert!(!project.root().join("libs/math/build").exists());
    assert!(!project.root().join("apps/calc/build").exists());

    // 3. Build specific workspace member using --package / -p
    project
        .beru("build")
        .arg("--package")
        .arg("calc")
        .assert()
        .success();

    assert!(project.root().join("apps/calc/build").exists());
}
