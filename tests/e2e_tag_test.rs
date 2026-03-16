//! End-to-end tests for `cocoa tag`.

mod helpers;

use assert_cmd::Command;
use helpers::git_repo::TestRepo;

/// Create a `cocoa` command scoped to the test repo directory.
fn cocoa(repo: &TestRepo) -> Command {
    let mut cmd = Command::cargo_bin("cocoa").unwrap();
    cmd.current_dir(&repo.path)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("HOME", repo.home_dir.path())
        .env("XDG_CONFIG_HOME", repo.home_dir.path().join(".config"));
    cmd
}

// ─── Dry-run behaviour
// ────────────────────────────────────────────────────────

#[test]
fn test_tag_dry_run_explicit_version_shows_tag_name() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "hello", "feat: initial commit");

    cocoa(&repo)
        .args(["--dry-run", "tag", "1.0.0"])
        .assert()
        .success()
        .stdout(predicates::str::contains("v1.0.0"));
}

#[test]
fn test_tag_dry_run_prefix_stripped_from_explicit_version() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "hello", "feat: initial commit");

    // user passes "v1.2.0" — the "v" prefix is accepted and stripped
    cocoa(&repo)
        .args(["--dry-run", "tag", "v1.2.0"])
        .assert()
        .success()
        .stdout(predicates::str::contains("v1.2.0"));
}

#[test]
fn test_tag_dry_run_auto_detects_version_from_commits() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: initial commit");
    repo.create_annotated_tag("v1.0.0", "first release");
    repo.create_commit("file.txt", "v2", "feat: add another thing");

    // one new feat commit after v1.0.0 → minor bump → v1.1.0
    cocoa(&repo)
        .args(["--dry-run", "tag"])
        .assert()
        .success()
        .stdout(predicates::str::contains("v1.1.0"));
}

#[test]
fn test_tag_dry_run_breaking_change_bumps_major() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: initial commit");
    repo.create_annotated_tag("v1.0.0", "first release");
    repo.create_commit("file.txt", "v2", "feat!: breaking change");

    cocoa(&repo)
        .args(["--dry-run", "tag"])
        .assert()
        .success()
        .stdout(predicates::str::contains("v2.0.0"));
}

#[test]
fn test_tag_dry_run_no_existing_tags_starts_from_zero() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "fix: patch something");

    // only fix commits, no prior tags → patch from 0.0.0 → 0.0.1
    cocoa(&repo)
        .args(["--dry-run", "tag"])
        .assert()
        .success()
        .stdout(predicates::str::contains("v0.0.1"));
}

// ─── Duplicate tag rejection
// ──────────────────────────────────────────────────

#[test]
fn test_tag_dry_run_duplicate_exits_with_error() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: initial commit");
    repo.create_annotated_tag("v1.0.0", "first release");

    // v1.0.0 already exists; using explicit version should fail
    cocoa(&repo)
        .args(["--dry-run", "tag", "1.0.0"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("already exists"));
}

// ─── Actual tag creation
// ──────────────────────────────────────────────────────

#[test]
fn test_tag_creates_annotated_tag_in_repo() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "hello", "feat: initial commit");

    cocoa(&repo)
        .args(["tag", "1.0.0"])
        .assert()
        .success()
        .stdout(predicates::str::contains("v1.0.0"));

    // verify the tag exists in git
    let tag_list = repo
        .create_git_command(&["tag", "-l"])
        .output()
        .expect("git tag -l failed");
    let tags = String::from_utf8_lossy(&tag_list.stdout);
    assert!(tags.contains("v1.0.0"), "tag v1.0.0 not found in repo");
}

#[test]
fn test_tag_message_contains_changelog_content() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "hello", "feat: add awesome feature");

    cocoa(&repo).args(["tag", "1.0.0"]).assert().success();

    // read the tag annotation message
    let output = repo
        .create_git_command(&["tag", "-l", "--format=%(contents)", "v1.0.0"])
        .output()
        .expect("git tag --format failed");

    let message = String::from_utf8_lossy(&output.stdout);
    // the annotation should mention the feature commit
    assert!(
        message.contains("feat")
            || message.contains("awesome feature")
            || message.contains("Release"),
        "tag message did not contain expected changelog content: {}",
        message
    );
}

#[test]
fn test_tag_duplicate_without_dry_run_fails() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: first");
    repo.create_annotated_tag("v1.0.0", "existing tag");

    cocoa(&repo)
        .args(["tag", "1.0.0"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("already exists"));
}
