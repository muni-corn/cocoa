//! End-to-end tests for `cocoa bump`.

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

/// Write a minimal `.cocoa.toml` with a version section pointing at the given
/// files (relative to the repo root).
fn write_version_config(repo: &TestRepo, files: &[&str]) {
    let list = files
        .iter()
        .map(|f| format!("\"{}\"", f))
        .collect::<Vec<_>>()
        .join(", ");

    let toml = format!(
        "[version]\nstrategy = \"semver\"\ntag_prefix = \"v\"\ncommit_version_files = [{}]\n",
        list
    );

    std::fs::write(repo.path.join(".cocoa.toml"), toml).unwrap();
}

// ─── Dry-run with explicit bump types ────────────────────────────────────────

#[test]
fn test_bump_major_dry_run_shows_new_version() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: initial commit");
    repo.create_annotated_tag("v1.2.3", "release");

    cocoa(&repo)
        .args(["--dry-run", "bump", "major"])
        .assert()
        .success()
        .stdout(predicates::str::contains("1.2.3"))
        .stdout(predicates::str::contains("2.0.0"));
}

#[test]
fn test_bump_minor_dry_run_shows_new_version() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: initial");
    repo.create_annotated_tag("v1.2.3", "release");

    cocoa(&repo)
        .args(["--dry-run", "bump", "minor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("1.3.0"));
}

#[test]
fn test_bump_patch_dry_run_shows_new_version() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "v1", "feat: initial");
    repo.create_annotated_tag("v2.0.0", "release");

    cocoa(&repo)
        .args(["--dry-run", "bump", "patch"])
        .assert()
        .success()
        .stdout(predicates::str::contains("2.0.1"));
}

// ─── Auto bump type detection
// ─────────────────────────────────────────────────

#[test]
fn test_bump_auto_detects_minor_from_feat_commit() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("b.txt", "b", "feat: add widget");

    cocoa(&repo)
        .args(["--dry-run", "bump", "auto"])
        .assert()
        .success()
        .stdout(predicates::str::contains("minor"))
        .stdout(predicates::str::contains("1.1.0"));
}

#[test]
fn test_bump_auto_detects_major_from_breaking_commit() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("b.txt", "b", "feat!: breaking change");

    cocoa(&repo)
        .args(["--dry-run", "bump", "auto"])
        .assert()
        .success()
        .stdout(predicates::str::contains("major"))
        .stdout(predicates::str::contains("2.0.0"));
}

#[test]
fn test_bump_auto_detects_patch_from_fix_only() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("b.txt", "b", "fix: correct a typo");

    cocoa(&repo)
        .args(["--dry-run", "bump", "auto"])
        .assert()
        .success()
        .stdout(predicates::str::contains("patch"))
        .stdout(predicates::str::contains("1.0.1"));
}

// ─── No version tag defaults to 0.0.0 ────────────────────────────────────────

#[test]
fn test_bump_no_tags_starts_from_zero() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "content", "feat: initial commit");

    cocoa(&repo)
        .args(["--dry-run", "bump", "minor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0.0.0"))
        .stdout(predicates::str::contains("0.1.0"));
}

// ─── Invalid bump type returns an error ──────────────────────────────────────

#[test]
fn test_bump_invalid_type_exits_with_error() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "content", "feat: initial");

    cocoa(&repo)
        .args(["bump", "supersonic"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("unknown bump type"));
}

// ─── Dry-run does not modify any files ───────────────────────────────────────

#[test]
fn test_bump_dry_run_does_not_modify_files() {
    let repo = TestRepo::new();
    repo.create_commit("Cargo.toml", "version = \"1.0.0\"\n", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("x.txt", "x", "feat: add thing");

    write_version_config(&repo, &["Cargo.toml"]);

    cocoa(&repo)
        .args(["--dry-run", "bump", "minor"])
        .assert()
        .success();

    let contents = std::fs::read_to_string(repo.path.join("Cargo.toml")).unwrap();
    assert!(
        contents.contains("1.0.0"),
        "dry-run must not modify Cargo.toml"
    );
    assert!(
        !contents.contains("1.1.0"),
        "dry-run must not write new version"
    );
}

// ─── File updates on real bump
// ────────────────────────────────────────────────

#[test]
fn test_bump_updates_configured_file() {
    let repo = TestRepo::new();
    repo.create_commit("Cargo.toml", "version = \"0.1.0\"\n", "feat: initial");
    repo.create_annotated_tag("v0.1.0", "first release");
    repo.create_commit("new.txt", "new", "feat: add thing");

    write_version_config(&repo, &["Cargo.toml"]);

    cocoa(&repo)
        .args(["bump", "minor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0.2.0"));

    let contents = std::fs::read_to_string(repo.path.join("Cargo.toml")).unwrap();
    assert!(
        contents.contains("0.2.0"),
        "Cargo.toml should contain new version"
    );
    assert!(
        !contents.contains("0.1.0"),
        "old version should be replaced"
    );
}

#[test]
fn test_bump_updates_multiple_files() {
    let repo = TestRepo::new();
    repo.create_commit("Cargo.toml", "version = \"1.0.0\"\n", "feat: initial");
    repo.create_commit(
        "package.json",
        "{\"version\":\"1.0.0\"}\n",
        "chore: add package.json",
    );
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("x.txt", "x", "fix: a bug");

    write_version_config(&repo, &["Cargo.toml", "package.json"]);

    cocoa(&repo)
        .args(["bump", "patch"])
        .assert()
        .success()
        .stdout(predicates::str::contains("1.0.1"));

    let cargo = std::fs::read_to_string(repo.path.join("Cargo.toml")).unwrap();
    let pkg = std::fs::read_to_string(repo.path.join("package.json")).unwrap();
    assert!(cargo.contains("1.0.1"));
    assert!(pkg.contains("1.0.1"));
}

// ─── No configured files emits a warning but succeeds ────────────────────────

#[test]
fn test_bump_no_configured_files_warns_and_succeeds() {
    let repo = TestRepo::new();
    repo.create_commit("file.txt", "content", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("new.txt", "new", "feat: add something");

    // no .cocoa.toml → no commit_version_files configured
    cocoa(&repo)
        .args(["bump", "minor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("1.1.0"));
}

// ─── Dry-run lists files that would be updated ───────────────────────────────

#[test]
fn test_bump_dry_run_lists_target_files() {
    let repo = TestRepo::new();
    repo.create_commit("Cargo.toml", "version = \"2.0.0\"\n", "feat: initial");
    repo.create_annotated_tag("v2.0.0", "release");
    repo.create_commit("x.txt", "x", "feat: new thing");

    write_version_config(&repo, &["Cargo.toml"]);

    cocoa(&repo)
        .args(["--dry-run", "bump", "minor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Cargo.toml"));
}

// ─── JSON output ─────────────────────────────────────────────────────────────

#[test]
fn test_bump_json_dry_run_output() {
    let repo = TestRepo::new();
    repo.create_commit("Cargo.toml", "version = \"1.0.0\"\n", "feat: initial");
    repo.create_annotated_tag("v1.0.0", "release");
    repo.create_commit("x.txt", "x", "feat: new feature");

    write_version_config(&repo, &["Cargo.toml"]);

    let output = cocoa(&repo)
        .args(["--json", "--dry-run", "bump", "minor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("output should be valid JSON");

    assert_eq!(json["old_version"], "1.0.0");
    assert_eq!(json["new_version"], "1.1.0");
    assert_eq!(json["bump_type"], "minor");
    assert_eq!(json["dry_run"], true);
}
