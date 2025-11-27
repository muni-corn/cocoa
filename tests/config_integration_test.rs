//! integration tests for config loading

use cocoa::Config;
use std::fs;
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
