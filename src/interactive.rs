//! Interactive commit creation for `cocoa commit`.
//!
//! Guides the user through a series of prompts to compose a valid conventional
//! commit message, validates it with the linter, and optionally executes the
//! commit.

use thiserror::Error;

use crate::{Config, Linter, git_ops::GitOperations};

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
    let _ = (config, git_ops, dry_run);
    unimplemented!("interactive commit prompts not yet implemented")
}

/// Interactive prompt implementations.
pub(crate) mod prompts {
    use dialoguer::{Confirm, FuzzySelect, Input, Select, theme::ColorfulTheme};

    use crate::{Config, interactive::InteractiveError};

    /// Prompts the user to select a commit type from configured types.
    pub fn commit_type(theme: &ColorfulTheme, config: &Config) -> Result<String, InteractiveError> {
        let mut types: Vec<&str> = config.commit.types.iter().map(|s| s.as_str()).collect();
        types.sort_unstable();

        let idx = Select::with_theme(theme)
            .with_prompt("select commit type")
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
            .with_prompt("add a scope? (optional)")
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
            scope_list.push("(enter custom scope)".to_string());

            let idx = FuzzySelect::with_theme(theme)
                .with_prompt("select scope (type to filter)")
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
            .with_prompt("scope")
            .interact_text()
            .map_err(|e| InteractiveError::Prompt(e.to_string()))?;

        let scope = scope.trim().to_string();
        Ok(if scope.is_empty() { None } else { Some(scope) })
    }
}
