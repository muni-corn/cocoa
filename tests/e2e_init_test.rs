//! End-to-end tests for `cocoa init`.
//!
//! All tests run the binary via `assert_cmd`. Because stderr is not a TTY in
//! the test harness, `init` runs in non-interactive mode and uses default
//! configuration values.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// --- helpers ---

/// Creates a command running in a fresh temporary directory.
fn init_cmd_in(dir: &TempDir) -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.current_dir(dir.path());
    cmd
}

// --- dry-run tests ---

#[test]
fn test_init_dry_run_exits_successfully() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp)
        .arg("--dry-run")
        .arg("init")
        .assert()
        .success();
}

#[test]
fn test_init_dry_run_prints_commit_section() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp)
        .arg("--dry-run")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("[commit]"));
}

#[test]
fn test_init_dry_run_prints_commit_rules() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp)
        .arg("--dry-run")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("[commit.rules]"));
}

#[test]
fn test_init_dry_run_does_not_create_file() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp)
        .arg("--dry-run")
        .arg("init")
        .assert()
        .success();

    assert!(
        !tmp.path().join(".cocoa.toml").exists(),
        "--dry-run must not write .cocoa.toml"
    );
}

// --- normal (write) tests ---

#[test]
fn test_init_creates_config_file() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp).arg("init").assert().success();

    assert!(
        tmp.path().join(".cocoa.toml").exists(),
        ".cocoa.toml should be created"
    );
}

#[test]
fn test_init_written_file_contains_commit_section() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp).arg("init").assert().success();

    let content = std::fs::read_to_string(tmp.path().join(".cocoa.toml")).unwrap();
    assert!(
        content.contains("[commit]"),
        "written TOML should contain [commit] section"
    );
}

#[test]
fn test_init_written_config_is_loadable() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp).arg("init").assert().success();

    let config_path = tmp.path().join(".cocoa.toml");
    let config = cocoa::Config::load(&config_path)
        .expect("written config should be parseable by Config::load");

    // default types must all be present in the non-interactive path
    for typ in &["feat", "fix", "chore", "docs", "style", "refactor", "test"] {
        assert!(
            config.commit.types.contains(*typ),
            "default type '{}' should be in written config",
            typ
        );
    }
}

#[test]
fn test_init_written_config_has_valid_rules() {
    let tmp = TempDir::new().unwrap();
    init_cmd_in(&tmp).arg("init").assert().success();

    let config_path = tmp.path().join(".cocoa.toml");
    let config = cocoa::Config::load(&config_path).unwrap();

    // default warn < deny (validation would have rejected otherwise)
    let warn_subj = config.commit.rules.warn.subject_length.unwrap();
    let deny_subj = config.commit.rules.deny.subject_length.unwrap();
    assert!(
        deny_subj > warn_subj,
        "deny.subject_length ({}) must be greater than warn.subject_length ({})",
        deny_subj,
        warn_subj
    );
}

// --- existing-file protection tests ---

#[test]
fn test_init_fails_non_interactively_when_file_exists() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join(".cocoa.toml");

    // pre-create the file
    std::fs::write(&config_path, "[commit]\n").unwrap();

    init_cmd_in(&tmp).arg("init").assert().failure();
}

#[test]
fn test_init_does_not_overwrite_existing_file_non_interactively() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join(".cocoa.toml");
    let original = "[commit]\n# sentinel\n";

    std::fs::write(&config_path, original).unwrap();

    // run init (will fail, but must not overwrite)
    let _ = init_cmd_in(&tmp).arg("init").assert().failure();

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(
        content, original,
        "existing .cocoa.toml must not be modified"
    );
}

#[test]
fn test_init_dry_run_does_not_fail_when_file_exists() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join(".cocoa.toml");
    std::fs::write(&config_path, "[commit]\n").unwrap();

    // dry-run never writes, so the existing-file guard should not trigger
    init_cmd_in(&tmp)
        .arg("--dry-run")
        .arg("init")
        .assert()
        .success();

    // original file must be unchanged
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(content, "[commit]\n");
}
