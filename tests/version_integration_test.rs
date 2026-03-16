//! Integration tests for version management.

mod helpers;

use std::fs;

use cocoa::{
    git_ops::{Git2Ops, GitOperations},
    version::{
        BumpType, SemVer, detect_bump_type, detect_current_semver, detect_latest_tag,
        update_version_files,
    },
};
use helpers::git_repo::TestRepo;

// ── helpers
// ───────────────────────────────────────────────────────────────────

/// Open a `Git2Ops` instance pointing at the given `TestRepo`.
fn ops_for(repo: &TestRepo) -> Git2Ops {
    Git2Ops::open_at(&repo.path).expect("failed to open Git2Ops")
}

// ── semver detection
// ──────────────────────────────────────────────────────────

#[test]
fn test_detect_semver_from_real_tags() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: initial commit");
    repo.create_annotated_tag("v1.0.0", "first release");
    repo.create_commit("file.txt", "v2", "feat: another feature");
    repo.create_annotated_tag("v1.1.0", "second release");

    let ops = ops_for(&repo);
    let version = detect_current_semver(&ops, "v").unwrap();
    assert_eq!(version.unwrap().to_string(), "1.1.0");
}

#[test]
fn test_detect_semver_no_tags_returns_none() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "content", "feat: first commit");

    let ops = ops_for(&repo);
    let version = detect_current_semver(&ops, "v").unwrap();
    assert_eq!(version, None);
}

#[test]
fn test_detect_semver_mixed_tags_picks_latest() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "a", "feat: a");
    repo.create_annotated_tag("v0.9.0", "pre-release");
    repo.create_commit("file.txt", "b", "feat: b");
    repo.create_annotated_tag("v2.0.0", "major release");
    repo.create_commit("file.txt", "c", "fix: c");
    repo.create_lightweight_tag("v1.5.0");

    let ops = ops_for(&repo);
    let version = detect_current_semver(&ops, "v").unwrap();
    assert_eq!(version.unwrap().to_string(), "2.0.0");
}

#[test]
fn test_detect_semver_lightweight_tag() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "content", "feat: initial");
    repo.create_lightweight_tag("v3.1.4");

    let ops = ops_for(&repo);
    let version = detect_current_semver(&ops, "v").unwrap();
    assert_eq!(version.unwrap().to_string(), "3.1.4");
}

#[test]
fn test_detect_latest_tag_info() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "a", "feat: a");
    repo.create_annotated_tag("v0.1.0", "initial release");
    repo.create_commit("file.txt", "b", "feat: b");
    repo.create_annotated_tag("v0.2.0", "second release");

    let ops = ops_for(&repo);
    let tag = detect_latest_tag(&ops, "v").unwrap().unwrap();
    assert_eq!(tag.name, "v0.2.0");
}

// ── bump type detection
// ───────────────────────────────────────────────────────

#[test]
fn test_detect_bump_type_from_real_commits() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: initial commit");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("b.txt", "b", "fix: patch a bug");
    repo.create_commit("c.txt", "c", "feat: add new thing");

    let ops = ops_for(&repo);
    let tag = detect_latest_tag(&ops, "v").unwrap().unwrap();

    // get commits since the last tag
    let commits = ops
        .get_commits_in_range(&tag.target, "HEAD")
        .expect("failed to get commits");

    assert_eq!(detect_bump_type(&commits), BumpType::Minor);
}

#[test]
fn test_detect_bump_type_breaking_change_from_commits() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("b.txt", "b", "feat!: breaking change");

    let ops = ops_for(&repo);
    let tag = detect_latest_tag(&ops, "v").unwrap().unwrap();
    let commits = ops
        .get_commits_in_range(&tag.target, "HEAD")
        .expect("failed to get commits");

    assert_eq!(detect_bump_type(&commits), BumpType::Major);
}

#[test]
fn test_detect_bump_type_only_fix_gives_patch() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("b.txt", "b", "fix: correct a typo");
    repo.create_commit("c.txt", "c", "chore: update readme");

    let ops = ops_for(&repo);
    let tag = detect_latest_tag(&ops, "v").unwrap().unwrap();
    let commits = ops
        .get_commits_in_range(&tag.target, "HEAD")
        .expect("failed to get commits");

    assert_eq!(detect_bump_type(&commits), BumpType::Patch);
}

// ── version file updates
// ──────────────────────────────────────────────────────

#[test]
fn test_update_version_files_in_real_project() {
    let dir = tempfile::tempdir().unwrap();

    let cargo_path = dir.path().join("Cargo.toml");
    let package_path = dir.path().join("package.json");

    fs::write(
        &cargo_path,
        "[package]\nname = \"myapp\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        &package_path,
        "{\n  \"name\": \"myapp\",\n  \"version\": \"1.0.0\"\n}\n",
    )
    .unwrap();

    let files = vec![
        cargo_path.to_string_lossy().into_owned(),
        package_path.to_string_lossy().into_owned(),
    ];

    update_version_files(&files, "1.0.0", "1.1.0").unwrap();

    let cargo_contents = fs::read_to_string(&cargo_path).unwrap();
    assert!(cargo_contents.contains("version = \"1.1.0\""));
    assert!(!cargo_contents.contains("1.0.0"));

    let package_contents = fs::read_to_string(&package_path).unwrap();
    assert!(package_contents.contains("\"version\": \"1.1.0\""));
    assert!(!package_contents.contains("1.0.0"));
}

#[test]
fn test_update_version_files_rollback_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let real_path = dir.path().join("file.txt");
    fs::write(&real_path, "version = 1.0.0\n").unwrap();

    let files = vec![
        real_path.to_string_lossy().into_owned(),
        "/nonexistent/path.txt".to_string(), // this will fail
    ];

    // the update should fail because the second file doesn't exist
    let result = update_version_files(&files, "1.0.0", "2.0.0");
    assert!(result.is_err());

    // the first file should be unchanged (or rolled back) — since the second
    // file fails at the read phase, the first file was never written to
    let contents = fs::read_to_string(&real_path).unwrap();
    assert_eq!(contents, "version = 1.0.0\n");
}

// ── semver engine round-trip
// ──────────────────────────────────────────────────

#[test]
fn test_semver_full_bump_workflow() {
    let v = SemVer::parse("1.2.3").unwrap();

    // simulate automatic bump: breaking → major
    let major = v.bump_major();
    assert_eq!(major.to_string(), "2.0.0");

    // simulate adding pre-release
    let pre = major.with_pre_release("rc.1").unwrap();
    assert_eq!(pre.to_string(), "2.0.0-rc.1");

    // release candidate becomes the real release (bump patch clears pre-release)
    let release = pre.bump_patch();
    assert_eq!(release.to_string(), "2.0.1");
}

#[test]
fn test_semver_tag_prefix_round_trip() {
    // simulate creating a tag from a version and reading it back
    let original = SemVer::parse("0.5.0").unwrap();
    let tag_name = format!("v{original}");
    assert_eq!(tag_name, "v0.5.0");

    let stripped = tag_name.strip_prefix("v").unwrap();
    let recovered = SemVer::parse(stripped).unwrap();
    assert_eq!(original, recovered);
}
