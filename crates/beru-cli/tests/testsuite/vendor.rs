use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_vendor_help_displays_flags() {
    let p = project("vendor-help").build();

    p.beru("vendor")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-delete"))
        .stdout(predicate::str::contains(
            "Directory where dependencies will be vendored",
        ));
}

#[test]
fn test_beru_vendor_no_lockfile_fails() {
    let p = project("vendor-no-lock")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "vendor-no-lock"
            version = "0.1.0"
            "#,
        )
        .build();

    p.beru("vendor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Lockfile Beru.lock not found"));
}

#[test]
fn test_beru_vendor_zero_deps_notice() {
    let p = project("vendor-zero-deps")
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "vendor-zero-deps"
            version = "0.1.0"
            "#,
        )
        .build();

    p.beru("resolve").assert().success();

    p.beru("vendor")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No dependencies found in Beru.lock to vendor",
        ));
}

#[test]
fn test_beru_vendor_path_dependency_and_build_offline() {
    let p = project("vendor-app")
        .file(
            "../helper-lib/Beru.toml",
            r#"
            [package]
            name = "helper-lib"
            version = "1.0.0"
            type = "library"
            "#,
        )
        .file(
            "../helper-lib/CMakeLists.txt",
            r#"
            cmake_minimum_required(VERSION 3.20)
            project(helper-lib LANGUAGES CXX)
            add_library(helper-lib src/helper.cpp)
            target_include_directories(helper-lib PUBLIC include)
            install(TARGETS helper-lib ARCHIVE DESTINATION lib LIBRARY DESTINATION lib)
            install(DIRECTORY include/ DESTINATION include)
            "#,
        )
        .file(
            "../helper-lib/include/helper.hpp",
            r#"
            #pragma once
            int get_number();
            "#,
        )
        .file(
            "../helper-lib/src/helper.cpp",
            r#"
            #include "helper.hpp"
            int get_number() { return 42; }
            "#,
        )
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "vendor-app"
            version = "0.1.0"

            [dependencies]
            helper-lib = { path = "../helper-lib" }
            "#,
        )
        .file(
            "CMakeLists.txt",
            r#"
            cmake_minimum_required(VERSION 3.20)
            project(vendor-app LANGUAGES CXX)
            add_executable(vendor-app src/main.cpp)
            if(COMMAND beru_link_dependencies)
                beru_link_dependencies(vendor-app)
            endif()
            "#,
        )
        .file(
            "src/main.cpp",
            r#"
            #include "helper.hpp"
            int main() {
                return get_number() == 42 ? 0 : 1;
            }
            "#,
        )
        .build();

    // Resolve dependencies to create Beru.lock
    p.beru("resolve").assert().success();

    // Vendor dependencies into ./vendor
    p.beru("vendor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Vendored helper-lib (1.0.0)"))
        .stdout(predicate::str::contains("Finished"));

    // Check that vendor directory exists and has source files
    let vendored_cpp = p
        .root()
        .join("vendor")
        .join("helper-lib-1.0.0")
        .join("src")
        .join("helper.cpp");
    assert!(
        vendored_cpp.exists(),
        "vendored cpp file should exist at {}",
        vendored_cpp.display()
    );

    // Build project offline using vendored dependencies
    p.beru("build")
        .arg("--offline")
        .assert()
        .success()
        .stdout(predicate::str::contains("helper-lib (1.0.0) — vendored"))
        .stdout(predicate::str::contains("built successfully"));
}
