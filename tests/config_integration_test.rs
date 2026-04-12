//! integration tests for config loading

use std::{fs, path::PathBuf};

use cocoa::config::{ChangelogConfig, Config, VersionConfig, VersionStrategy};
use tempfile::TempDir;

#[test]
fn test_load_config_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");

    // use a minimal but complete config
    let config_content = r#"
[commit]
types = ["feat", "fix", "docs"]

[commit.rules]
enabled = true
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true

[commit.rules.warn]
[commit.rules.deny]
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load(config_path.to_str().unwrap()).unwrap();

    assert!(config.commit.types.contains("feat"));
    assert!(config.commit.types.contains("fix"));
    assert!(config.commit.types.contains("docs"));
    assert_eq!(config.commit.types.len(), 3);
}

#[test]
fn test_load_config_with_custom_rules() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");

    let config_content = r#"
[commit]
types = ["feat", "fix"]

[commit.rules]
enabled = true
ignore_fixup_commits = false
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true

[commit.rules.warn]
[commit.rules.deny]
subject_length = 72
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load(config_path.to_str().unwrap()).unwrap();

    assert!(config.commit.rules.enabled);
    assert!(!config.commit.rules.ignore_fixup_commits);
    assert_eq!(config.commit.rules.deny.subject_length, Some(72));
}

#[test]
fn test_load_config_with_scopes() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");

    let config_content = r#"
[commit]
types = ["feat"]
scopes = ["api", "ui", "db"]

[commit.rules]
enabled = true
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true

[commit.rules.warn]
[commit.rules.deny]
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load(config_path.to_str().unwrap()).unwrap();

    let scopes = config.commit.scopes.unwrap();
    assert!(scopes.contains("api"));
    assert!(scopes.contains("ui"));
    assert!(scopes.contains("db"));
    assert_eq!(scopes.len(), 3);
}

#[test]
fn test_load_or_default_with_missing_file() {
    let config = Config::load_or_default("nonexistent.toml");

    // should return default config
    assert!(config.commit.types.contains("feat"));
    assert!(config.commit.types.contains("fix"));
}

#[test]
fn test_load_config_with_ai_section() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");

    let config_content = r#"
[commit]
types = ["feat", "fix"]

[commit.rules]
enabled = true
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true

[commit.rules.warn]
[commit.rules.deny]

[ai]
provider = "OpenAi"
model = "gpt-4"
temperature = 0.7
max_tokens = 500

[ai.secret]
env = "OPENAI_API_KEY"
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load(config_path.to_str().unwrap()).unwrap();

    assert!(config.ai.is_some());
    let ai_config = config.ai.unwrap();
    assert_eq!(ai_config.model, "gpt-4");
    assert_eq!(ai_config.temperature, 0.7);
    assert_eq!(ai_config.max_tokens, 500);
}

#[test]
fn test_load_config_invalid_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.toml");

    let invalid_content = "not valid toml {{{}";
    fs::write(&config_path, invalid_content).unwrap();

    let result = Config::load(config_path.to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_default_config_has_standard_types() {
    let config = Config::default();

    let expected_types = vec![
        "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style",
        "test",
    ];

    for expected_type in expected_types {
        assert!(
            config.commit.types.contains(expected_type),
            "default config should contain type: {}",
            expected_type
        );
    }
}

#[test]
fn test_config_rules_are_enabled_by_default() {
    let config = Config::default();
    assert!(config.commit.rules.enabled);
}

// --- cascading config tests ---

/// Writes a TOML string to a file inside the given directory.
fn write_config(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_load_merged_empty_paths_returns_default() {
    let config = Config::load_merged(&[]).unwrap();

    assert!(config.commit.types.contains("feat"));
    assert!(config.commit.rules.enabled);
}

#[test]
fn test_load_merged_single_file() {
    let dir = TempDir::new().unwrap();
    let path = write_config(
        &dir,
        "repo.toml",
        r#"
[commit]
types = ["feat", "fix", "docs"]
"#,
    );

    let config = Config::load_merged(&[path]).unwrap();

    assert_eq!(config.commit.types.len(), 3);
    assert!(config.commit.types.contains("feat"));
    assert!(config.commit.types.contains("docs"));
    // rules should fall back to defaults
    assert_eq!(config.commit.rules.warn.subject_length, Some(50));
}

#[test]
fn test_load_merged_higher_priority_overrides_lower() {
    let dir = TempDir::new().unwrap();

    // "system" config: custom warn subject_length
    let system = write_config(
        &dir,
        "system.toml",
        r#"
[commit.rules.warn]
subject_length = 40
"#,
    );

    // "repo" config: overrides warn subject_length
    let repo = write_config(
        &dir,
        "repo.toml",
        r#"
[commit.rules.warn]
subject_length = 60
"#,
    );

    // load system first (lowest priority), then repo (highest)
    let config = Config::load_merged(&[system, repo]).unwrap();

    assert_eq!(config.commit.rules.warn.subject_length, Some(60));
}

#[test]
fn test_load_merged_missing_files_are_skipped() {
    let dir = TempDir::new().unwrap();

    let existing = write_config(
        &dir,
        "repo.toml",
        r#"
[commit]
types = ["feat"]
"#,
    );
    let missing = dir.path().join("nonexistent.toml");

    let config = Config::load_merged(&[missing, existing]).unwrap();

    assert_eq!(config.commit.types.len(), 1);
    assert!(config.commit.types.contains("feat"));
}

#[test]
fn test_load_merged_tables_are_deep_merged() {
    let dir = TempDir::new().unwrap();

    // user sets ai provider
    let user = write_config(
        &dir,
        "user.toml",
        r#"
[ai]
model = "gpt-4"
temperature = 0.5
max_tokens = 300

[ai.secret]
env = "OPENAI_API_KEY"
"#,
    );

    // repo overrides just the model
    let repo = write_config(
        &dir,
        "repo.toml",
        r#"
[ai]
model = "gpt-4o"
temperature = 0.5
max_tokens = 300

[ai.secret]
env = "OPENAI_API_KEY"
"#,
    );

    let config = Config::load_merged(&[user, repo]).unwrap();

    let ai = config.ai.expect("ai config should be present");
    assert_eq!(ai.model, "gpt-4o");
}

#[test]
fn test_load_merged_arrays_are_replaced_not_merged() {
    let dir = TempDir::new().unwrap();

    let system = write_config(
        &dir,
        "system.toml",
        r#"
[commit]
types = ["feat", "fix", "docs", "chore"]
"#,
    );

    // repo uses a reduced, project-specific type list
    let repo = write_config(
        &dir,
        "repo.toml",
        r#"
[commit]
types = ["feat", "fix"]
"#,
    );

    let config = Config::load_merged(&[system, repo]).unwrap();

    // repo's array should win entirely - no merging of arrays
    assert_eq!(config.commit.types.len(), 2);
    assert!(config.commit.types.contains("feat"));
    assert!(config.commit.types.contains("fix"));
    assert!(!config.commit.types.contains("docs"));
}

#[test]
fn test_load_merged_changelog_config() {
    let dir = TempDir::new().unwrap();

    let path = write_config(
        &dir,
        "repo.toml",
        r#"
[changelog]
output_file = "CHANGES.md"
include_merge_commits = true
date_format = "%d/%m/%Y"
"#,
    );

    let config = Config::load_merged(&[path]).unwrap();

    let cl = config
        .changelog
        .expect("changelog config should be present");
    assert_eq!(cl.output_file, "CHANGES.md");
    assert!(cl.include_merge_commits);
    assert_eq!(cl.date_format, "%d/%m/%Y");
}

#[test]
fn test_load_merged_version_config() {
    let dir = TempDir::new().unwrap();

    let path = write_config(
        &dir,
        "repo.toml",
        r#"
[version]
strategy = "calver"
tag_prefix = ""
sign_tags = true
commit_version_files = ["Cargo.toml", "package.json"]
"#,
    );

    let config = Config::load_merged(&[path]).unwrap();

    let v = config.version.expect("version config should be present");
    assert_eq!(v.strategy, VersionStrategy::Calver);
    assert_eq!(v.tag_prefix, "");
    assert!(v.sign_tags);
    assert_eq!(
        v.commit_version_files,
        Some(vec!["Cargo.toml".to_string(), "package.json".to_string()])
    );
}

#[test]
fn test_changelog_config_defaults() {
    let cl = ChangelogConfig::default();

    assert_eq!(cl.output_file, "CHANGELOG.md");
    assert!(!cl.include_merge_commits);
    assert!(cl.include_reverts);
    assert_eq!(cl.date_format, "%Y-%m-%d");
    assert_eq!(cl.sections.len(), 7);
}

#[test]
fn test_version_config_defaults() {
    let v = VersionConfig::default();

    assert_eq!(v.strategy, VersionStrategy::Semver);
    assert_eq!(v.tag_prefix, "v");
    assert!(!v.sign_tags);
    assert!(v.commit_version_files.is_none());
}
