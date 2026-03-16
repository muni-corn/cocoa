//! End-to-end tests for `lint` command

mod helpers;

use assert_cmd::cargo::cargo_bin_cmd;
use helpers::git_repo::TestRepo;
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

// --- file path linting ---

#[test]
fn test_lint_from_file_path_valid() {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "feat: add new feature").unwrap();

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("lint")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn test_lint_from_file_path_invalid() {
    use std::io::Write;

    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "bad commit message").unwrap();

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("lint").arg(tmp.path()).assert().failure().code(3);
}

#[test]
fn test_lint_from_commit_editmsg_style_file() {
    use std::io::Write;

    // simulate what git writes to .git/COMMIT_EDITMSG
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "fix(parser): correct off-by-one error in tokenizer").unwrap();
    writeln!(tmp).unwrap();
    writeln!(
        tmp,
        "# Please enter the commit message for your changes. Lines starting"
    )
    .unwrap();
    writeln!(
        tmp,
        "# with '#' will be ignored, and an empty message aborts the commit."
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("lint")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

// --- dry-run mode ---

#[test]
fn test_lint_dry_run_invalid_still_exits_zero() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("--dry-run")
        .arg("lint")
        .arg("--stdin")
        .write_stdin("bad commit message\n")
        .assert()
        .success();
}

#[test]
fn test_lint_dry_run_valid_still_exits_zero() {
    let mut cmd = cargo_bin_cmd!("cocoa");

    cmd.arg("--dry-run")
        .arg("lint")
        .arg("--stdin")
        .write_stdin("feat: add a new thing\n")
        .assert()
        .success();
}

// --- git range linting ---

/// Create a test repo with an initial setup commit and two test commits.
///
/// Returns the repo. The two test commits are at HEAD~1 and HEAD;
/// the range `HEAD~2..HEAD` includes both of them.
fn make_test_repo_with_range(msg1: &str, msg2: &str) -> TestRepo {
    let repo = TestRepo::new();
    // initial commit so HEAD~2 is always resolvable in the tests below
    repo.create_commit("init.txt", "init", "chore: initial repository setup");
    repo.create_commit("a.txt", "hello", msg1);
    repo.create_commit("b.txt", "world", msg2);
    repo
}

#[test]
fn test_lint_range_all_valid() {
    let repo = make_test_repo_with_range("feat: add feature alpha", "fix: correct a small bug");

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("lint")
        .arg("HEAD~2..HEAD")
        .current_dir(&repo.path)
        .assert()
        .success()
        .stdout(predicate::str::contains("passed"));
}

#[test]
fn test_lint_range_has_invalid_commit() {
    let repo = make_test_repo_with_range("feat: add feature alpha", "oops this is a bad message");

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("lint")
        .arg("HEAD~2..HEAD")
        .current_dir(&repo.path)
        .assert()
        .failure()
        .code(3);
}

#[test]
fn test_lint_range_dry_run_with_invalid() {
    let repo = make_test_repo_with_range("feat: valid commit", "oops this is a bad message");

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("--dry-run")
        .arg("lint")
        .arg("HEAD~2..HEAD")
        .current_dir(&repo.path)
        .assert()
        .success();
}

#[test]
fn test_lint_range_json_output() {
    let repo = make_test_repo_with_range("feat: add feature alpha", "fix: patch the thing");

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("--json")
        .arg("lint")
        .arg("HEAD~2..HEAD")
        .current_dir(&repo.path)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"is_valid\":true"));
}

#[test]
fn test_lint_range_json_output_with_invalid() {
    let repo = make_test_repo_with_range("feat: valid commit here", "not a conventional commit");

    let mut cmd = cargo_bin_cmd!("cocoa");
    cmd.arg("--json")
        .arg("lint")
        .arg("HEAD~2..HEAD")
        .current_dir(&repo.path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"is_valid\":false"));
}
