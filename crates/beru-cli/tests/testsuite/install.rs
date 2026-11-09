use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_install_and_uninstall_executable() {
    let p = project("my-tool").build();

    p.beru("init")
        .arg("--type")
        .arg("executable")
        .assert()
        .success();

    // Install the executable into ~/.beru/bin
    p.beru("install")
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed `my-tool`"));

    let bin_dir = p.beru_home().join("bin");
    let installed_exe = if cfg!(windows) {
        bin_dir.join("my-tool.exe")
    } else {
        bin_dir.join("my-tool")
    };
    assert!(
        installed_exe.exists(),
        "Installed binary should exist in bin_dir"
    );

    // Execute the installed binary directly
    let mut cmd = std::process::Command::new(&installed_exe);
    let output = cmd.output().expect("failed to execute installed binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from Beru!"));

    // Now uninstall it
    p.beru("uninstall")
        .arg("my-tool")
        .assert()
        .success()
        .stdout(predicate::str::contains("Uninstalled binary `my-tool`"));

    assert!(
        !installed_exe.exists(),
        "Binary should no longer exist after uninstall"
    );
}

#[test]
fn test_beru_uninstall_nonexistent() {
    let p = project("uninstall-nonexistent").build();

    p.beru("uninstall")
        .arg("ghost-tool")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Binary 'ghost-tool' is not installed",
        ));
}

#[test]
fn test_beru_install_library_fails() {
    let p = project("lib-proj").build();

    p.beru("init")
        .arg("--type")
        .arg("library")
        .assert()
        .success();

    p.beru("install")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "only supported for executable projects",
        ));
}
