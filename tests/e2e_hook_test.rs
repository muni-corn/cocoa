//! End-to-end tests for `cocoa hook` and `cocoa unhook`.
//!
//! All tests run the binary via `assert_cmd` inside real temporary git
//! repositories so the hook path resolution goes through `git2`.

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

// --- helpers ---

/// Creates a minimal git repo in a temp directory, returning the dir and the
/// path to `.git/hooks`.
fn make_git_repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();
    let path = dir.path();

    // init repo and configure git identity
    for args in [
        vec!["init"],
        vec!["config", "user.name", "Test User"],
        vec!["config", "user.email", "test@example.com"],
    ] {
        std::process::Command::new("git")
            .args(&args)
            .current_dir(path)
            .env("HOME", home_dir.path())
            .output()
            .expect("git command failed");
    }

    // create an initial commit so the repo has a HEAD
    let readme = path.join("README.md");
    fs::write(&readme, "# test\n").unwrap();

    for args in [
        vec!["add", "."],
        vec!["commit", "-m", "chore: initial commit"],
    ] {
        std::process::Command::new("git")
            .args(&args)
            .current_dir(path)
            .env("HOME", home_dir.path())
            .output()
            .expect("git setup failed");
    }

    let hooks_dir = path.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();

    // keep home_dir alive by leaking; TempDir drops on scope exit
    std::mem::forget(home_dir);

    (dir, hooks_dir)
}

/// Returns a `cocoa` command running inside `dir`.
fn cocoa_in(dir: &TempDir) -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.current_dir(dir.path());
    cmd
}

// --- cocoa hook (default = all) ---

#[test]
fn test_hook_default_installs_both_hooks() {
    let (dir, hooks_dir) = make_git_repo();

    // default (no kind arg) installs both hooks
    cocoa_in(&dir).arg("hook").assert().success();

    assert!(
        hooks_dir.join("commit-msg").exists(),
        "commit-msg hook should be installed"
    );
    assert!(
        hooks_dir.join("prepare-commit-msg").exists(),
        "prepare-commit-msg hook should be installed"
    );
}

#[test]
fn test_hook_lint_installs_only_commit_msg() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).args(["hook", "lint"]).assert().success();

    assert!(
        hooks_dir.join("commit-msg").exists(),
        "commit-msg hook should be installed"
    );
    assert!(
        !hooks_dir.join("prepare-commit-msg").exists(),
        "prepare-commit-msg hook should NOT be installed with lint kind"
    );
}

#[test]
fn test_hook_generate_installs_only_prepare_commit_msg() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).args(["hook", "generate"]).assert().success();

    assert!(
        !hooks_dir.join("commit-msg").exists(),
        "commit-msg hook should NOT be installed with generate kind"
    );
    assert!(
        hooks_dir.join("prepare-commit-msg").exists(),
        "prepare-commit-msg hook should be installed"
    );
}

#[test]
fn test_hook_all_installs_both_hooks() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).args(["hook", "all"]).assert().success();

    assert!(hooks_dir.join("commit-msg").exists());
    assert!(hooks_dir.join("prepare-commit-msg").exists());
}

#[test]
fn test_hook_lint_script_calls_cocoa_lint() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).args(["hook", "lint"]).assert().success();

    let contents = fs::read_to_string(hooks_dir.join("commit-msg")).unwrap();
    assert!(
        contents.contains(r#"cocoa lint "$1""#),
        "commit-msg hook should invoke cocoa lint with the filename argument"
    );
    assert!(
        contents.contains("managed by cocoa"),
        "hook should contain the cocoa marker"
    );
}

#[test]
fn test_hook_generate_script_calls_cocoa_generate() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).args(["hook", "generate"]).assert().success();

    let contents = fs::read_to_string(hooks_dir.join("prepare-commit-msg")).unwrap();
    assert!(
        contents.contains(r#"cocoa generate "$1" "$2" "$3""#),
        "prepare-commit-msg hook should invoke cocoa generate with 3 arguments"
    );
    assert!(
        contents.contains("managed by cocoa"),
        "hook should contain the cocoa marker"
    );
}

#[test]
fn test_hook_generate_script_skips_known_sources() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).args(["hook", "generate"]).assert().success();

    let contents = fs::read_to_string(hooks_dir.join("prepare-commit-msg")).unwrap();
    assert!(
        contents.contains("message|merge|squash|commit"),
        "prepare-commit-msg hook should skip amend/merge/squash/-m sources"
    );
}

#[test]
fn test_hook_makes_files_executable() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).arg("hook").assert().success();

    for hook_name in ["commit-msg", "prepare-commit-msg"] {
        let mode = fs::metadata(hooks_dir.join(hook_name))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(
            mode & 0o100,
            0,
            "{hook_name} must have the executable bit set"
        );
    }
}

#[test]
fn test_hook_is_idempotent() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).arg("hook").assert().success();
    let first_lint = fs::read_to_string(hooks_dir.join("commit-msg")).unwrap();
    let first_gen = fs::read_to_string(hooks_dir.join("prepare-commit-msg")).unwrap();

    cocoa_in(&dir).arg("hook").assert().success();
    let second_lint = fs::read_to_string(hooks_dir.join("commit-msg")).unwrap();
    let second_gen = fs::read_to_string(hooks_dir.join("prepare-commit-msg")).unwrap();

    assert_eq!(
        first_lint, second_lint,
        "commit-msg contents should not change on repeat install"
    );
    assert_eq!(
        first_gen, second_gen,
        "prepare-commit-msg contents should not change on repeat install"
    );
}

#[test]
fn test_hook_backs_up_existing_non_cocoa_hook() {
    let (dir, hooks_dir) = make_git_repo();
    let hook_path = hooks_dir.join("commit-msg");
    let backup_path = hooks_dir.join("commit-msg.cocoa-backup");

    let original = "#!/bin/sh\necho 'existing hook'\n";
    fs::write(&hook_path, original).unwrap();

    cocoa_in(&dir).args(["hook", "lint"]).assert().success();

    assert!(backup_path.exists(), "backup file should have been created");
    let backup_contents = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(
        backup_contents, original,
        "backup should contain the original hook"
    );
}

#[test]
fn test_hook_dry_run_does_not_write_files() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir)
        .args(["--dry-run", "hook"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run").or(predicate::str::contains("would")));

    assert!(
        !hooks_dir.join("commit-msg").exists(),
        "dry-run must not create commit-msg"
    );
    assert!(
        !hooks_dir.join("prepare-commit-msg").exists(),
        "dry-run must not create prepare-commit-msg"
    );
}

// --- cocoa unhook ---

#[test]
fn test_unhook_default_removes_both_hooks() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).arg("hook").assert().success();
    assert!(hooks_dir.join("commit-msg").exists());
    assert!(hooks_dir.join("prepare-commit-msg").exists());

    cocoa_in(&dir).arg("unhook").assert().success();

    assert!(
        !hooks_dir.join("commit-msg").exists(),
        "commit-msg should be removed"
    );
    assert!(
        !hooks_dir.join("prepare-commit-msg").exists(),
        "prepare-commit-msg should be removed"
    );
}

#[test]
fn test_unhook_lint_only_removes_commit_msg() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).arg("hook").assert().success();

    cocoa_in(&dir).args(["unhook", "lint"]).assert().success();

    assert!(
        !hooks_dir.join("commit-msg").exists(),
        "commit-msg should be removed"
    );
    assert!(
        hooks_dir.join("prepare-commit-msg").exists(),
        "prepare-commit-msg should remain"
    );
}

#[test]
fn test_unhook_generate_only_removes_prepare_commit_msg() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).arg("hook").assert().success();

    cocoa_in(&dir)
        .args(["unhook", "generate"])
        .assert()
        .success();

    assert!(
        hooks_dir.join("commit-msg").exists(),
        "commit-msg should remain"
    );
    assert!(
        !hooks_dir.join("prepare-commit-msg").exists(),
        "prepare-commit-msg should be removed"
    );
}

#[test]
fn test_unhook_restores_backup() {
    let (dir, hooks_dir) = make_git_repo();
    let hook_path = hooks_dir.join("commit-msg");
    let backup_path = hooks_dir.join("commit-msg.cocoa-backup");

    let original = "#!/bin/sh\necho 'existing hook'\n";
    fs::write(&hook_path, original).unwrap();

    cocoa_in(&dir).args(["hook", "lint"]).assert().success();
    assert!(backup_path.exists(), "backup should exist after install");

    cocoa_in(&dir).args(["unhook", "lint"]).assert().success();

    assert!(!backup_path.exists(), "backup should be cleaned up");
    let restored = fs::read_to_string(&hook_path).unwrap();
    assert_eq!(restored, original, "original hook should be restored");
}

#[test]
fn test_unhook_not_installed_exits_with_warning() {
    let (dir, _hooks_dir) = make_git_repo();

    // no hooks installed; should succeed with a warning message
    cocoa_in(&dir).arg("unhook").assert().success();
}

#[test]
fn test_unhook_refuses_non_cocoa_hook() {
    let (dir, hooks_dir) = make_git_repo();
    let hook_path = hooks_dir.join("commit-msg");

    fs::write(&hook_path, "#!/bin/sh\necho 'not managed by cocoa'\n").unwrap();

    // cocoa writes all output to stdout (not stderr)
    cocoa_in(&dir)
        .args(["unhook", "lint"])
        .assert()
        .failure()
        .stdout(
            predicate::str::contains("not managed by cocoa")
                .or(predicate::str::contains("manually")),
        );
}

#[test]
fn test_unhook_dry_run_does_not_remove_files() {
    let (dir, hooks_dir) = make_git_repo();

    cocoa_in(&dir).arg("hook").assert().success();
    assert!(hooks_dir.join("commit-msg").exists());
    assert!(hooks_dir.join("prepare-commit-msg").exists());

    cocoa_in(&dir)
        .args(["--dry-run", "unhook"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run").or(predicate::str::contains("would")));

    assert!(
        hooks_dir.join("commit-msg").exists(),
        "dry-run must not remove commit-msg"
    );
    assert!(
        hooks_dir.join("prepare-commit-msg").exists(),
        "dry-run must not remove prepare-commit-msg"
    );
}
