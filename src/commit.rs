//! This module provides a small parser for conventional commit-style messages.
//! It extracts the commit type, optional scope, breaking-change marker,
//! subject, optional body, and structured footers. It is intentionally
//! permissive for bodies and enforces a simple, readable header grammar.
//!
//! Non-goals:
//! - enforcing allowed type/scope taxonomy
//! - reflowing or wrapping text
//!
//! See [`CommitMessage::parse`] for usage and more examples.

use std::collections::HashMap;

use nom::{
    IResult, Parser,
    bytes::complete::{is_not, take_while1},
    character::complete::{char, space0},
    combinator::{map, opt, rest},
    sequence::delimited,
};
use thiserror::Error;

/// parser errors for commit messages
#[derive(Debug, Error)]
pub enum ParseError {
    /// input didn't match the expected header/body/footer layout
    #[error("invalid commit format")]
    InvalidFormat,
}

/// a structured representation of a commit message
///
/// fields mirror conventional commit parts and some helpful derived flags.
/// all strings are kept as provided (after minimal trimming), with the body
/// preserving line breaks.
///
/// use [`CommitMessage::parse`] to build an instance from raw text.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitMessage {
    /// commit type like `feat`, `fix`, `docs`, `chore`, etc.
    pub commit_type: String,
    /// optional scope in parentheses from the header
    pub scope: Option<String>,
    /// true when `!` is present after type/scope or when a breaking footer
    /// exists
    pub breaking: bool,
    /// short imperative description (first line after the colon)
    pub subject: String,
    /// optional free-form body separated by a blank line from the header
    pub body: Option<String>,
    /// structured trailer lines of the form `Key: Value` (multi-line supported)
    pub footers: HashMap<String, String>,
}

impl CommitMessage {
    /// parse a raw commit message into a [`CommitMessage`].
    ///
    /// the parser expects the first line to be a valid header following the
    /// simplified conventional commits grammar documented at the top of this
    /// module. a blank line separates header from body/footers. once a valid
    /// footer line is detected after a blank line, following lines are
    /// considered footers, allowing multi-line values.
    ///
    /// examples
    ///
    /// ```rust
    /// use cocoa::commit::CommitMessage;
    ///
    /// let msg = "feat(auth)!: rotate tokens\n\nreasoning\n\nCloses: #42";
    /// let parsed = CommitMessage::parse(msg).unwrap();
    /// assert_eq!(parsed.commit_type, "feat");
    /// assert_eq!(parsed.scope.as_deref(), Some("auth"));
    /// assert!(parsed.breaking);
    /// assert_eq!(parsed.subject, "rotate tokens");
    /// assert!(parsed.body.unwrap().starts_with("reasoning"));
    /// assert_eq!(
    ///     parsed.footers.get("Closes").map(String::as_str),
    ///     Some("#42")
    /// );
    /// ```
    pub fn parse(message: &str) -> Result<Self, ParseError> {
        let mut lines = message.lines();
        let Some(header) = lines.next() else {
            return Err(ParseError::InvalidFormat);
        };

        let mut body_lines = Vec::new();
        let mut footer_lines = Vec::new();
        let mut in_footer = false;
        let mut blank_line_found = false;

        for line in lines {
            if line.trim().is_empty() && !blank_line_found {
                blank_line_found = true;
                continue;
            }

            if blank_line_found && is_footer_line(line) {
                in_footer = true;
            }

            if in_footer {
                footer_lines.push(line);
            } else if blank_line_found {
                body_lines.push(line);
            }
        }

        let (commit_type, scope, breaking, subject) = parse_header(header)
            .map(|(_, r)| r)
            .map_err(|_| ParseError::InvalidFormat)?;
        let body = if body_lines.is_empty() {
            None
        } else {
            Some(body_lines.join("\n").trim().to_string())
        };
        let footers = parse_footers(&footer_lines);

        let breaking = breaking
            || footers.contains_key("BREAKING CHANGE")
            || footers.contains_key("BREAKING-CHANGE");

        Ok(CommitMessage {
            commit_type,
            scope,
            breaking,
            subject,
            body,
            footers,
        })
    }

    /// true when the commit is a `fixup:` commit (by type)
    pub fn is_fixup(&self) -> bool {
        self.commit_type == "fixup"
    }

    /// true when the commit is a `squash:` commit (by type)
    pub fn is_squash(&self) -> bool {
        self.commit_type == "squash"
    }

    /// true for likely merge commits (subject starts with `Merge`)
    pub fn is_merge(&self) -> bool {
        self.subject.starts_with("Merge")
    }

    /// true for revert commits (`revert:` type or subject starts with `Revert`)
    pub fn is_revert(&self) -> bool {
        self.commit_type == "revert" || self.subject.starts_with("Revert")
    }

    /// unicode scalar count of the subject line
    pub fn get_subject_length(&self) -> usize {
        self.subject.chars().count()
    }

    /// unicode scalar count of the entire body (0 when absent)
    pub fn get_body_length(&self) -> usize {
        self.body.as_ref().map_or(0, |b| b.chars().count())
    }
}

/// allowed identifier characters for type and footer keys
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// parse the header into `(type, scope, breaking, subject)`
fn parse_header(input: &str) -> IResult<&str, (String, Option<String>, bool, String)> {
    let (input, commit_type) =
        map(take_while1(is_ident_char), |s: &str| s.to_string()).parse(input)?;
    let (input, scope) = opt(delimited(
        char('('),
        map(is_not(")"), |s: &str| s.to_string()),
        char(')'),
    ))
    .parse(input)?;
    let (input, bang) = opt(char('!')).parse(input)?;
    let (input, _) = (char(':'), space0).parse(input)?;
    let (input, subject) = map(rest, |s: &str| s.trim().to_string()).parse(input)?;
    let breaking = bang.is_some();
    Ok((input, (commit_type, scope, breaking, subject)))
}

/// detect whether a line matches the `Key: Value` footer pattern
fn is_footer_line(line: &str) -> bool {
    let Some((key, val)) = line
        .split_once(":")
        .map(|(key, val)| (key.trim(), val.trim()))
    else {
        return false;
    };

    // allow alphanumeric chars, underscore, and hyphen for footer keys
    // also allow "BREAKING CHANGE" as a key, because the space will not be picked
    // up otherwise
    key == "BREAKING CHANGE" || key.chars().all(is_ident_char) && !key.is_empty() && !val.is_empty()
}

/// parse trailer lines, supporting multi-line values by continuation
fn parse_footers(footer_lines: &[&str]) -> HashMap<String, String> {
    let mut footers = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in footer_lines {
        if is_footer_line(line) {
            if let Some(key) = current_key.take() {
                footers.insert(key, current_value.trim().to_string());
                current_value.clear();
            }
            if let Some(colon_pos) = line.find(':') {
                current_key = Some(line[..colon_pos].to_string());
                current_value = line[colon_pos + 1..].trim().to_string();
            }
        } else if current_key.is_some() {
            current_value.push('\n');
            current_value.push_str(line);
        }
    }

    if let Some(key) = current_key {
        footers.insert(key, current_value.trim().to_string());
    }

    footers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_commit() {
        let message = "feat: add new feature";
        let commit = CommitMessage::parse(message).unwrap();

        assert_eq!(commit.commit_type, "feat");
        assert_eq!(commit.scope, None);
        assert!(!commit.breaking);
        assert_eq!(commit.subject, "add new feature");
        assert_eq!(commit.body, None);
        assert!(commit.footers.is_empty());
    }

    #[test]
    fn test_parse_commit_with_scope() {
        let message = "fix(api): resolve authentication issue";
        let commit = CommitMessage::parse(message).unwrap();

        assert_eq!(commit.commit_type, "fix");
        assert_eq!(commit.scope, Some("api".to_string()));
        assert!(!commit.breaking);
        assert_eq!(commit.subject, "resolve authentication issue");
    }

    #[test]
    fn test_parse_breaking_change_with_exclamation() {
        let message = "feat!: remove deprecated API";
        let commit = CommitMessage::parse(message).unwrap();

        assert_eq!(commit.commit_type, "feat");
        assert_eq!(commit.scope, None);
        assert!(commit.breaking);
        assert_eq!(commit.subject, "remove deprecated API");
    }

    #[test]
    fn test_parse_commit_with_body() {
        let message = r#"feat: add user authentication

This commit adds JWT-based authentication
for all API endpoints."#;

        let commit = CommitMessage::parse(message).unwrap();

        assert_eq!(commit.commit_type, "feat");
        assert_eq!(commit.subject, "add user authentication");
        assert!(commit.body.is_some());
        assert!(
            commit
                .body
                .as_ref()
                .unwrap()
                .contains("JWT-based authentication")
        );
    }

    #[test]
    fn test_parse_commit_with_footers() {
        let message = r#"feat: add new feature

Some body text

Closes: #123
Reviewed-by: John Doe"#;

        let commit = CommitMessage::parse(message).unwrap();

        assert_eq!(commit.footers.get("Closes"), Some(&"#123".to_string()));
        assert_eq!(
            commit.footers.get("Reviewed-by"),
            Some(&"John Doe".to_string())
        );
    }

    #[test]
    fn test_parse_breaking_change_footer() {
        let message = r#"feat: change API response format

BREAKING CHANGE: API now returns data in a different format"#;

        let commit = CommitMessage::parse(message).unwrap();

        assert!(commit.breaking);
        assert_eq!(
            commit.footers.get("BREAKING CHANGE"),
            Some(&"API now returns data in a different format".to_string())
        );
    }

    #[test]
    fn test_invalid_format() {
        let message = "invalid commit message";
        let result = CommitMessage::parse(message);
        assert!(result.is_err());
    }

    #[test]
    fn test_special_commit_types() {
        let fixup = CommitMessage::parse("fixup: fix typo").unwrap();
        assert!(fixup.is_fixup());

        let squash = CommitMessage::parse("squash: combine commits").unwrap();
        assert!(squash.is_squash());

        let revert = CommitMessage::parse("revert: undo previous change").unwrap();
        assert!(revert.is_revert());

        let merge = CommitMessage::parse("feat: Merge branch 'feature'").unwrap();
        assert!(merge.is_merge());
    }

    #[test]
    fn test_length_calculations() {
        let commit = CommitMessage::parse("feat: short").unwrap();
        assert_eq!(commit.get_subject_length(), 5);
        assert_eq!(commit.get_body_length(), 0);

        let commit_with_body = CommitMessage::parse("feat: short\n\nLonger body text").unwrap();
        assert_eq!(commit_with_body.get_subject_length(), 5);
        assert_eq!(commit_with_body.get_body_length(), 16);
    }
}
