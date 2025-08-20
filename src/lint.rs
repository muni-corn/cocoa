use regex::Regex;
use serde::{Deserialize, Serialize};

use thiserror::Error;

use crate::{
    commit::CommitMessage,
    config::{CommitRules, Config},
};

#[derive(Debug, Error)]
pub enum LintError {
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintViolation {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintResult {
    pub violations: Vec<LintViolation>,
    pub is_valid: bool,
}

pub struct Linter<'a> {
    config: &'a Config,
    rules: &'a CommitRules,
}

impl<'a> Linter<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            rules: &config.commit.rules,
        }
    }

    pub fn lint(&self, message: &str) -> Result<LintResult, LintError> {
        let mut violations = Vec::new();

        let commit = match CommitMessage::parse(message) {
            Ok(commit) => commit,
            Err(_) => {
                violations.push(LintViolation {
                    rule: "format".to_string(),
                    severity: Severity::Error,
                    message: "Commit message does not follow conventional commits format"
                        .to_string(),
                    line: Some(1),
                    column: None,
                });
                return Ok(LintResult {
                    violations,
                    is_valid: false,
                });
            }
        };

        if self.should_ignore_commit(&commit) {
            return Ok(LintResult {
                violations: vec![],
                is_valid: true,
            });
        }

        if self.rules.enabled {
            self.check_type(&commit, &mut violations);
            self.check_scope(&commit, &mut violations);
            self.check_subject_length(&commit, &mut violations);
            self.check_body_length(&commit, &mut violations);
            self.check_required_body(&commit, &mut violations);
            self.check_breaking_change_footer(&commit, &mut violations);
            self.check_custom_patterns(&commit, message, &mut violations)?;
        }

        let is_valid = violations.iter().all(|v| v.severity != Severity::Error);

        Ok(LintResult {
            violations,
            is_valid,
        })
    }

    fn should_ignore_commit(&self, commit: &CommitMessage) -> bool {
        (self.rules.ignore_fixup_commits && commit.is_fixup())
            || (self.rules.ignore_squash_commits && commit.is_squash())
            || (self.rules.ignore_merge_commits && commit.is_merge())
            || (self.rules.ignore_revert_commits && commit.is_revert())
    }

    fn check_type(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        if self.rules.require_type && commit.commit_type.is_empty() {
            violations.push(LintViolation {
                rule: "type-required".to_string(),
                severity: Severity::Error,
                message: "Commit type is required".to_string(),
                line: Some(1),
                column: None,
            });
            return;
        }

        let allowed_types = self.config.get_allowed_types();
        if !allowed_types.contains(&commit.commit_type) {
            violations.push(LintViolation {
                rule: "type-enum".to_string(),
                severity: Severity::Error,
                message: format!(
                    "Invalid commit type '{}'. Allowed types: {}",
                    commit.commit_type,
                    allowed_types.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
                line: Some(1),
                column: None,
            });
        }
    }

    fn check_scope(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        if self.rules.require_scope && commit.scope.is_none() {
            violations.push(LintViolation {
                rule: "scope-required".to_string(),
                severity: Severity::Error,
                message: "Commit scope is required".to_string(),
                line: Some(1),
                column: None,
            });
            return;
        }

        if let Some(allowed_scopes) = self.config.get_allowed_scopes()
            && let Some(ref scope) = commit.scope
            && !allowed_scopes.contains(scope)
        {
            violations.push(LintViolation {
                rule: "scope-enum".to_string(),
                severity: Severity::Error,
                message: format!(
                    "Invalid scope '{}'. Allowed scopes: {}",
                    scope,
                    allowed_scopes
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                line: Some(1),
                column: None,
            });
        }
    }

    fn check_subject_length(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        let length = commit.get_subject_length();
        if length > self.rules.max_subject_length {
            violations.push(LintViolation {
                rule: "subject-max-length".to_string(),
                severity: Severity::Error,
                message: format!(
                    "Subject line too long ({} chars). Maximum is {} chars",
                    length, self.rules.max_subject_length
                ),
                line: Some(1),
                column: None,
            });
        }
    }

    fn check_body_length(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        let length = commit.get_body_length();
        if length > self.rules.max_body_length {
            violations.push(LintViolation {
                rule: "body-max-length".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "Body too long ({} chars). Maximum is {} chars",
                    length, self.rules.max_body_length
                ),
                line: Some(3),
                column: None,
            });
        }
    }

    fn check_required_body(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        if self.rules.require_body && commit.body.is_none() {
            violations.push(LintViolation {
                rule: "body-required".to_string(),
                severity: Severity::Error,
                message: "Body is required".to_string(),
                line: Some(3),
                column: None,
            });
        }
    }

    fn check_breaking_change_footer(
        &self,
        commit: &CommitMessage,
        violations: &mut Vec<LintViolation>,
    ) {
        if self.rules.require_breaking_change_footer
            && commit.breaking
            && !commit.footers.contains_key("BREAKING CHANGE")
            && !commit.footers.contains_key("BREAKING-CHANGE")
        {
            violations.push(LintViolation {
                rule: "breaking-change-footer".to_string(),
                severity: Severity::Error,
                message: "Breaking changes must include a BREAKING CHANGE footer".to_string(),
                line: None,
                column: None,
            });
        }
    }

    fn check_custom_patterns(
        &self,
        _commit: &CommitMessage,
        message: &str,
        violations: &mut Vec<LintViolation>,
    ) -> Result<(), LintError> {
        for pattern in &self.rules.regex_patterns {
            let regex = Regex::new(pattern)?;
            if !regex.is_match(message) {
                violations.push(LintViolation {
                    rule: "custom-pattern".to_string(),
                    severity: Severity::Warning,
                    message: format!("Message does not match required pattern: {}", pattern),
                    line: None,
                    column: None,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CommitConfig, CommitRules};

    fn create_test_config() -> Config {
        Config {
            commit: CommitConfig {
                types: vec!["feat".to_string(), "fix".to_string()],
                scopes: Some(vec!["api".to_string(), "ui".to_string()]),
                rules: CommitRules {
                    enabled: true,
                    max_subject_length: 50,
                    max_body_length: 100,
                    require_scope: false,
                    require_body: false,
                    require_type: true,
                    require_breaking_change_footer: false,
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

    #[test]
    fn test_valid_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add new feature").unwrap();

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_invalid_type() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("invalid: add new feature").unwrap();

        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "type-enum");
        assert_eq!(result.violations[0].severity, Severity::Error);
    }

    #[test]
    fn test_subject_too_long() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let long_subject = "a".repeat(60);
        let message = format!("feat: {}", long_subject);
        let result = linter.lint(&message).unwrap();

        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "subject-max-length");
    }

    #[test]
    fn test_invalid_scope() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("feat(invalid): add new feature").unwrap();

        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "scope-enum");
    }

    #[test]
    fn test_valid_scope() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("feat(api): add new feature").unwrap();

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_ignore_fixup_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("fixup: fix typo").unwrap();

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_breaking_change_footer_requirement() {
        let mut config = create_test_config();
        config.commit.rules.require_breaking_change_footer = true;
        let linter = Linter::new(&config);

        let result = linter
            .lint("feat!: breaking change without footer")
            .unwrap();

        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "breaking-change-footer");
    }

    #[test]
    fn test_disabled_linting() {
        let mut config = create_test_config();
        config.commit.rules.enabled = false;
        let linter = Linter::new(&config);

        let result = linter.lint("invalid: this should not be linted").unwrap();

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }
}
