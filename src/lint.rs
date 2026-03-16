use std::fmt;

use console::style;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    commit::CommitMessage,
    config::{CommitRules, Config},
};

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

impl fmt::Display for LintViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol_styled = match self.severity {
            Severity::Error => style("×").red().bold(),
            Severity::Warning => style("◆").yellow().bold(),
            Severity::Info => style("!").blue().bold(),
        };

        write!(f, "{} {}", symbol_styled, self.message)
    }
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

    pub fn lint(&self, message: &str) -> LintResult {
        let mut violations = Vec::new();

        let commit = match CommitMessage::parse(message) {
            Ok(commit) => commit,
            Err(_) => {
                violations.push(LintViolation {
                    rule: "format".to_string(),
                    severity: Severity::Error,
                    message: "commit message does not follow conventional commits format"
                        .to_string(),
                    line: Some(1),
                    column: None,
                });
                return LintResult {
                    violations,
                    is_valid: false,
                };
            }
        };

        if self.should_ignore_commit(&commit) {
            return LintResult {
                violations: vec![],
                is_valid: true,
            };
        }

        if self.rules.enabled {
            self.check_type(&commit, &mut violations);
            self.check_scope(&commit, &mut violations);
            self.check_subject_length(&commit, &mut violations);
            self.check_body_length(&commit, &mut violations);
            self.check_required_body(&commit, &mut violations);
            self.check_breaking_change_footer(&commit, &mut violations);
            self.check_custom_patterns(&commit, message, &mut violations);
        }

        let is_valid = violations.iter().all(|v| v.severity != Severity::Error);

        LintResult {
            violations,
            is_valid,
        }
    }

    fn should_ignore_commit(&self, commit: &CommitMessage) -> bool {
        (self.rules.ignore_fixup_commits && commit.is_fixup())
            || (self.rules.ignore_squash_commits && commit.is_squash())
            || (self.rules.ignore_merge_commits && commit.is_merge())
            || (self.rules.ignore_revert_commits && commit.is_revert())
    }

    fn check_type(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        let warn_no_type = self.rules.warn.get_no_type();
        let deny_no_type = self.rules.deny.get_no_type();

        if commit.commit_type.is_empty() {
            if deny_no_type {
                violations.push(LintViolation {
                    rule: "no-type".to_string(),
                    severity: Severity::Error,
                    message: "commit type is required".to_string(),
                    line: Some(1),
                    column: None,
                });
            } else if warn_no_type {
                violations.push(LintViolation {
                    rule: "no-type".to_string(),
                    severity: Severity::Warning,
                    message: "commit type is missing".to_string(),
                    line: Some(1),
                    column: None,
                });
            }
            return;
        }

        let allowed_types = self.config.get_allowed_types();
        if !allowed_types.contains(&commit.commit_type) {
            violations.push(LintViolation {
                rule: "type-enum".to_string(),
                severity: Severity::Error,
                message: format!(
                    "invalid commit type '{}'. allowed types: {}",
                    commit.commit_type,
                    allowed_types.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
                line: Some(1),
                column: None,
            });
        }
    }

    fn check_scope(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        let warn_no_scope = self.rules.warn.get_no_scope();
        let deny_no_scope = self.rules.deny.get_no_scope();

        if commit.scope.is_none() {
            if deny_no_scope {
                violations.push(LintViolation {
                    rule: "no-scope".to_string(),
                    severity: Severity::Error,
                    message: "commit scope is required".to_string(),
                    line: Some(1),
                    column: None,
                });
            } else if warn_no_scope {
                violations.push(LintViolation {
                    rule: "no-scope".to_string(),
                    severity: Severity::Warning,
                    message: "commit scope is missing".to_string(),
                    line: Some(1),
                    column: None,
                });
            }
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
                    "invalid scope '{}'. allowed scopes: {}",
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

        if let Some(deny_length) = self.rules.deny.get_subject_length()
            && length > deny_length
        {
            violations.push(LintViolation {
                rule: "subject-max-length".to_string(),
                severity: Severity::Error,
                message: format!(
                    "subject line too long ({} chars). maximum is {} chars",
                    length, deny_length
                ),
                line: Some(1),
                column: None,
            });
            return;
        }

        if let Some(warn_length) = self.rules.warn.get_subject_length()
            && length > warn_length
        {
            violations.push(LintViolation {
                rule: "subject-max-length".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "subject line is long ({} chars). consider keeping it under {} chars",
                    length, warn_length
                ),
                line: Some(1),
                column: None,
            });
        }
    }

    fn check_body_length(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        let length = commit.get_body_length();

        if let Some(deny_length) = self.rules.deny.get_body_length()
            && length > deny_length
        {
            violations.push(LintViolation {
                rule: "body-max-length".to_string(),
                severity: Severity::Error,
                message: format!(
                    "body too long ({} chars). maximum is {} chars",
                    length, deny_length
                ),
                line: Some(3),
                column: None,
            });
            return;
        }

        if let Some(warn_length) = self.rules.warn.get_body_length()
            && length > warn_length
        {
            violations.push(LintViolation {
                rule: "body-max-length".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "body is long ({} chars). consider keeping it under {} chars",
                    length, warn_length
                ),
                line: Some(3),
                column: None,
            });
        }
    }

    fn check_required_body(&self, commit: &CommitMessage, violations: &mut Vec<LintViolation>) {
        let warn_no_body = self.rules.warn.get_no_body();
        let deny_no_body = self.rules.deny.get_no_body();

        if commit.body.is_none() {
            if deny_no_body {
                violations.push(LintViolation {
                    rule: "no-body".to_string(),
                    severity: Severity::Error,
                    message: "body is required".to_string(),
                    line: Some(3),
                    column: None,
                });
            } else if warn_no_body {
                violations.push(LintViolation {
                    rule: "no-body".to_string(),
                    severity: Severity::Warning,
                    message: "body is missing".to_string(),
                    line: Some(3),
                    column: None,
                });
            }
        }
    }

    fn check_breaking_change_footer(
        &self,
        commit: &CommitMessage,
        violations: &mut Vec<LintViolation>,
    ) {
        let warn_no_breaking_footer = self.rules.warn.get_no_breaking_change_footer();
        let deny_no_breaking_footer = self.rules.deny.get_no_breaking_change_footer();

        if commit.breaking
            && !commit.footers.contains_key("BREAKING CHANGE")
            && !commit.footers.contains_key("BREAKING-CHANGE")
        {
            if deny_no_breaking_footer {
                violations.push(LintViolation {
                    rule: "no-breaking-change-footer".to_string(),
                    severity: Severity::Error,
                    message: "breaking changes must include a BREAKING CHANGE footer".to_string(),
                    line: None,
                    column: None,
                });
            } else if warn_no_breaking_footer {
                violations.push(LintViolation {
                    rule: "no-breaking-change-footer".to_string(),
                    severity: Severity::Warning,
                    message: "breaking changes should include a BREAKING CHANGE footer".to_string(),
                    line: None,
                    column: None,
                });
            }
        }
    }

    fn check_custom_patterns(
        &self,
        _commit: &CommitMessage,
        message: &str,
        violations: &mut Vec<LintViolation>,
    ) {
        let warn_patterns = self.rules.warn.get_regex_patterns();
        let deny_patterns = self.rules.deny.get_regex_patterns();

        for pattern in &deny_patterns {
            let matched = Regex::new(pattern)
                .map(|re| re.is_match(message))
                .unwrap_or(false);
            if !matched {
                violations.push(LintViolation {
                    rule: "regex-pattern".to_string(),
                    severity: Severity::Error,
                    message: format!("message does not match required pattern: {}", pattern),
                    line: None,
                    column: None,
                });
            }
        }

        for pattern in &warn_patterns {
            if deny_patterns.contains(pattern) {
                continue;
            }
            let matched = Regex::new(pattern)
                .map(|re| re.is_match(message))
                .unwrap_or(false);
            if !matched {
                violations.push(LintViolation {
                    rule: "regex-pattern".to_string(),
                    severity: Severity::Warning,
                    message: format!("message should match pattern: {}", pattern),
                    line: None,
                    column: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::config::{CommitConfig, CommitRules, RuleLevel};

    fn create_test_config() -> Config {
        Config {
            ai: None,
            changelog: None,
            version: None,
            commit: CommitConfig {
                types: HashSet::from(["feat".to_string(), "fix".to_string()]),
                scopes: Some(HashSet::from(["api".to_string(), "ui".to_string()])),
                rules: CommitRules {
                    enabled: true,
                    ignore_fixup_commits: true,
                    ignore_amend_commits: true,
                    ignore_squash_commits: true,
                    ignore_merge_commits: true,
                    ignore_revert_commits: true,
                    warn: RuleLevel {
                        subject_length: Some(50),
                        body_length: Some(100),
                        no_scope: Some(false),
                        no_body: Some(false),
                        no_type: Some(true),
                        no_breaking_change_footer: Some(false),
                        regex_patterns: Some(vec![]),
                    },
                    deny: RuleLevel {
                        subject_length: Some(100),
                        body_length: Some(200),
                        no_scope: Some(false),
                        no_body: Some(false),
                        no_type: Some(true),
                        no_breaking_change_footer: Some(false),
                        regex_patterns: Some(vec![]),
                    },
                },
            },
        }
    }

    #[test]
    fn test_valid_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add new feature");

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_invalid_type() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("invalid: add new feature");

        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "type-enum");
        assert_eq!(result.violations[0].severity, Severity::Error);
    }

    #[test]
    fn test_subject_warning() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let long_subject = "a".repeat(60); // 60 chars > 50 (warn) but < 100 (deny)
        let message = format!("feat: {}", long_subject);
        let result = linter.lint(&message);

        assert!(result.is_valid); // Should be valid (only warning)
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "subject-max-length");
        assert_eq!(result.violations[0].severity, Severity::Warning);
    }

    #[test]
    fn test_subject_error() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let very_long_subject = "a".repeat(120); // 120 chars > 100 (deny)
        let message = format!("feat: {}", very_long_subject);
        let result = linter.lint(&message);

        assert!(!result.is_valid); // Should be invalid (error)
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "subject-max-length");
        assert_eq!(result.violations[0].severity, Severity::Error);
    }

    #[test]
    fn test_invalid_scope() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("feat(invalid): add new feature");

        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "scope-enum");
    }

    #[test]
    fn test_valid_scope() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("feat(api): add new feature");

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_ignore_fixup_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        let result = linter.lint("fixup: fix typo");

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_breaking_change_footer_requirement() {
        let mut config = create_test_config();
        config.commit.rules.deny.no_breaking_change_footer = Some(true);
        let linter = Linter::new(&config);

        let result = linter.lint("feat!: breaking change without footer");

        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "no-breaking-change-footer");
        assert_eq!(result.violations[0].severity, Severity::Error);
    }

    #[test]
    fn test_disabled_linting() {
        let mut config = create_test_config();
        config.commit.rules.enabled = false;
        let linter = Linter::new(&config);

        let result = linter.lint("invalid: this should not be linted");

        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }
}
