//! Changelog output format renderers.

use crate::{
    changelog::{Changelog, ChangelogEntry, ChangelogError, ChangelogVersion, OutputFormat},
    config::ChangelogConfig,
};

/// Render a `Changelog` in the requested output format.
pub fn render(
    changelog: &Changelog,
    format: &OutputFormat,
    _config: &ChangelogConfig,
) -> Result<String, ChangelogError> {
    match format {
        OutputFormat::Markdown => Ok(render_markdown(changelog)),
        OutputFormat::Json => Err(ChangelogError::Render(
            "JSON format not yet implemented".to_string(),
        )),
        OutputFormat::Html => Err(ChangelogError::Render(
            "HTML format not yet implemented".to_string(),
        )),
        OutputFormat::ReStructuredText => Err(ChangelogError::Render(
            "RST format not yet implemented".to_string(),
        )),
        OutputFormat::AsciiDoc => Err(ChangelogError::Render(
            "AsciiDoc format not yet implemented".to_string(),
        )),
        OutputFormat::Template(_) => Err(ChangelogError::Render(
            "Template format not yet implemented".to_string(),
        )),
    }
}

// ─── Markdown ────────────────────────────────────────────────────────────────

/// Render the changelog as GitHub-flavored Markdown.
pub fn render_markdown(changelog: &Changelog) -> String {
    let mut out = String::from("# Changelog\n");

    for version in &changelog.versions {
        out.push('\n');
        out.push_str(&markdown_version_heading(version));
        out.push('\n');

        if !version.breaking_changes.is_empty() {
            out.push_str("\n### Breaking Changes\n\n");
            for entry in &version.breaking_changes {
                out.push_str(&markdown_entry(entry));
            }
        }

        for section in &version.sections {
            out.push_str(&format!("\n### {}\n\n", section.title));
            for entry in &section.entries {
                out.push_str(&markdown_entry(entry));
            }
        }

        out.push_str("\n---\n");
    }

    out
}

fn markdown_version_heading(version: &ChangelogVersion) -> String {
    let name = version.version.as_deref().unwrap_or("Unreleased");
    match &version.date {
        Some(date) => format!("## [{}] - {}", name, date),
        None => format!("## [{}]", name),
    }
}

fn markdown_entry(entry: &ChangelogEntry) -> String {
    let scope = entry
        .scope
        .as_ref()
        .map(|s| format!("**{}:** ", s))
        .unwrap_or_default();
    format!("- {}{} (`{}`)\n", scope, entry.subject, entry.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::{ChangelogSection, ChangelogVersion};

    fn sample_entry(id: &str, typ: &str, subject: &str) -> ChangelogEntry {
        ChangelogEntry {
            id: id.to_string(),
            commit_type: typ.to_string(),
            scope: None,
            breaking: false,
            subject: subject.to_string(),
            body: None,
            author: "Author".to_string(),
            timestamp: 1000,
            date: "2024-01-01".to_string(),
        }
    }

    fn sample_changelog() -> Changelog {
        Changelog {
            versions: vec![ChangelogVersion {
                version: Some("v1.0.0".to_string()),
                date: Some("2024-01-01".to_string()),
                breaking_changes: vec![],
                sections: vec![
                    ChangelogSection {
                        commit_type: "feat".to_string(),
                        title: "Features".to_string(),
                        entries: vec![sample_entry("abc12345", "feat", "add login")],
                    },
                    ChangelogSection {
                        commit_type: "fix".to_string(),
                        title: "Bug Fixes".to_string(),
                        entries: vec![sample_entry("def67890", "fix", "fix crash")],
                    },
                ],
            }],
        }
    }

    #[test]
    fn test_render_markdown_contains_version() {
        let cl = sample_changelog();
        let out = render_markdown(&cl);
        assert!(out.contains("# Changelog"));
        assert!(out.contains("## [v1.0.0] - 2024-01-01"));
        assert!(out.contains("### Features"));
        assert!(out.contains("### Bug Fixes"));
        assert!(out.contains("add login"));
        assert!(out.contains("`abc12345`"));
    }

    #[test]
    fn test_render_markdown_unreleased() {
        let cl = Changelog {
            versions: vec![ChangelogVersion {
                version: None,
                date: None,
                breaking_changes: vec![],
                sections: vec![ChangelogSection {
                    commit_type: "feat".to_string(),
                    title: "Features".to_string(),
                    entries: vec![sample_entry("abc12345", "feat", "new thing")],
                }],
            }],
        };
        let out = render_markdown(&cl);
        assert!(out.contains("## [Unreleased]"));
    }

    #[test]
    fn test_render_markdown_breaking_changes() {
        let mut cl = sample_changelog();
        cl.versions[0].breaking_changes = vec![{
            let mut e = sample_entry("brk12345", "feat", "api overhaul");
            e.breaking = true;
            e
        }];
        let out = render_markdown(&cl);
        assert!(out.contains("### Breaking Changes"));
        assert!(out.contains("api overhaul"));
    }

    #[test]
    fn test_render_markdown_scope() {
        let mut cl = sample_changelog();
        cl.versions[0].sections[0].entries[0].scope = Some("auth".to_string());
        let out = render_markdown(&cl);
        assert!(out.contains("**auth:**"));
    }
}
