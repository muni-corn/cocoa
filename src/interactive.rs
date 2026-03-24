//! Interactive commit creation for `cocoa commit`.
//!
//! Guides the user through a series of prompts to compose a valid conventional
//! commit message, validates it with the linter, and optionally executes the
//! commit.

use thiserror::Error;

use crate::{Config, git_ops::GitOperations, lint::Linter};

/// Errors that can occur during interactive commit creation.
#[derive(Debug, Error)]
pub enum InteractiveError {
    /// An interactive prompt returned an error (e.g., terminal I/O failure).
    #[error("interactive prompt failed: {0}")]
    Prompt(String),

    /// The assembled message failed linting.
    #[error("commit message failed validation: {0}")]
    Lint(String),

    /// The git commit operation failed.
    #[error("commit failed: {0}")]
    Commit(String),

    /// The user aborted the interactive session.
    #[error("commit aborted by user")]
    Aborted,
}

/// The decomposed parts of a conventional commit message, collected from
/// interactive prompts.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitParts {
    /// Conventional commit type (e.g., `feat`, `fix`).
    pub commit_type: String,
    /// Optional scope (e.g., `api`, `ui`).
    pub scope: Option<String>,
    /// Whether this commit introduces a breaking change.
    pub breaking: bool,
    /// Description of the breaking change, placed in a `BREAKING CHANGE:`
    /// footer.
    pub breaking_description: Option<String>,
    /// Short imperative subject line.
    pub subject: String,
    /// Optional multi-line commit body.
    pub body: Option<String>,
    /// Optional issue reference footer lines (e.g., `Closes #123`).
    pub issue_refs: Option<String>,
}

impl CommitParts {
    /// Assembles a fully formatted conventional commit message string from the
    /// decomposed parts.
    ///
    /// The output format follows the Conventional Commits specification:
    /// - Header: `type(scope)!: subject`
    /// - Optional blank-line-separated body
    /// - Optional `BREAKING CHANGE:` footer
    /// - Optional issue reference footer
    ///
    /// # Example
    ///
    /// ```rust
    /// use cocoa::interactive::CommitParts;
    ///
    /// let parts = CommitParts {
    ///     commit_type: "feat".into(),
    ///     scope: Some("auth".into()),
    ///     breaking: true,
    ///     breaking_description: Some("tokens now expire after 1h".into()),
    ///     subject: "rotate session tokens".into(),
    ///     body: None,
    ///     issue_refs: Some("Closes #42".into()),
    /// };
    ///
    /// let msg = parts.to_message();
    /// assert!(msg.starts_with("feat(auth)!: rotate session tokens"));
    /// assert!(msg.contains("BREAKING CHANGE: tokens now expire after 1h"));
    /// assert!(msg.contains("Closes #42"));
    /// ```
    pub fn to_message(&self) -> String {
        assemble_message(self)
    }
}

/// Builds the full commit message string from [`CommitParts`].
fn assemble_message(parts: &CommitParts) -> String {
    // build header: type(scope)!: subject
    let mut header = parts.commit_type.clone();
    if let Some(scope) = &parts.scope {
        header.push('(');
        header.push_str(scope);
        header.push(')');
    }
    if parts.breaking {
        header.push('!');
    }
    header.push_str(": ");
    header.push_str(&parts.subject);

    let mut message = header;

    if let Some(body) = &parts.body {
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            message.push_str("\n\n");
            message.push_str(trimmed);
        }
    }

    // footers must be separated from the body (or header) by a blank line
    let mut footers: Vec<String> = Vec::new();

    if parts.breaking
        && let Some(desc) = &parts.breaking_description
    {
        let trimmed = desc.trim();
        if !trimmed.is_empty() {
            footers.push(format!("BREAKING CHANGE: {}", trimmed));
        }
    }

    if let Some(refs) = &parts.issue_refs {
        let trimmed = refs.trim();
        if !trimmed.is_empty() {
            footers.push(trimmed.to_string());
        }
    }

    if !footers.is_empty() {
        message.push_str("\n\n");
        message.push_str(&footers.join("\n"));
    }

    message
}

/// Validates a commit message against the configured lint rules.
///
/// Returns `Ok(())` when the message passes, or
/// [`InteractiveError::Lint`] with a summary of violations on failure.
fn validate_message(config: &Config, message: &str) -> Result<(), InteractiveError> {
    let linter = Linter::new(config);
    let result = linter.lint(message);

    if result.is_valid {
        return Ok(());
    }

    let errors: Vec<String> = result
        .violations
        .iter()
        .filter(|v| matches!(v.severity, crate::lint::Severity::Error))
        .map(|v| format!("[{}] {}", v.rule, v.message))
        .collect();

    Err(InteractiveError::Lint(errors.join("; ")))
}

/// Runs the interactive commit wizard.
///
/// Guides the user through type, scope, subject, body, breaking change, and
/// issue reference prompts. On success the assembled message is validated and
/// the commit is executed via `git_ops`. In dry-run mode the message is
/// printed to stdout and no commit is made.
///
/// Returns the assembled commit message on success.
pub fn run(
    config: &Config,
    git_ops: &dyn GitOperations,
    dry_run: bool,
) -> Result<String, InteractiveError> {
    use dialoguer::theme::ColorfulTheme;
    let theme = ColorfulTheme::default();

    let commit_type = prompts::commit_type(&theme, config)?;
    let scope = prompts::scope(&theme, config)?;
    let subject = prompts::subject(&theme, config, &commit_type, scope.as_deref())?;
    let body = prompts::body(&theme)?;
    let (breaking, breaking_description) = prompts::breaking(&theme)?;
    let issue_refs = prompts::issue_refs(&theme)?;

    let parts = CommitParts {
        commit_type,
        scope,
        breaking,
        breaking_description,
        subject,
        body,
        issue_refs,
    };

    let message = assemble_message(&parts);

    // validate before committing
    validate_message(config, &message)?;

    if dry_run {
        println!("{}", message);
        return Ok(message);
    }

    git_ops
        .create_commit(&message)
        .map_err(|e| InteractiveError::Commit(e.to_string()))?;

    Ok(message)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{
        Config,
        config::{CommitConfig, CommitRules, RuleLevel},
    };

    fn default_config() -> Config {
        Config::default()
    }

    fn config_with_deny_len(subject_deny: usize) -> Config {
        Config {
            commit: CommitConfig {
                rules: CommitRules {
                    deny: RuleLevel {
                        subject_length: Some(subject_deny),
                        ..RuleLevel::default()
                    },
                    ..CommitRules::default()
                },
                ..CommitConfig::default()
            },
            ..Config::default()
        }
    }

    // --- CommitParts::to_message tests ---

    #[test]
    fn test_simple_message() {
        let parts = CommitParts {
            commit_type: "feat".into(),
            scope: None,
            breaking: false,
            breaking_description: None,
            subject: "add login page".into(),
            body: None,
            issue_refs: None,
        };
        assert_eq!(parts.to_message(), "feat: add login page");
    }

    #[test]
    fn test_message_with_scope() {
        let parts = CommitParts {
            commit_type: "fix".into(),
            scope: Some("api".into()),
            breaking: false,
            breaking_description: None,
            subject: "handle null response".into(),
            body: None,
            issue_refs: None,
        };
        assert_eq!(parts.to_message(), "fix(api): handle null response");
    }

    #[test]
    fn test_message_with_breaking_change() {
        let parts = CommitParts {
            commit_type: "feat".into(),
            scope: Some("auth".into()),
            breaking: true,
            breaking_description: Some("tokens now expire after 1h".into()),
            subject: "rotate session tokens".into(),
            body: None,
            issue_refs: None,
        };
        let msg = parts.to_message();
        assert!(msg.starts_with("feat(auth)!: rotate session tokens"));
        assert!(msg.contains("BREAKING CHANGE: tokens now expire after 1h"));
    }

    #[test]
    fn test_message_with_body() {
        let parts = CommitParts {
            commit_type: "refactor".into(),
            scope: None,
            breaking: false,
            breaking_description: None,
            subject: "extract helper functions".into(),
            body: Some("Moves shared utilities to a dedicated module.".into()),
            issue_refs: None,
        };
        let msg = parts.to_message();
        assert_eq!(
            msg,
            "refactor: extract helper functions\n\nMoves shared utilities to a dedicated module."
        );
    }

    #[test]
    fn test_message_with_issue_refs() {
        let parts = CommitParts {
            commit_type: "fix".into(),
            scope: None,
            breaking: false,
            breaking_description: None,
            subject: "prevent crash on empty input".into(),
            body: None,
            issue_refs: Some("Closes #99".into()),
        };
        let msg = parts.to_message();
        assert_eq!(msg, "fix: prevent crash on empty input\n\nCloses #99");
    }

    #[test]
    fn test_message_full() {
        let parts = CommitParts {
            commit_type: "feat".into(),
            scope: Some("payments".into()),
            breaking: true,
            breaking_description: Some("old payment API removed".into()),
            subject: "migrate to stripe v3".into(),
            body: Some("Replaces the legacy payment module.".into()),
            issue_refs: Some("Closes #200".into()),
        };
        let msg = parts.to_message();
        let expected = "feat(payments)!: migrate to stripe v3\n\n\
                        Replaces the legacy payment module.\n\n\
                        BREAKING CHANGE: old payment API removed\n\
                        Closes #200";
        assert_eq!(msg, expected);
    }

    #[test]
    fn test_breaking_flag_without_description() {
        // breaking: true but no description; header gets `!` but no footer
        let parts = CommitParts {
            commit_type: "chore".into(),
            scope: None,
            breaking: true,
            breaking_description: None,
            subject: "drop Node 14 support".into(),
            body: None,
            issue_refs: None,
        };
        let msg = parts.to_message();
        assert!(msg.starts_with("chore!: drop Node 14 support"));
        assert!(!msg.contains("BREAKING CHANGE"));
    }

    #[test]
    fn test_empty_body_ignored() {
        let parts = CommitParts {
            commit_type: "docs".into(),
            scope: None,
            breaking: false,
            breaking_description: None,
            subject: "update readme".into(),
            body: Some("   ".into()), // whitespace-only
            issue_refs: None,
        };
        assert_eq!(parts.to_message(), "docs: update readme");
    }

    // --- validate_message tests ---

    #[test]
    fn test_validate_valid_message() {
        let config = default_config();
        assert!(validate_message(&config, "feat: add something").is_ok());
    }

    #[test]
    fn test_validate_invalid_type() {
        let mut config = default_config();
        // narrow the allowed types to just "feat"
        config.commit.types = HashSet::from(["feat".to_string()]);
        let result = validate_message(&config, "unknowntype: something");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("validation"));
    }

    #[test]
    fn test_validate_subject_too_long() {
        let config = config_with_deny_len(10);
        let result = validate_message(&config, "feat: this subject is way too long to pass");
        assert!(result.is_err());
    }
}

/// Interactive prompt implementations.
pub(crate) mod prompts {
    use dialoguer::{Confirm, Editor, FuzzySelect, Input, Select, theme::ColorfulTheme};
    use rust_i18n::t;

    use crate::{Config, interactive::InteractiveError};

    /// Prompts the user to select a commit type from configured types.
    pub fn commit_type(theme: &ColorfulTheme, config: &Config) -> Result<String, InteractiveError> {
        let mut types: Vec<&str> = config.commit.types.iter().map(|s| s.as_str()).collect();
        types.sort_unstable();

        let idx = Select::with_theme(theme)
            .with_prompt(t!("interactive.prompt.commit_type").as_ref())
            .items(&types)
            .default(0)
            .interact()
            .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

        Ok(types[idx].to_string())
    }

    /// Prompts the user for an optional commit scope.
    ///
    /// When the config restricts scopes to a known set, a fuzzy-search
    /// selector is shown. Otherwise a free-text input is used. The user can
    /// always skip to leave the scope empty.
    pub fn scope(
        theme: &ColorfulTheme,
        config: &Config,
    ) -> Result<Option<String>, InteractiveError> {
        let add_scope = Confirm::with_theme(theme)
            .with_prompt(t!("interactive.prompt.add_scope").as_ref())
            .default(false)
            .interact()
            .unwrap_or(false);

        if !add_scope {
            return Ok(None);
        }

        if let Some(scopes) = &config.commit.scopes {
            // build a sorted list with a trailing free-text escape hatch
            let mut scope_list: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
            scope_list.sort();
            scope_list.push(t!("interactive.prompt.custom_scope").to_string());

            let idx = FuzzySelect::with_theme(theme)
                .with_prompt(t!("interactive.prompt.select_scope").as_ref())
                .items(&scope_list)
                .default(0)
                .interact()
                .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

            if idx < scope_list.len() - 1 {
                return Ok(Some(scope_list[idx].clone()));
            }
            // fall through to free-text input
        }

        let scope: String = Input::with_theme(theme)
            .with_prompt(t!("interactive.prompt.scope").as_ref())
            .interact_text()
            .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

        let scope = scope.trim().to_string();
        Ok(if scope.is_empty() { None } else { Some(scope) })
    }

    /// Prompts for the short subject line, enforcing configured length limits.
    ///
    /// A character counter is shown live via the prompt label. The configured
    /// deny threshold is enforced as a hard validation error; the warn
    /// threshold is shown as a hint in the prompt text.
    pub fn subject(
        theme: &ColorfulTheme,
        config: &Config,
        commit_type: &str,
        scope: Option<&str>,
    ) -> Result<String, InteractiveError> {
        let warn_len = config.commit.rules.warn.subject_length.unwrap_or(50);
        let deny_len = config.commit.rules.deny.subject_length.unwrap_or(72);

        // show a preview of the header prefix so the user can see the total length
        let prefix = match scope {
            Some(s) => format!("{}({}): ", commit_type, s),
            None => format!("{}: ", commit_type),
        };

        let prompt = t!(
            "interactive.prompt.subject",
            warn = warn_len,
            deny = deny_len,
            prefix = prefix.len()
        )
        .to_string();

        let subject: String = Input::with_theme(theme)
            .with_prompt(&prompt)
            .validate_with(|s: &String| {
                if s.len() > deny_len {
                    Err(t!(
                        "interactive.prompt.subject_too_long",
                        len = s.len(),
                        max = deny_len
                    )
                    .to_string())
                } else {
                    Ok(())
                }
            })
            .interact_text()
            .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

        Ok(subject.trim().to_string())
    }

    /// Prompts for an optional multi-line commit body via the system editor.
    ///
    /// The user is first asked whether they want a body; if yes, the
    /// `$EDITOR` (or a fallback) is opened. Returning an empty file is treated
    /// as no body.
    pub fn body(theme: &ColorfulTheme) -> Result<Option<String>, InteractiveError> {
        let add_body = Confirm::with_theme(theme)
            .with_prompt(t!("interactive.prompt.add_body").as_ref())
            .default(false)
            .interact()
            .unwrap_or(false);

        if !add_body {
            return Ok(None);
        }

        let body = Editor::new()
            .require_save(true)
            .edit(
                "# Enter commit body above. Lines starting with '#' are ignored.\n\
                 # Save and close the editor to continue. Leave empty to skip.",
            )
            .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

        Ok(body
            .map(|s| {
                // strip comment lines (like git does)
                let cleaned: String = s
                    .lines()
                    .filter(|l| !l.trim_start().starts_with('#'))
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                cleaned.trim().to_string()
            })
            .filter(|s| !s.is_empty()))
    }

    /// Prompts whether the commit is a breaking change and, if so, for a
    /// short description to place in the `BREAKING CHANGE:` footer.
    pub fn breaking(theme: &ColorfulTheme) -> Result<(bool, Option<String>), InteractiveError> {
        let is_breaking = Confirm::with_theme(theme)
            .with_prompt(t!("interactive.prompt.breaking").as_ref())
            .default(false)
            .interact()
            .unwrap_or(false);

        if !is_breaking {
            return Ok((false, None));
        }

        let description: String = Input::with_theme(theme)
            .with_prompt(t!("interactive.prompt.breaking_description").as_ref())
            .interact_text()
            .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

        Ok((true, Some(description.trim().to_string())))
    }

    /// Prompts for optional issue reference footer lines.
    ///
    /// The user is first asked whether they want to add references; if yes,
    /// a free-text input is collected (e.g., `Closes #123, Refs #456`).
    pub fn issue_refs(theme: &ColorfulTheme) -> Result<Option<String>, InteractiveError> {
        let add_refs = Confirm::with_theme(theme)
            .with_prompt(t!("interactive.prompt.add_refs").as_ref())
            .default(false)
            .interact()
            .unwrap_or(false);

        if !add_refs {
            return Ok(None);
        }

        let refs: String = Input::with_theme(theme)
            .with_prompt(t!("interactive.prompt.issue_refs").as_ref())
            .interact_text()
            .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

        let refs = refs.trim().to_string();
        Ok(if refs.is_empty() { None } else { Some(refs) })
    }
}
