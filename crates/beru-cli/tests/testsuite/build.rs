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
    // (Note: CMake only generates compile_commands.json for Makefile and Ninja generators, ignoring it for Visual Studio/MSBuild)
    if p.root()
        .join("build")
        .join("compile_commands.json")
        .exists()
    {
        assert!(p.root().join("compile_commands.json").exists());
        // Verify beru clean cleans up compile_commands.json
        p.beru("clean").assert().success();
        assert!(!p.root().join("compile_commands.json").exists());
    } else {
        p.beru("clean").assert().success();
    }
}

#[test]
fn test_beru_build_header_only_nested_include_and_mismatched_cmake_package() {
    // Mimics Eigen's install layout: package name "myeigen", headers installed to
    // "include/eigen3/Eigen/Dense", and CMake config "Eigen3Config.cmake" under
    // "share/eigen3/cmake/" with target "Eigen3::Eigen" and NO compiled libraries.
    let p = project("eigen-app")
        .file(
            "../myeigen/Beru.toml",
            r#"
            [package]
            name = "myeigen"
            version = "3.4.0"
            type = "header-only"
            "#,
        )
        .file(
            "../myeigen/CMakeLists.txt",
            r#"
            cmake_minimum_required(VERSION 3.20)
            project(Eigen3 VERSION 3.4.0 LANGUAGES NONE)
            include(CMakePackageConfigHelpers)

            add_library(Eigen INTERFACE)
            add_library(Eigen3::Eigen ALIAS Eigen)
            target_include_directories(Eigen INTERFACE
                $<INSTALL_INTERFACE:include/eigen3>
            )

            install(DIRECTORY include/eigen3 DESTINATION include)
            install(TARGETS Eigen EXPORT Eigen3Targets)
            install(EXPORT Eigen3Targets
                FILE Eigen3Targets.cmake
                NAMESPACE Eigen3::
                DESTINATION share/eigen3/cmake
            )
            file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/Eigen3Config.cmake"
                "include(\"\${CMAKE_CURRENT_LIST_DIR}/Eigen3Targets.cmake\")\n"
            )
            install(FILES "${CMAKE_CURRENT_BINARY_DIR}/Eigen3Config.cmake"
                DESTINATION share/eigen3/cmake
            )
            "#,
        )
        .file(
            "../myeigen/include/eigen3/Eigen/Dense",
            r#"
            #pragma once
            namespace Eigen { inline int magic() { return 77; } }
            "#,
        )
        .file(
            "Beru.toml",
            r#"
            [package]
            name = "eigen-app"
            version = "0.1.0"
            cxx-std = "c++17"
            type = "executable"

            [dependencies]
            myeigen = { path = "../myeigen" }
            "#,
        )
        .file(
            "CMakeLists.txt",
            r#"
            cmake_minimum_required(VERSION 3.20)
            project(eigen-app LANGUAGES CXX)
            add_executable(eigen-app src/main.cpp)
            if(COMMAND beru_link_dependencies)
                beru_link_dependencies(eigen-app)
            endif()
            "#,
        )
        .file(
            "src/main.cpp",
            r#"
            #include <iostream>
            #include <Eigen/Dense>
            int main() {
                std::cout << "magic=" << Eigen::magic() << "\n";
                return 0;
            }
            "#,
        )
        .build();

    p.beru("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("magic=77"));

    // Reconfigure with existing build cache: must succeed cleanly without the unused CMAKE_TOOLCHAIN_FILE warning
    p.beru("build")
        .assert()
        .success()
        .stderr(predicate::str::contains("Manually-specified variables were not used").not())
        .stdout(predicate::str::contains("built successfully"));
}
