use std::collections::HashMap;

use nom::{
    IResult, Parser,
    bytes::complete::{is_not, take_while1},
    character::complete::{char, space0},
    combinator::{map, opt, rest},
    sequence::delimited,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid commit format")]
    InvalidFormat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommitMessage {
    pub commit_type: String,
    pub scope: Option<String>,
    pub breaking: bool,
    pub subject: String,
    pub body: Option<String>,
    pub footers: HashMap<String, String>,
}

impl CommitMessage {
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

    pub fn is_fixup(&self) -> bool {
        self.commit_type == "fixup"
    }

    pub fn is_squash(&self) -> bool {
        self.commit_type == "squash"
    }

    pub fn is_merge(&self) -> bool {
        self.subject.starts_with("Merge")
    }

    pub fn is_revert(&self) -> bool {
        self.commit_type == "revert" || self.subject.starts_with("Revert")
    }

    pub fn get_subject_length(&self) -> usize {
        self.subject.chars().count()
    }

    pub fn get_body_length(&self) -> usize {
        self.body.as_ref().map_or(0, |b| b.chars().count())
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

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

fn is_footer_line(input: &str) -> bool {
    // BREAKING CHANGE / BREAKING-CHANGE
    if input.starts_with("BREAKING CHANGE:") || input.starts_with("BREAKING-CHANGE:") {
        let rest = input.split_once(':').map(|x| x.1).unwrap_or("");
        return !rest.trim().is_empty();
    }
    // key: value with word/hyphen key
    if let Some(colon) = input.find(':') {
        let (key, val) = input.split_at(colon);
        let val = &val[1..];
        let valid_key = key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        return valid_key && !val.trim().is_empty();
    }
    false
}

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
