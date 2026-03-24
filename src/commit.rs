//! This module provides a small parser for conventional commit-style messages.
//! It extracts the commit type, optional scope, breaking-change marker,
//! subject, optional body, and structured footers. It is intentionally
//! permissive for bodies and enforces a simple, readable header grammar.
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

/// Parser errors for commit messages.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Input didn't match the expected header/body/footer layout.
    #[error("invalid commit format")]
    InvalidFormat,
}

/// A structured representation of a commit message.
///
/// Fields mirror conventional commit parts and some helpful derived flags.
/// All strings are kept as provided (after minimal trimming), with the body
/// preserving line breaks.
///
/// Use [`CommitMessage::parse`] to build an instance from raw text.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitMessage {
    /// Commit type like `feat`, `fix`, `docs`, `chore`, etc.
    pub commit_type: String,
    /// Optional scope in parentheses from the header.
    pub scope: Option<String>,
    /// True when `!` is present after type/scope or when a breaking footer
    /// exists.
    pub breaking: bool,
    /// Short imperative description (first line after the colon).
    pub subject: String,
    /// Optional free-form body separated by a blank line from the header.
    pub body: Option<String>,
    /// Structured trailer lines of the form `Key: Value` (multi-line
    /// supported).
    pub footers: HashMap<String, String>,
}

impl CommitMessage {
    /// Parse a raw commit message into a [`CommitMessage`].
    ///
    /// The parser expects the first line to be a valid header following the
    /// simplified conventional commits grammar documented at the top of this
    /// module. A blank line separates header from body/footers. Once a valid
    /// footer line is detected after a blank line, following lines are
    /// considered footers, allowing multi-line values.
    ///
    /// ## Example
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
        let message = strip_git_context(message);
        let lines = message.lines();
        let mut header_lines = Vec::new();
        let mut body_lines = Vec::new();
        let mut footer_lines = Vec::new();

        enum Section {
            Header,
            Body,
            Footers,
        }

        let mut current_section = Section::Header;

        for line in lines {
            match current_section {
                Section::Header => {
                    if line.trim().is_empty() {
                        current_section = Section::Body
                    } else {
                        header_lines.push(line);
                    }
                }
                Section::Body => {
                    if is_footer_line(line) {
                        current_section = Section::Footers;
                        footer_lines.push(line);
                    } else {
                        body_lines.push(line);
                    }
                }
                Section::Footers => {
                    footer_lines.push(line);
                }
            }
        }

        // parse the header into conventional commit data. if the header contains a
        // colon, we'll assume the commit summary has a type and maybe a scope. if
        // there's no colon at all, we assume the entire commit summary is just the
        // subject.
        let header = header_lines.join(" ");
        let (commit_type, scope, breaking, subject) = if header.contains(":") {
            parse_header(&header)
                .map(|(_, r)| r)
                .map_err(|_| ParseError::InvalidFormat)?
        } else {
            (String::new(), None, false, header)
        };

        let body = if body_lines.is_empty() {
            None
        } else {
            Some(body_lines.join(" ").trim().to_string())
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

    /// Returns true when the commit is a `fixup:` commit (by type).
    pub fn is_fixup(&self) -> bool {
        self.commit_type == "fixup"
    }

    /// Returns true when the commit is a `squash:` commit (by type).
    pub fn is_squash(&self) -> bool {
        self.commit_type == "squash"
    }

    /// Returns true for likely merge commits (subject starts with `Merge`).
    pub fn is_merge(&self) -> bool {
        self.subject.starts_with("Merge")
    }

    /// Returns true for revert commits (`revert:` type or subject starts with
    /// `Revert`).
    pub fn is_revert(&self) -> bool {
        self.commit_type == "revert" || self.subject.starts_with("Revert")
    }

    /// Returns the Unicode scalar count of the subject line.
    pub fn get_subject_length(&self) -> usize {
        self.subject.len()
    }

    /// Returns the length of the entire body (0 when absent).
    pub fn get_body_length(&self) -> usize {
        self.body.as_ref().map_or(0, |b| b.len())
    }
}

/// Determines if a character is allowed in identifier contexts for type and
/// footer keys.
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Represents parsed commit header components: type, scope, breaking flag, and
/// subject.
type CommitHeader = (String, Option<String>, bool, String);

/// Parses a header into `(type, scope, breaking, subject)` components.
fn parse_header(input: &str) -> IResult<&str, CommitHeader> {
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

/// Determines whether a line matches the `Key: Value` footer pattern.
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

/// Parses trailer lines, supporting multi-line values by continuation.
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
            current_value.push(' ');
            current_value.push_str(line);
        }
    }

    if let Some(key) = current_key {
        footers.insert(key, current_value.trim().to_string());
    }

    footers
}

/// Strips git-added context from a raw commit message.
///
/// Removes two kinds of noise that git writes into `COMMIT_EDITMSG`:
///
/// 1. The scissors line (any `#`-prefixed line containing `>8`) and everything
///    below it; including verbose diffs added by `git commit --verbose`.
/// 2. Comment lines; any line whose first non-whitespace character is `#`.
///
/// This mirrors git's own cleanup behavior so that only the user-authored
/// content is returned. The result is trimmed of leading and trailing
/// whitespace.
///
/// # Example
///
/// ```rust
/// use cocoa::commit::strip_git_context;
///
/// let raw = "feat: add cocoa\n\n# Please enter the commit message\n# On branch main\n# ------------------------ >8 ------------------------\ndiff --git a/foo b/foo\n";
/// assert_eq!(strip_git_context(raw), "feat: add cocoa");
/// ```
pub fn strip_git_context(message: &str) -> String {
    // truncate at the scissors line (# ---- >8 ----) and everything below it
    let above_scissors = message
        .lines()
        .take_while(|l| {
            let trimmed = l.trim_start();
            !(trimmed.starts_with('#') && trimmed.contains(">8"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    // strip remaining comment lines (lines starting with #)
    above_scissors
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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
        assert_eq!(
            commit.body,
            Some("This commit adds JWT-based authentication for all API endpoints.".to_string())
        );
    }

    #[test]
    fn test_parse_commit_with_comments() {
        let message = r#"fix(commit): remove comments from commit messages before parsing

Fixes a problem where cocoa would count the entirety of a commit text file as its body.
# Please enter the commit message for your changes. Lines starting
# with '#' will be ignored, and an empty message aborts the commit.
#
# On branch main
# Your branch is up to date with 'origin/main'.
#
# Changes to be committed:
#	modified:   src/commit.rs
#
# Changes not staged for commit:
#	modified:   src/commit.rs
#
# ------------------------ >8 ------------------------
# Do not modify or remove the line above.
# Everything below it will be ignored.
diff --git a/src/commit.rs b/src/commit.rs
index 4bf40a1..88b9714 100644
--- a/src/commit.rs
+++ b/src/commit.rs
@@ -22,6 +22,9 @@ pub enum ParseError {
     /// Input didn't match the expected header/body/footer layout.
     #[error("invalid commit format")]
     InvalidFormat,
+
+    #[error("couldn't prettify message: {0}")]
+    PrettifyError(#[from] git2::Error),
 }
 
 /// A structured representation of a commit message.
@@ -76,6 +79,7 @@ impl CommitMessage {
     /// );
     /// ```
     pub fn parse(message: &str) -> Result<Self, ParseError> {
+        let message = git2::message_prettify(message, None)?;
         let lines = message.lines();
         let mut header_lines = Vec::new();
         let mut body_lines = Vec::new();
"#;

        let commit = CommitMessage::parse(message).unwrap();

        assert_eq!(commit.commit_type, "fix");
        assert_eq!(commit.scope, Some("commit".to_string()));
        assert_eq!(
            commit.subject,
            "remove comments from commit messages before parsing"
        );
        assert_eq!(
            commit.body,
            Some("Fixes a problem where cocoa would count the entirety of a commit text file as its body.".to_string())
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
    fn test_long_lines() {
        let message = r#"test: test really long messages that break up over
multiple lines and are really really annoying to deal with

BREAKING CHANGE: this footer value is really really really long and hopefully won't
actually break the stuff we so lovingly programmed into cocoa because i put a lot
of passion into my work and i will be very sad when tests fail
Reviewed-by: municorn himself, probably, though honestly most of the work is just
being done by `cargo test`
"#;

        let commit = CommitMessage::parse(message).unwrap();

        assert_eq!(commit.commit_type, "test");
        assert_eq!(commit.scope, None);
        assert_eq!(
            commit.subject,
            "test really long messages that break up over multiple lines and are really really annoying to deal with"
        );
        assert_eq!(
            commit.footers.get("BREAKING CHANGE"),
            Some(
                &"this footer value is really really really long and hopefully won't actually break the stuff we so lovingly programmed into cocoa because i put a lot of passion into my work and i will be very sad when tests fail".to_string()
            )
        );
        assert_eq!(
            commit.footers.get("Reviewed-by"),
            Some(&"municorn himself, probably, though honestly most of the work is just being done by `cargo test`".to_string())
        );
    }

    #[test]
    fn test_no_colon() {
        let message = "commit message that's just a subject";
        let commit = CommitMessage::parse(message).unwrap();
        assert_eq!(commit.subject, "commit message that's just a subject");
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

    // --- strip_git_context ---

    #[test]
    fn test_strip_git_context_clean_message() {
        // already-clean messages pass through unchanged
        assert_eq!(strip_git_context("feat: add cocoa"), "feat: add cocoa");
    }

    #[test]
    fn test_strip_git_context_strips_comment_lines() {
        let msg = "fix: correct typo\n\n# On branch main\n# Changes to be committed:\n#\tmodified: foo.rs";
        assert_eq!(strip_git_context(msg), "fix: correct typo");
    }

    #[test]
    fn test_strip_git_context_strips_scissors_and_diff() {
        let msg = "feat: add cocoa\n\n# ------------------------ >8 ------------------------\ndiff --git a/foo b/foo\nindex abc..def 100644\n--- a/foo\n+++ b/foo";
        assert_eq!(strip_git_context(msg), "feat: add cocoa");
    }

    #[test]
    fn test_strip_git_context_strips_comments_and_scissors() {
        // realistic full COMMIT_EDITMSG as produced by git commit --verbose
        let msg = concat!(
            "feat: add cocoa\n",
            "\n",
            "# Please enter the commit message for your changes. Lines starting\n",
            "# with '#' will be ignored, and an empty message aborts the commit.\n",
            "#\n",
            "# On branch main\n",
            "# Your branch is up to date with 'origin/main'.\n",
            "#\n",
            "# Changes to be committed:\n",
            "#\tmodified:   flake.nix\n",
            "#\n",
            "# ------------------------ >8 ------------------------\n",
            "# Do not modify or remove the line above.\n",
            "# Everything below it will be ignored.\n",
            "diff --git a/flake.nix b/flake.nix\n",
            "index 8aad7bfe..70b2247e 100644\n",
            "--- a/flake.nix\n",
            "+++ b/flake.nix\n",
            "@@ -99,6 +99,10 @@\n",
        );
        assert_eq!(strip_git_context(msg), "feat: add cocoa");
    }

    #[test]
    fn test_strip_git_context_preserves_body() {
        let msg = concat!(
            "fix(parser): correct off-by-one error\n",
            "\n",
            "The tokenizer was counting from 1 instead of 0.\n",
            "\n",
            "# On branch main\n",
            "# ------------------------ >8 ------------------------\n",
            "diff --git a/src/parser.rs b/src/parser.rs\n",
        );
        assert_eq!(
            strip_git_context(msg),
            "fix(parser): correct off-by-one error\n\nThe tokenizer was counting from 1 instead of 0."
        );
    }

    #[test]
    fn test_strip_git_context_empty_message() {
        assert_eq!(strip_git_context(""), "");
        assert_eq!(strip_git_context("   "), "");
    }

    #[test]
    fn test_strip_git_context_only_comments() {
        let msg = "# On branch main\n# Changes to be committed:\n#\tmodified: foo.rs";
        assert_eq!(strip_git_context(msg), "");
    }
}
