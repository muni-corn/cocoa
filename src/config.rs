use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub commit: CommitConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitConfig {
    pub types: Vec<String>,
    pub scopes: Option<Vec<String>>,
    pub rules: CommitRules,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitRules {
    pub enabled: bool,
    pub max_subject_length: usize,
    pub max_body_length: usize,
    pub require_scope: bool,
    pub require_body: bool,
    pub require_type: bool,
    pub require_breaking_change_footer: bool,
    pub ignore_fixup_commits: bool,
    pub ignore_amend_commits: bool,
    pub ignore_squash_commits: bool,
    pub ignore_merge_commits: bool,
    pub ignore_revert_commits: bool,
    pub regex_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            commit: CommitConfig {
                types: vec![
                    "build".to_string(),
                    "chore".to_string(),
                    "ci".to_string(),
                    "docs".to_string(),
                    "feat".to_string(),
                    "fix".to_string(),
                    "perf".to_string(),
                    "refactor".to_string(),
                    "revert".to_string(),
                    "style".to_string(),
                    "test".to_string(),
                ],
                scopes: None,
                rules: CommitRules {
                    enabled: true,
                    max_subject_length: 72,
                    max_body_length: 500,
                    require_scope: false,
                    require_body: false,
                    require_type: true,
                    require_breaking_change_footer: true,
                    ignore_fixup_commits: true,
                    ignore_amend_commits: true,
                    ignore_squash_commits: true,
                    ignore_merge_commits: true,
                    ignore_revert_commits: true,
                    regex_patterns: vec![],
                },
            },
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        Self::load(path).unwrap_or_default()
    }

    pub fn get_allowed_types(&self) -> HashSet<String> {
        self.commit.types.iter().cloned().collect()
    }

    pub fn get_allowed_scopes(&self) -> Option<HashSet<String>> {
        self.commit
            .scopes
            .as_ref()
            .map(|scopes| scopes.iter().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.max_subject_length, 72);
        assert!(config.commit.types.contains(&"feat".to_string()));
        assert!(config.commit.types.contains(&"fix".to_string()));
    }

    #[test]
    fn test_load_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[commit]
types = ["feat", "fix", "test"]
scopes = ["api", "ui"]

[commit.rules]
enabled = true
max_subject_length = 50
max_body_length = 200
require_scope = true
require_body = false
require_type = true
require_breaking_change_footer = false
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true
regex_patterns = []
"#
        )
        .unwrap();

        let config = Config::load(file.path()).unwrap();
        assert_eq!(config.commit.types.len(), 3);
        assert!(config.commit.types.contains(&"feat".to_string()));
        assert_eq!(config.commit.rules.max_subject_length, 50);
        assert!(config.commit.rules.require_scope);

        let scopes = config.get_allowed_scopes().unwrap();
        assert!(scopes.contains("api"));
        assert!(scopes.contains("ui"));
    }

    #[test]
    fn test_load_or_default_fallback() {
        let config = Config::load_or_default("nonexistent.toml");
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.max_subject_length, 72);
    }
}
