//! Changelog generation from git history.

pub mod parser;
pub mod renderer;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A single entry in the changelog, representing one commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    /// Short commit SHA (8 characters).
    pub id: String,
    /// Conventional commit type (e.g., "feat", "fix").
    pub commit_type: String,
    /// Optional scope from the commit header.
    pub scope: Option<String>,
    /// Whether this commit introduces a breaking change.
    pub breaking: bool,
    /// Short description from the commit subject.
    pub subject: String,
    /// Optional longer body text.
    pub body: Option<String>,
    /// Commit author name.
    pub author: String,
    /// Unix timestamp of the commit.
    pub timestamp: i64,
    /// Formatted date string (per `ChangelogConfig::date_format`).
    pub date: String,
}

/// A section within a changelog version, grouping entries by commit type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogSection {
    /// Conventional commit type for this section.
    pub commit_type: String,
    /// Human-readable section title.
    pub title: String,
    /// Entries in this section, sorted deterministically.
    pub entries: Vec<ChangelogEntry>,
}

/// All changes for a single version (a tag, or "Unreleased").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogVersion {
    /// Tag name, or `None` for unreleased changes.
    pub version: Option<String>,
    /// Release date string, or `None` when not yet released.
    pub date: Option<String>,
    /// Breaking-change entries extracted across all sections.
    pub breaking_changes: Vec<ChangelogEntry>,
    /// Sections grouped by commit type, in display order.
    pub sections: Vec<ChangelogSection>,
}

/// Complete generated changelog with all versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changelog {
    /// Versions in order from newest (Unreleased) to oldest.
    pub versions: Vec<ChangelogVersion>,
}

/// Supported output formats for changelog rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Json,
    Html,
    ReStructuredText,
    AsciiDoc,
    /// Path to a Jinja2-compatible template file.
    Template(String),
}

impl OutputFormat {
    /// Parse an output format from a string identifier.
    ///
    /// Recognized values: `markdown`, `md`, `json`, `html`, `rst`,
    /// `restructuredtext`, `asciidoc`, `adoc`, and `template:<path>`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Some(Self::Markdown),
            "json" => Some(Self::Json),
            "html" => Some(Self::Html),
            "rst" | "restructuredtext" => Some(Self::ReStructuredText),
            "asciidoc" | "adoc" => Some(Self::AsciiDoc),
            s if s.starts_with("template:") => Some(Self::Template(s[9..].to_string())),
            _ => None,
        }
    }
}

/// Errors that can occur during changelog generation.
#[derive(Debug, Error)]
pub enum ChangelogError {
    /// A git operation failed.
    #[error("git error: {0}")]
    Git(String),

    /// Rendering the output failed.
    #[error("render error: {0}")]
    Render(String),

    /// Template loading or evaluation failed.
    #[error("template error: {0}")]
    Template(String),

    /// An i/o operation failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parse() {
        assert_eq!(
            OutputFormat::parse("markdown"),
            Some(OutputFormat::Markdown)
        );
        assert_eq!(OutputFormat::parse("md"), Some(OutputFormat::Markdown));
        assert_eq!(OutputFormat::parse("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse("html"), Some(OutputFormat::Html));
        assert_eq!(
            OutputFormat::parse("rst"),
            Some(OutputFormat::ReStructuredText)
        );
        assert_eq!(
            OutputFormat::parse("restructuredtext"),
            Some(OutputFormat::ReStructuredText)
        );
        assert_eq!(
            OutputFormat::parse("asciidoc"),
            Some(OutputFormat::AsciiDoc)
        );
        assert_eq!(OutputFormat::parse("adoc"), Some(OutputFormat::AsciiDoc));
        assert_eq!(
            OutputFormat::parse("template:/my/template.jinja"),
            Some(OutputFormat::Template("/my/template.jinja".to_string()))
        );
        assert_eq!(
            OutputFormat::parse("MARKDOWN"),
            Some(OutputFormat::Markdown)
        );
        assert_eq!(OutputFormat::parse("unknown"), None);
    }
}
