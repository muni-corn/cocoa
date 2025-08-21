use std::{collections::HashSet, fs, path::Path};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Read(#[from] std::io::Error),

    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("configuration validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub commit: CommitConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitConfig {
    pub types: HashSet<String>,
    pub scopes: Option<HashSet<String>>,
    pub rules: CommitRules,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitRules {
    pub enabled: bool,
    pub ignore_fixup_commits: bool,
    pub ignore_amend_commits: bool,
    pub ignore_squash_commits: bool,
    pub ignore_merge_commits: bool,
    pub ignore_revert_commits: bool,
    pub warn: RuleLevel,
    pub deny: RuleLevel,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleLevel {
    pub subject_length: Option<usize>,
    pub body_length: Option<usize>,
    pub no_scope: Option<bool>,
    pub no_body: Option<bool>,
    pub no_type: Option<bool>,
    pub no_breaking_change_footer: Option<bool>,
    pub regex_patterns: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            commit: CommitConfig {
                types: HashSet::from(
                    [
                        "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor",
                        "revert", "style", "test",
                    ]
                    .map(String::from),
                ),
                scopes: None,
                rules: CommitRules {
                    enabled: true,
                    ignore_fixup_commits: true,
                    ignore_amend_commits: true,
                    ignore_squash_commits: true,
                    ignore_merge_commits: true,
                    ignore_revert_commits: true,
                    warn: RuleLevel {
                        subject_length: Some(72),
                        body_length: Some(500),
                        no_scope: Some(false),
                        no_body: Some(false),
                        no_type: Some(true),
                        no_breaking_change_footer: Some(true),
                        regex_patterns: Some(vec![]),
                    },
                    deny: RuleLevel {
                        subject_length: Some(72),
                        body_length: Some(500),
                        no_scope: Some(false),
                        no_body: Some(false),
                        no_type: Some(true),
                        no_breaking_change_footer: Some(true),
                        regex_patterns: Some(vec![]),
                    },
                },
            },
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        match Self::load(path) {
            Ok(config) => config,
            Err(ConfigError::Validation(msg)) => {
                eprintln!("configuration validation failed: {}", msg);
                Self::default()
            }
            Err(_) => Self::default(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.commit.rules.validate()
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

impl CommitRules {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check if deny values are less than or equal to warn values
        if let (Some(warn_subject), Some(deny_subject)) =
            (self.warn.subject_length, self.deny.subject_length)
            && deny_subject <= warn_subject
        {
            return Err(ConfigError::Validation(format!(
                "deny.subject_length ({}) must be greater than warn.subject_length ({})",
                deny_subject, warn_subject
            )));
        }

        if let (Some(warn_body), Some(deny_body)) = (self.warn.body_length, self.deny.body_length)
            && deny_body <= warn_body
        {
            return Err(ConfigError::Validation(format!(
                "deny.body_length ({}) must be greater than warn.body_length ({})",
                deny_body, warn_body
            )));
        }

        Ok(())
    }
}

impl RuleLevel {
    pub fn get_subject_length(&self) -> Option<usize> {
        self.subject_length
    }

    pub fn get_body_length(&self) -> Option<usize> {
        self.body_length
    }

    pub fn get_no_scope(&self) -> bool {
        self.no_scope.unwrap_or(false)
    }

    pub fn get_no_body(&self) -> bool {
        self.no_body.unwrap_or(false)
    }

    pub fn get_no_type(&self) -> bool {
        self.no_type.unwrap_or(true)
    }

    pub fn get_no_breaking_change_footer(&self) -> bool {
        self.no_breaking_change_footer.unwrap_or(true)
    }

    pub fn get_regex_patterns(&self) -> Vec<String> {
        self.regex_patterns.as_ref().cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.warn.subject_length, Some(72));
        assert_eq!(config.commit.rules.deny.subject_length, Some(72));
        assert!(config.commit.types.contains("feat"));
        assert!(config.commit.types.contains("fix"));
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
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true

[commit.rules.warn]
subject_length = 50
body_length = 200
no_scope = true
no_body = false
no_type = true
no_breaking_change_footer = false
regex_patterns = []

[commit.rules.deny]
subject_length = 100
body_length = 400
no_scope = false
no_body = false
no_type = true
no_breaking_change_footer = false
regex_patterns = []
"#
        )
        .unwrap();

        let config = Config::load(file.path()).unwrap();
        assert_eq!(config.commit.types.len(), 3);
        assert!(config.commit.types.contains("feat"));
        assert_eq!(config.commit.rules.warn.subject_length, Some(50));
        assert_eq!(config.commit.rules.deny.subject_length, Some(100));
        assert!(config.commit.rules.warn.get_no_scope());

        let scopes = config.get_allowed_scopes().unwrap();
        assert!(scopes.contains("api"));
        assert!(scopes.contains("ui"));
    }

    #[test]
    fn test_load_or_default_fallback() {
        let config = Config::load_or_default("nonexistent.toml");
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.warn.subject_length, Some(72));
    }

    #[test]
    fn test_config_validation_fails() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
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
subject_length = 100

[commit.rules.deny]
subject_length = 50
"#
        )
        .unwrap();

        let result = Config::load(file.path());
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains(
                "deny.subject_length (50) must be greater than warn.subject_length (100)"
            ));
        } else {
            panic!("Expected ValidationError");
        }
    }
}
