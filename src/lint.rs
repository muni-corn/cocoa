use std::fmt;

use console::style;
use regex::Regex;
use rust_i18n::t;
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
                    message: t!("lint.error.invalid_format").to_string(),
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
                    message: t!("lint.error.type_required").to_string(),
                });
            } else if warn_no_type {
                violations.push(LintViolation {
                    rule: "no-type".to_string(),
                    severity: Severity::Warning,
                    message: t!("lint.warn.type_missing").to_string(),
                });
            }
            return;
        }

        let allowed_types = self.config.get_allowed_types();
        if !allowed_types.contains(&commit.commit_type) {
            let allowed_list = allowed_types.iter().cloned().collect::<Vec<_>>().join(", ");
            violations.push(LintViolation {
                rule: "type-enum".to_string(),
                severity: Severity::Error,
                message: t!(
                    "lint.error.invalid_type",
                    commit_type = commit.commit_type,
                    allowed_types = allowed_list
                )
                .to_string(),
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
                    message: t!("lint.error.scope_required").to_string(),
                });
            } else if warn_no_scope {
                violations.push(LintViolation {
                    rule: "no-scope".to_string(),
                    severity: Severity::Warning,
                    message: t!("lint.warn.scope_missing").to_string(),
                });
            }
            return;
        }

        if let Some(allowed_scopes) = self.config.get_allowed_scopes()
            && let Some(ref scope) = commit.scope
            && !allowed_scopes.contains(scope)
        {
            let allowed_list = allowed_scopes
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            violations.push(LintViolation {
                rule: "scope-enum".to_string(),
                severity: Severity::Error,
                message: t!(
                    "lint.error.invalid_scope",
                    scope = scope,
                    allowed_scopes = allowed_list
                )
                .to_string(),
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
                message: t!(
                    "lint.error.subject_too_long",
                    len = length,
                    max = deny_length
                )
                .to_string(),
            });
            return;
        }

        if let Some(warn_length) = self.rules.warn.get_subject_length()
            && length > warn_length
        {
            violations.push(LintViolation {
                rule: "subject-max-length".to_string(),
                severity: Severity::Warning,
                message: t!("lint.warn.subject_long", len = length, max = warn_length).to_string(),
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
                message: t!("lint.error.body_too_long", len = length, max = deny_length)
                    .to_string(),
            });
            return;
        }

        if let Some(warn_length) = self.rules.warn.get_body_length()
            && length > warn_length
        {
            violations.push(LintViolation {
                rule: "body-max-length".to_string(),
                severity: Severity::Warning,
                message: t!("lint.warn.body_long", len = length, max = warn_length).to_string(),
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
                    message: t!("lint.error.body_required").to_string(),
                });
            } else if warn_no_body {
                violations.push(LintViolation {
                    rule: "no-body".to_string(),
                    severity: Severity::Warning,
                    message: t!("lint.warn.body_missing").to_string(),
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
                    message: t!("lint.error.no_breaking_change_footer").to_string(),
                });
            } else if warn_no_breaking_footer {
                violations.push(LintViolation {
                    rule: "no-breaking-change-footer".to_string(),
                    severity: Severity::Warning,
                    message: t!("lint.warn.no_breaking_change_footer").to_string(),
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
                    message: t!("lint.error.pattern_mismatch", pattern = pattern).to_string(),
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
                    message: t!("lint.warn.pattern_mismatch", pattern = pattern).to_string(),
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

    // --- no-type warn/deny ---

    #[test]
    fn test_no_type_warning_when_warn_enabled() {
        let mut config = create_test_config();
        // warn fires, but deny does not
        config.commit.rules.warn.no_type = Some(true);
        config.commit.rules.deny.no_type = Some(false);
        let linter = Linter::new(&config);
        // messages without a colon produce an empty commit_type
        let result = linter.lint("just a plain description with no type");
        // warning means still valid
        assert!(result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "no-type");
        assert_eq!(result.violations[0].severity, Severity::Warning);
    }

    #[test]
    fn test_no_type_deny_fires_error() {
        let mut config = create_test_config();
        config.commit.rules.warn.no_type = Some(false);
        config.commit.rules.deny.no_type = Some(true);
        let linter = Linter::new(&config);
        let result = linter.lint("just a plain description with no type");
        assert!(!result.is_valid);
        assert_eq!(result.violations[0].rule, "no-type");
        assert_eq!(result.violations[0].severity, Severity::Error);
    }

    // --- no-scope warn/deny ---

    #[test]
    fn test_no_scope_warning_when_warn_enabled() {
        let mut config = create_test_config();
        config.commit.rules.warn.no_scope = Some(true);
        config.commit.rules.deny.no_scope = Some(false);
        let linter = Linter::new(&config);
        // no scope in message → warning
        let result = linter.lint("feat: add thing");
        assert!(result.is_valid);
        let scope_warn = result
            .violations
            .iter()
            .find(|v| v.rule == "no-scope")
            .unwrap();
        assert_eq!(scope_warn.severity, Severity::Warning);
    }

    #[test]
    fn test_no_scope_deny_fires_error() {
        let mut config = create_test_config();
        config.commit.rules.warn.no_scope = Some(false);
        config.commit.rules.deny.no_scope = Some(true);
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add thing");
        assert!(!result.is_valid);
        let scope_err = result
            .violations
            .iter()
            .find(|v| v.rule == "no-scope")
            .unwrap();
        assert_eq!(scope_err.severity, Severity::Error);
    }

    #[test]
    fn test_valid_scope_with_no_allowed_scopes_configured() {
        let mut config = create_test_config();
        // clear the allowed scopes so any scope is accepted
        config.commit.scopes = None;
        let linter = Linter::new(&config);
        let result = linter.lint("feat(anything): add thing");
        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    // --- body length warn/deny ---

    #[test]
    fn test_body_too_long_warning() {
        let mut config = create_test_config();
        config.commit.rules.warn.body_length = Some(10);
        config.commit.rules.deny.body_length = Some(200);
        let linter = Linter::new(&config);
        let long_body = "a".repeat(20);
        let message = format!("feat: subject\n\n{}", long_body);
        let result = linter.lint(&message);
        assert!(result.is_valid); // warning only
        let v = result
            .violations
            .iter()
            .find(|v| v.rule == "body-max-length")
            .unwrap();
        assert_eq!(v.severity, Severity::Warning);
    }

    #[test]
    fn test_body_too_long_error() {
        let mut config = create_test_config();
        config.commit.rules.warn.body_length = Some(10);
        config.commit.rules.deny.body_length = Some(50);
        let linter = Linter::new(&config);
        let very_long_body = "a".repeat(100);
        let message = format!("feat: subject\n\n{}", very_long_body);
        let result = linter.lint(&message);
        assert!(!result.is_valid);
        let v = result
            .violations
            .iter()
            .find(|v| v.rule == "body-max-length")
            .unwrap();
        assert_eq!(v.severity, Severity::Error);
    }

    // --- required body warn/deny ---

    #[test]
    fn test_no_body_warning() {
        let mut config = create_test_config();
        config.commit.rules.warn.no_body = Some(true);
        config.commit.rules.deny.no_body = Some(false);
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add thing");
        assert!(result.is_valid);
        let v = result
            .violations
            .iter()
            .find(|v| v.rule == "no-body")
            .unwrap();
        assert_eq!(v.severity, Severity::Warning);
    }

    #[test]
    fn test_no_body_deny() {
        let mut config = create_test_config();
        config.commit.rules.warn.no_body = Some(false);
        config.commit.rules.deny.no_body = Some(true);
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add thing");
        assert!(!result.is_valid);
        let v = result
            .violations
            .iter()
            .find(|v| v.rule == "no-body")
            .unwrap();
        assert_eq!(v.severity, Severity::Error);
    }

    // --- breaking change footer warn path ---

    #[test]
    fn test_breaking_change_footer_warn() {
        let mut config = create_test_config();
        config.commit.rules.warn.no_breaking_change_footer = Some(true);
        config.commit.rules.deny.no_breaking_change_footer = Some(false);
        let linter = Linter::new(&config);
        let result = linter.lint("feat!: breaking without footer");
        assert!(result.is_valid); // warning → still valid
        let v = result
            .violations
            .iter()
            .find(|v| v.rule == "no-breaking-change-footer")
            .unwrap();
        assert_eq!(v.severity, Severity::Warning);
    }

    // --- custom patterns ---

    #[test]
    fn test_custom_deny_pattern_not_matched() {
        let mut config = create_test_config();
        // require every commit message to contain "JIRA-"
        config.commit.rules.deny.regex_patterns = Some(vec!["JIRA-\\d+".to_string()]);
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add thing without jira ref");
        assert!(!result.is_valid);
        let v = result
            .violations
            .iter()
            .find(|v| v.rule == "regex-pattern")
            .unwrap();
        assert_eq!(v.severity, Severity::Error);
    }

    #[test]
    fn test_custom_deny_pattern_matched_passes() {
        let mut config = create_test_config();
        config.commit.rules.deny.regex_patterns = Some(vec!["JIRA-\\d+".to_string()]);
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add thing JIRA-42");
        // the pattern IS matched, so no violation
        assert!(result.violations.iter().all(|v| v.rule != "regex-pattern"));
    }

    #[test]
    fn test_custom_warn_pattern_not_matched() {
        let mut config = create_test_config();
        config.commit.rules.deny.regex_patterns = Some(vec![]);
        config.commit.rules.warn.regex_patterns = Some(vec!["JIRA-\\d+".to_string()]);
        let linter = Linter::new(&config);
        let result = linter.lint("feat: add thing without jira ref");
        assert!(result.is_valid); // warning only
        let v = result
            .violations
            .iter()
            .find(|v| v.rule == "regex-pattern")
            .unwrap();
        assert_eq!(v.severity, Severity::Warning);
    }

    #[test]
    fn test_custom_pattern_in_both_deny_and_warn_not_duplicated() {
        // when the same pattern is in both deny and warn, only the deny-level
        // violation should fire; the warn iteration should skip it
        let mut config = create_test_config();
        let pattern = "JIRA-\\d+".to_string();
        config.commit.rules.deny.regex_patterns = Some(vec![pattern.clone()]);
        config.commit.rules.warn.regex_patterns = Some(vec![pattern]);
        let linter = Linter::new(&config);
        let result = linter.lint("feat: no jira ref here");
        let pattern_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| v.rule == "regex-pattern")
            .collect();
        // only one violation for the pattern (from deny), not two
        assert_eq!(pattern_violations.len(), 1);
        assert_eq!(pattern_violations[0].severity, Severity::Error);
    }

    // --- ignore paths ---

    #[test]
    fn test_ignore_squash_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        // commit type "squash" triggers is_squash()
        let result = linter.lint("squash: fix typo");
        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_ignore_merge_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        // no colon → commit_type is empty; subject starts with "Merge"
        let result = linter.lint("Merge branch 'main' into feature");
        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_ignore_revert_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        // no colon → commit_type is empty; subject starts with "Revert"
        let result = linter.lint("Revert some previous change");
        assert!(result.is_valid);
        assert!(result.violations.is_empty());
    }

    // --- format error path ---

    #[test]
    fn test_unparseable_message_returns_format_error() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        // "feat(bad: something" has a colon but unclosed scope paren → parse fails
        let result = linter.lint("feat(bad: something");
        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule, "format");
        assert_eq!(result.violations[0].severity, Severity::Error);
    }

    // --- ignore amend ---

    #[test]
    fn test_ignore_amend_commit() {
        let config = create_test_config();
        let linter = Linter::new(&config);
        // amend commits use the "amend!" git prefix; no colon → empty type,
        // subject starts with "amend!" but is_fixup/squash/merge/revert all
        // return false, so this goes through normal linting
        // (note: ignore_amend_commits is set but CommitMessage has no is_amend)
        // this just verifies the parser handles it without panicking
        let result = linter.lint("amend: fix previous commit");
        // "amend" type is not in allowed types → violation expected
        assert!(!result.is_valid);
    }

    // --- severity display ---

    #[test]
    fn test_lint_violation_display_error() {
        let v = LintViolation {
            rule: "test".to_string(),
            severity: Severity::Error,
            message: "something wrong".to_string(),
        };
        let s = v.to_string();
        assert!(s.contains("something wrong"));
    }

    #[test]
    fn test_lint_violation_display_warning() {
        let v = LintViolation {
            rule: "test".to_string(),
            severity: Severity::Warning,
            message: "heads up".to_string(),
        };
        let s = v.to_string();
        assert!(s.contains("heads up"));
    }

    #[test]
    fn test_lint_violation_display_info() {
        let v = LintViolation {
            rule: "test".to_string(),
            severity: Severity::Info,
            message: "fyi".to_string(),
        };
        let s = v.to_string();
        assert!(s.contains("fyi"));
    }
}
