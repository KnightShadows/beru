use crate::support::ProjectBuilder;

#[test]
fn test_beru_build_with_profile_and_system_dependencies() {
    let project = ProjectBuilder::new("sys-app")
        .file(
            "Beru.toml",
            r#"[package]
name = "sys-app"
version = "0.1.0"
cxx-std = "c++17"
type = "executable"

[system-dependencies]
threads = "Threads"

[profile.debug]
opt-level = 0
warnings = "strict"
flags = ["-DAPP_FEATURE_FLAG=1"]
"#,
        )
        .file(
            "src/main.cpp",
            r#"#include <iostream>
#include <thread>

#ifndef APP_FEATURE_FLAG
#error "APP_FEATURE_FLAG must be defined!"
#endif

void worker() {
    std::cout << "Thread executed" << std::endl;
}

int main() {
    std::thread t(worker);
    t.join();
    return 0;
}
"#,
        )
        .build();

    project
        .beru("build")
        .arg("--profile")
        .arg("debug")
        .assert()
        .success();

    // Verify toolchain file contains profile configurations and system dependencies
    let toolchain_path = project.root().join("beru-toolchain.cmake");
    let toolchain_content = std::fs::read_to_string(&toolchain_path).unwrap();
    assert!(
        toolchain_content.contains("-Wall -Wextra -Wpedantic") || toolchain_content.contains("/W4")
    );
    assert!(toolchain_content.contains("APP_FEATURE_FLAG=1"));
    assert!(toolchain_content.contains("find_package(Threads REQUIRED)"));
    assert!(toolchain_content.contains("Threads::Threads"));

    // Verify execution runs and outputs thread message
    project
        .beru("run")
        .arg("--profile")
        .arg("debug")
        .assert()
        .success()
        .stdout(predicates::str::contains("Thread executed"));
}
