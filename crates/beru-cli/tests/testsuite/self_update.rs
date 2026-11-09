use crate::support::project;
use predicates::prelude::*;

#[test]
fn test_beru_self_update_help_displays_flags() {
    let p = project("self-update-help").build();

    p.beru("self-update")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--version"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--repo"));
}

#[test]
fn test_beru_self_update_offline_fails_fast() {
    let p = project("self-update-offline").build();

    p.beru("self-update")
        .arg("--offline")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot perform self-update in offline mode",
        ));
}

#[test]
fn test_beru_self_update_override_source() {
    let p = project("self-update-override").build();

    // Create a dummy binary file in tempdir
    let temp_dir = tempfile::tempdir().unwrap();
    let binary_name = if cfg!(windows) { "beru.exe" } else { "beru" };
    let dummy_exe = temp_dir.path().join(binary_name);
    std::fs::write(&dummy_exe, b"dummy binary content").unwrap();

    // Package into a .tar.gz archive
    let archive_path = temp_dir.path().join("update.tar.gz");
    let tar_gz_file = std::fs::File::create(&archive_path).unwrap();
    let enc = flate2::write::GzEncoder::new(tar_gz_file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_path_with_name(&dummy_exe, binary_name).unwrap();
    tar.finish().unwrap();
    let enc = tar.into_inner().unwrap();
    enc.finish().unwrap();

    // Copy current beru binary to a dummy target location
    let test_run_dir = temp_dir.path().join("bin");
    std::fs::create_dir_all(&test_run_dir).unwrap();
    let dummy_current = test_run_dir.join(binary_name);
    std::fs::write(&dummy_current, b"old version content").unwrap();

    p.beru("self-update")
        .env("BERU_SELF_UPDATE_URL", archive_path.to_str().unwrap())
        .env("BERU_SELF_UPDATE_TARGET", dummy_current.to_str().unwrap())
        .assert()
        .stdout(predicate::str::contains(
            "successfully self-updated from override source",
        ));

    // Verify dummy_current was replaced with new binary content
    let content = std::fs::read(&dummy_current).unwrap();
    assert_eq!(content, b"dummy binary content");
}
