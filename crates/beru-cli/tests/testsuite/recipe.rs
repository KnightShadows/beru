use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_beru_recipe_init_and_lint() {
    let dir = tempdir().unwrap();

    // 1. Scaffold a new recipe
    let mut init_cmd = Command::cargo_bin("beru").unwrap();
    init_cmd
        .current_dir(dir.path())
        .arg("recipe")
        .arg("init")
        .arg("sample-lib")
        .arg("--version")
        .arg("1.2.3")
        .arg("--git")
        .arg("https://github.com/example/sample-lib.git")
        .arg("--tag")
        .arg("v1.2.3")
        .assert()
        .success();

    let recipe_path = dir.path().join("recipe.toml");
    assert!(recipe_path.exists());
    let content = std::fs::read_to_string(&recipe_path).unwrap();
    assert!(content.contains("name = \"sample-lib\""));
    assert!(content.contains("version = \"1.2.3\""));
    assert!(content.contains("git = \"https://github.com/example/sample-lib.git\""));

    // 2. Lint the scaffolded recipe
    let mut lint_cmd = Command::cargo_bin("beru").unwrap();
    lint_cmd
        .current_dir(dir.path())
        .arg("recipe")
        .arg("lint")
        .arg("recipe.toml")
        .assert()
        .success()
        .stdout(predicate::str::contains("Recipe is valid"));
}

#[test]
fn test_beru_recipe_init_invalid_name_suggests() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("beru").unwrap();
    cmd.current_dir(dir.path())
        .arg("recipe")
        .arg("init")
        .arg("bad_name")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Did you mean: 'bad-name'?"));
}

#[test]
fn test_beru_recipe_lint_invalid_sha256() {
    let dir = tempdir().unwrap();
    let recipe_path = dir.path().join("recipe.toml");
    let content = r#"
[package]
name = "bad-hash"
version = "1.0.0"

[source]
url = "https://example.com/bad.tar.gz"
sha256 = "short_hash"

[build]
system = "cmake"

[export]
cmake-package = "bad_hash"
"#;
    std::fs::write(&recipe_path, content).unwrap();

    let mut cmd = Command::cargo_bin("beru").unwrap();
    cmd.current_dir(dir.path())
        .arg("recipe")
        .arg("lint")
        .arg("recipe.toml")
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "must be a 64-character hexadecimal string",
        ));
}
