//! End-to-end tests for `cocoa migrate`.
//!
//! Tests run the binary via `assert_cmd` inside temporary directories so
//! file detection and writing go through the real filesystem.

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// --- helpers ---

/// Returns a `cocoa` command running inside `dir`.
fn cocoa_in(dir: &TempDir) -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.current_dir(dir.path());
    cmd
}

// --- cocoa migrate: commitlint ---

#[test]
fn test_migrate_commitlint_json_writes_config() {
    let dir = TempDir::new().unwrap();
    let commitlintrc = dir.path().join(".commitlintrc.json");
    fs::write(
        &commitlintrc,
        r#"{"rules":{"type-enum":[2,"always",["feat","fix","chore"]],"header-max-length":[2,"always",72]}}"#,
    )
    .unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "commitlint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("migrated commitlint"));

    let cocoa_toml = dir.path().join(".cocoa.toml");
    assert!(cocoa_toml.exists(), ".cocoa.toml should have been written");

    let contents = fs::read_to_string(&cocoa_toml).unwrap();
    assert!(
        contents.contains("feat"),
        "config should include type 'feat'"
    );
}

#[test]
fn test_migrate_commitlint_yaml_writes_config() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join(".commitlintrc.yaml");
    fs::write(
        &src,
        "rules:\n  type-enum:\n    - 2\n    - always\n    - [feat, fix]\n  header-max-length:\n    - 2\n    - always\n    - 100\n",
    )
    .unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "commitlint"])
        .assert()
        .success();

    let cocoa_toml = dir.path().join(".cocoa.toml");
    let contents = fs::read_to_string(&cocoa_toml).unwrap();
    assert!(contents.contains("feat"));
}

#[test]
fn test_migrate_commitlint_js_fails_with_helpful_error() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("commitlint.config.js");
    fs::write(&src, "module.exports = { rules: {} };").unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "commitlint"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("JavaScript"));
}

// --- cocoa migrate: conventional-changelog ---

#[test]
fn test_migrate_conventional_changelog_js() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("changelog.config.js");
    fs::write(
        &src,
        r#"module.exports = {
  types: [
    { type: 'feat', section: 'Features', hidden: false },
    { type: 'fix', section: 'Bug Fixes', hidden: false },
    { type: 'chore', hidden: true },
  ]
};"#,
    )
    .unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "conventional-changelog"])
        .assert()
        .success();

    let cocoa_toml = dir.path().join(".cocoa.toml");
    assert!(cocoa_toml.exists());
    let contents = fs::read_to_string(&cocoa_toml).unwrap();
    assert!(contents.contains("feat"));
    // sections should be written to changelog config
    assert!(contents.contains("Features") || contents.contains("sections"));
}

#[test]
fn test_migrate_conventional_changelog_json() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("changelog.config.json");
    fs::write(
        &src,
        r#"{"types":[{"type":"feat","section":"Features","hidden":false},{"type":"fix","section":"Bug Fixes","hidden":false}]}"#,
    )
    .unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "conventional-changelog"])
        .assert()
        .success();

    let cocoa_toml = dir.path().join(".cocoa.toml");
    let contents = fs::read_to_string(&cocoa_toml).unwrap();
    assert!(contents.contains("feat"));
}

// --- cocoa migrate: semantic-release ---

#[test]
fn test_migrate_semantic_release_json() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join(".releaserc.json");
    fs::write(&src, r#"{"tagFormat":"v${version}","branches":["main"]}"#).unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "semantic-release"])
        .assert()
        .success();

    let cocoa_toml = dir.path().join(".cocoa.toml");
    assert!(cocoa_toml.exists());
    let contents = fs::read_to_string(&cocoa_toml).unwrap();
    // tag_prefix = "v"
    assert!(
        contents.contains("tag_prefix") || contents.contains("\"v\""),
        "config should contain tag_prefix"
    );
}

#[test]
fn test_migrate_semantic_release_yaml() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join(".releaserc.yaml");
    fs::write(&src, "tagFormat: ${version}\nbranches:\n  - main\n").unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "semantic-release"])
        .assert()
        .success();

    let cocoa_toml = dir.path().join(".cocoa.toml");
    assert!(cocoa_toml.exists());
}

// --- auto-detection ---

#[test]
fn test_migrate_auto_detects_commitlint() {
    let dir = TempDir::new().unwrap();
    // only commitlint config present
    let src = dir.path().join(".commitlintrc.json");
    fs::write(
        &src,
        r#"{"rules":{"type-enum":[2,"always",["feat","fix"]]}}"#,
    )
    .unwrap();

    cocoa_in(&dir)
        .arg("migrate")
        .assert()
        .success()
        .stdout(predicate::str::contains("commitlint"));
}

#[test]
fn test_migrate_auto_detect_no_source_fails() {
    let dir = TempDir::new().unwrap();

    cocoa_in(&dir)
        .arg("migrate")
        .assert()
        .failure()
        .stdout(predicate::str::contains("no supported configuration"));
}

// --- dry run ---

#[test]
fn test_migrate_dry_run_does_not_write_file() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join(".commitlintrc.json");
    fs::write(
        &src,
        r#"{"rules":{"type-enum":[2,"always",["feat","fix"]]}}"#,
    )
    .unwrap();

    cocoa_in(&dir)
        .args(["--dry-run", "migrate", "--from", "commitlint"])
        .assert()
        .success();

    // dry-run should NOT write .cocoa.toml
    assert!(
        !dir.path().join(".cocoa.toml").exists(),
        "dry-run should not write .cocoa.toml"
    );
}

#[test]
fn test_migrate_dry_run_prints_toml() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join(".commitlintrc.json");
    fs::write(
        &src,
        r#"{"rules":{"type-enum":[2,"always",["feat","fix"]]}}"#,
    )
    .unwrap();

    cocoa_in(&dir)
        .args(["--dry-run", "migrate", "--from", "commitlint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[commit]"));
}

// --- backup and rollback ---

#[test]
fn test_migrate_backs_up_existing_cocoa_toml() {
    let dir = TempDir::new().unwrap();
    // write an existing .cocoa.toml
    let existing = dir.path().join(".cocoa.toml");
    fs::write(&existing, "[commit]\n# existing config\n").unwrap();

    let src = dir.path().join(".commitlintrc.json");
    fs::write(
        &src,
        r#"{"rules":{"type-enum":[2,"always",["feat","fix"]]}}"#,
    )
    .unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--from", "commitlint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backed up"));

    // .cocoa.toml.bak should exist with the old content
    let backup = dir.path().join(".cocoa.toml.bak");
    assert!(backup.exists(), ".cocoa.toml.bak should have been created");
    let bak_contents = fs::read_to_string(&backup).unwrap();
    assert!(bak_contents.contains("existing config"));
}

#[test]
fn test_migrate_undo_restores_backup() {
    let dir = TempDir::new().unwrap();
    // create a backup as if a migration had already run
    let backup = dir.path().join(".cocoa.toml.bak");
    fs::write(&backup, "[commit]\n# original config\n").unwrap();

    // also write a "migrated" .cocoa.toml
    let cocoa_toml = dir.path().join(".cocoa.toml");
    fs::write(&cocoa_toml, "[commit]\n# migrated config\n").unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--undo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("restored"));

    // .cocoa.toml should now have the original content
    let contents = fs::read_to_string(&cocoa_toml).unwrap();
    assert!(
        contents.contains("original config"),
        ".cocoa.toml should be restored from backup"
    );

    // .cocoa.toml.bak should no longer exist
    assert!(
        !backup.exists(),
        ".cocoa.toml.bak should have been removed after rollback"
    );
}

#[test]
fn test_migrate_undo_no_backup_fails() {
    let dir = TempDir::new().unwrap();

    cocoa_in(&dir)
        .args(["migrate", "--undo"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("no backup"));
}
