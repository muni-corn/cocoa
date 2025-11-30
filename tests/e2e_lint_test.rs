//! End-to-end tests for `lint` command

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn test_lint_valid_commit_via_stdin() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("lint")
        .arg("--stdin")
        .write_stdin("feat: add new feature\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_lint_invalid_commit_via_stdin() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("lint")
        .arg("--stdin")
        .write_stdin("bad commit message\n")
        .assert()
        .failure()
        .code(3);
}

#[test]
fn test_lint_with_scope() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("lint")
        .arg("--stdin")
        .write_stdin("feat(api): add new endpoint\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_lint_breaking_change() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("lint")
        .arg("--stdin")
        .write_stdin("feat!: breaking API change\n\nBREAKING CHANGE: removed old endpoint\n")
        .assert()
        .success();
}

#[test]
fn test_lint_json_output_valid() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("--json")
        .arg("lint")
        .arg("--stdin")
        .write_stdin("feat: valid commit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"is_valid\":true"));
}

#[test]
fn test_lint_json_output_invalid() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("--json")
        .arg("lint")
        .arg("--stdin")
        .write_stdin("invalid commit\n")
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"is_valid\":false"));
}

#[test]
fn test_lint_quiet_mode() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("--quiet")
        .arg("lint")
        .arg("--stdin")
        .write_stdin("feat: test\n")
        .assert()
        .success();
    // quiet mode may still show welcome message, just checking it succeeds
}

#[test]
fn test_lint_multiple_types() {
    let types = vec![
        "feat: new feature",
        "fix: bug fix",
        "docs: update docs",
        "style: format code",
        "refactor: restructure",
        "test: add tests",
        "chore: update deps",
    ];

    for commit_msg in types {
        let mut cmd = cargo_bin_cmd!("cocoa");
        cmd.arg("lint")
            .arg("--stdin")
            .write_stdin(format!("{}\n", commit_msg))
            .assert()
            .success();
    }
}

#[test]
fn test_lint_subject_too_long() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    let long_subject = format!("feat: {}", "a".repeat(100));

    cmd.arg("lint")
        .arg("--stdin")
        .write_stdin(format!("{}\n", long_subject))
        .assert()
        .failure() // fails because of rule violations
        .code(3);
}
