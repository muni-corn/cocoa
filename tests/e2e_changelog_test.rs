//! End-to-end tests for `cocoa changelog`.

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

// ─── Dry-run generates output without writing a file ─────────────────────────

#[test]
fn test_changelog_dry_run_prints_output() {
    let repo = TestRepo::new();
    repo.create_commit("README.md", "# Project", "feat: initial project setup");
    repo.create_commit("src/main.rs", "fn main() {}", "fix: add main function");

    let output_file = repo.path.join("CHANGELOG.md");
    assert!(!output_file.exists(), "changelog should not exist yet");

    cocoa(&repo)
        .args(["--dry-run", "changelog"])
        .assert()
        .success()
        .stdout(predicates::str::contains("# Changelog"));

    assert!(!output_file.exists(), "dry-run must not write the file");
}

// ─── Writes CHANGELOG.md by default ──────────────────────────────────────────

#[test]
fn test_changelog_writes_markdown_file() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: add a");
    repo.create_commit("b.txt", "b", "fix: fix b");

    let output_file = repo.path.join("CHANGELOG.md");

    cocoa(&repo).arg("changelog").assert().success();

    assert!(
        output_file.exists(),
        "CHANGELOG.md should have been created"
    );
    let contents = std::fs::read_to_string(&output_file).unwrap();
    assert!(contents.contains("# Changelog"));
    assert!(contents.contains("## [Unreleased]"));
}

// ─── JSON format
// ──────────────────────────────────────────────────────────────

#[test]
fn test_changelog_json_format() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: add widget");

    // write to a file so we can parse without stripping UI output
    cocoa(&repo)
        .args([
            "changelog",
            "--format",
            "json",
            "--output",
            "changelog.json",
        ])
        .assert()
        .success();

    let contents = std::fs::read_to_string(repo.path.join("changelog.json"))
        .expect("changelog.json should exist");
    let parsed: serde_json::Value =
        serde_json::from_str(&contents).expect("changelog.json should be valid JSON");
    assert!(parsed["versions"].is_array());
}

// ─── HTML format
// ──────────────────────────────────────────────────────────────

#[test]
fn test_changelog_html_format() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: add feature");

    cocoa(&repo)
        .args(["--dry-run", "changelog", "--format", "html"])
        .assert()
        .success()
        .stdout(predicates::str::contains("<!DOCTYPE html>"));
}

// ─── RST format
// ───────────────────────────────────────────────────────────────

#[test]
fn test_changelog_rst_format() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: add thing");

    cocoa(&repo)
        .args(["--dry-run", "changelog", "--format", "rst"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Changelog\n========="));
}

// ─── AsciiDoc format
// ──────────────────────────────────────────────────────────

#[test]
fn test_changelog_asciidoc_format() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: add thing");

    cocoa(&repo)
        .args(["--dry-run", "changelog", "--format", "asciidoc"])
        .assert()
        .success()
        .stdout(predicates::str::contains("= Changelog"));
}

// ─── Unknown format exits with error ─────────────────────────────────────────

#[test]
fn test_changelog_unknown_format_fails() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: add thing");

    cocoa(&repo)
        .args(["changelog", "--format", "pdf"])
        .assert()
        .failure();
}

// ─── Custom output path
// ───────────────────────────────────────────────────────

#[test]
fn test_changelog_custom_output_path() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: first feature");

    let custom_path = repo.path.join("docs/CHANGES.md");
    std::fs::create_dir_all(custom_path.parent().unwrap()).unwrap();

    cocoa(&repo)
        .args(["changelog", "--output", custom_path.to_str().unwrap()])
        .assert()
        .success();

    assert!(custom_path.exists());
    let contents = std::fs::read_to_string(&custom_path).unwrap();
    assert!(contents.contains("# Changelog"));
}

// ─── Versioned history with tags ─────────────────────────────────────────────

#[test]
fn test_changelog_with_tags_shows_versions() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: initial feature");
    repo.create_lightweight_tag("v1.0.0");
    repo.create_commit("b.txt", "b", "feat: new post-release feature");

    cocoa(&repo)
        .args(["--dry-run", "changelog"])
        .assert()
        .success()
        .stdout(predicates::str::contains("v1.0.0"))
        .stdout(predicates::str::contains("Unreleased"));
}

// ─── Range argument
// ───────────────────────────────────────────────────────────

#[test]
fn test_changelog_with_range() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: base");
    repo.create_lightweight_tag("v0.1.0");
    repo.create_commit("b.txt", "b", "feat: after tag");

    cocoa(&repo)
        .args(["--dry-run", "changelog", "v0.1.0..HEAD"])
        .assert()
        .success()
        .stdout(predicates::str::contains("after tag"));
}

// ─── Breaking changes appear prominently ─────────────────────────────────────

#[test]
fn test_changelog_shows_breaking_changes() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat!: breaking api change");

    cocoa(&repo)
        .args(["--dry-run", "changelog"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Breaking Changes"))
        .stdout(predicates::str::contains("breaking api change"));
}

// ─── Template format ─────────────────────────────────────────────────────────

#[test]
fn test_changelog_template_format() {
    let repo = TestRepo::new();
    repo.create_commit("a.txt", "a", "feat: add feature");

    // write a simple template to a stable, absolute path
    let tmpl = repo.path.join("changelog.jinja");
    std::fs::write(
        &tmpl,
        "{% for v in changelog.versions %}VERSIONS:{{ v.version }}{% endfor %}",
    )
    .unwrap();

    let fmt = format!("template:{}", tmpl.to_str().unwrap());

    cocoa(&repo)
        .args(["--dry-run", "changelog", "--format", &fmt])
        .assert()
        .success()
        .stdout(predicates::str::contains("VERSIONS:"));
}
