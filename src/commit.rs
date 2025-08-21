use std::collections::HashMap;

use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid commit format")]
    InvalidFormat,

    #[error("invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
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
        let lines: Vec<&str> = message.lines().collect();
        if lines.is_empty() {
            return Err(ParseError::InvalidFormat);
        }

        let header = lines[0];
        let mut body_lines = Vec::new();
        let mut footer_lines = Vec::new();
        let mut in_footer = false;
        let mut blank_line_found = false;

        for line in lines.iter().skip(1) {
            if line.trim().is_empty() && !blank_line_found {
                blank_line_found = true;
                continue;
            }

            if blank_line_found && Self::is_footer_line(line) {
                in_footer = true;
            }

            if in_footer {
                footer_lines.push(*line);
            } else if blank_line_found {
                body_lines.push(*line);
            }
        }

        let (commit_type, scope, breaking, subject) = Self::parse_header(header)?;
        let body = if body_lines.is_empty() {
            None
        } else {
            Some(body_lines.join("\n").trim().to_string())
        };
        let footers = Self::parse_footers(&footer_lines);

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

    fn parse_header(header: &str) -> Result<(String, Option<String>, bool, String), ParseError> {
        let re = Regex::new(r"^(\w+)(\(([^)]+)\))?(!?):\s*(.+)$")?;

        if let Some(caps) = re.captures(header) {
            let commit_type = caps.get(1).unwrap().as_str().to_string();
            let scope = caps.get(3).map(|m| m.as_str().to_string());
            let breaking = caps.get(4).is_some_and(|m| m.as_str() == "!");
            let subject = caps.get(5).unwrap().as_str().to_string();

            Ok((commit_type, scope, breaking, subject))
        } else {
            Err(ParseError::InvalidFormat)
        }
    }

    fn is_footer_line(line: &str) -> bool {
        let footer_re = Regex::new(r"^[\w-]+:\s+.+$").unwrap();
        let breaking_re = Regex::new(r"^BREAKING[ -]CHANGE:\s+.+$").unwrap();
        footer_re.is_match(line) || breaking_re.is_match(line)
    }

    fn parse_footers(footer_lines: &[&str]) -> HashMap<String, String> {
        let mut footers = HashMap::new();
        let mut current_key: Option<String> = None;
        let mut current_value = String::new();

        for line in footer_lines {
            if Self::is_footer_line(line) {
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
