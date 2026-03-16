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
        OutputFormat::Json => render_json(changelog),
        OutputFormat::Html => Ok(render_html(changelog)),
        OutputFormat::ReStructuredText => Ok(render_rst(changelog)),
        OutputFormat::AsciiDoc => Ok(render_asciidoc(changelog)),
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

// ─── JSON ─────────────────────────────────────────────────────────────────────

/// Serialize the changelog to pretty-printed JSON.
pub fn render_json(changelog: &Changelog) -> Result<String, ChangelogError> {
    serde_json::to_string_pretty(changelog).map_err(|e| ChangelogError::Render(e.to_string()))
}

// ─── HTML ─────────────────────────────────────────────────────────────────────

/// Render the changelog as a standalone HTML document.
pub fn render_html(changelog: &Changelog) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"UTF-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\
         <title>Changelog</title>\n\
         </head>\n\
         <body>\n\
         <h1>Changelog</h1>\n",
    );

    for version in &changelog.versions {
        out.push_str("<section>\n");
        out.push_str(&format!("<h2>{}</h2>\n", html_version_heading(version)));

        if !version.breaking_changes.is_empty() {
            out.push_str("<h3>Breaking Changes</h3>\n<ul>\n");
            for entry in &version.breaking_changes {
                out.push_str(&html_entry(entry));
            }
            out.push_str("</ul>\n");
        }

        for section in &version.sections {
            out.push_str(&format!("<h3>{}</h3>\n<ul>\n", html_escape(&section.title)));
            for entry in &section.entries {
                out.push_str(&html_entry(entry));
            }
            out.push_str("</ul>\n");
        }

        out.push_str("</section>\n");
    }

    out.push_str("</body>\n</html>\n");
    out
}

fn html_version_heading(version: &ChangelogVersion) -> String {
    let name = html_escape(version.version.as_deref().unwrap_or("Unreleased"));
    match &version.date {
        Some(date) => format!("{} &mdash; {}", name, html_escape(date)),
        None => name,
    }
}

fn html_entry(entry: &ChangelogEntry) -> String {
    let scope = entry
        .scope
        .as_ref()
        .map(|s| format!("<strong>{}:</strong> ", html_escape(s)))
        .unwrap_or_default();
    format!(
        "<li>{}{} (<code>{}</code>)</li>\n",
        scope,
        html_escape(&entry.subject),
        html_escape(&entry.id)
    )
}

/// Escape characters that have special meaning in HTML.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ─── reStructuredText
// ─────────────────────────────────────────────────────────

/// Render the changelog as reStructuredText.
pub fn render_rst(changelog: &Changelog) -> String {
    let mut out = String::new();
    let title = "Changelog";
    out.push_str(title);
    out.push('\n');
    out.push_str(&"=".repeat(title.len()));
    out.push_str("\n\n");

    for version in &changelog.versions {
        let heading = rst_version_heading(version);
        out.push_str(&heading);
        out.push('\n');
        out.push_str(&"-".repeat(heading.len()));
        out.push_str("\n\n");

        if !version.breaking_changes.is_empty() {
            let sub = "Breaking Changes";
            out.push_str(sub);
            out.push('\n');
            out.push_str(&"~".repeat(sub.len()));
            out.push_str("\n\n");
            for entry in &version.breaking_changes {
                out.push_str(&rst_entry(entry));
            }
            out.push('\n');
        }

        for section in &version.sections {
            out.push_str(&section.title);
            out.push('\n');
            out.push_str(&"~".repeat(section.title.len()));
            out.push_str("\n\n");
            for entry in &section.entries {
                out.push_str(&rst_entry(entry));
            }
            out.push('\n');
        }
    }

    out
}

fn rst_version_heading(version: &ChangelogVersion) -> String {
    let name = version.version.as_deref().unwrap_or("Unreleased");
    match &version.date {
        Some(date) => format!("[{}] - {}", name, date),
        None => format!("[{}]", name),
    }
}

fn rst_entry(entry: &ChangelogEntry) -> String {
    let scope = entry
        .scope
        .as_ref()
        .map(|s| format!("**{}:** ", s))
        .unwrap_or_default();
    format!("* {}{} (``{}``)\n", scope, entry.subject, entry.id)
}

// ─── AsciiDoc
// ─────────────────────────────────────────────────────────────────

/// Render the changelog as AsciiDoc.
pub fn render_asciidoc(changelog: &Changelog) -> String {
    let mut out = String::from("= Changelog\n\n");

    for version in &changelog.versions {
        out.push_str(&format!("== {}\n\n", asciidoc_version_heading(version)));

        if !version.breaking_changes.is_empty() {
            out.push_str("=== Breaking Changes\n\n");
            for entry in &version.breaking_changes {
                out.push_str(&asciidoc_entry(entry));
            }
            out.push('\n');
        }

        for section in &version.sections {
            out.push_str(&format!("=== {}\n\n", section.title));
            for entry in &section.entries {
                out.push_str(&asciidoc_entry(entry));
            }
            out.push('\n');
        }
    }

    out
}

fn asciidoc_version_heading(version: &ChangelogVersion) -> String {
    let name = version.version.as_deref().unwrap_or("Unreleased");
    match &version.date {
        Some(date) => format!("[{}] - {}", name, date),
        None => format!("[{}]", name),
    }
}

fn asciidoc_entry(entry: &ChangelogEntry) -> String {
    let scope = entry
        .scope
        .as_ref()
        .map(|s| format!("*{}:* ", s))
        .unwrap_or_default();
    format!("* {}{} (`{}`)\n", scope, entry.subject, entry.id)
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

    #[test]
    fn test_render_json_is_valid() {
        let cl = sample_changelog();
        let out = render_json(&cl).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed["versions"].is_array());
        assert_eq!(parsed["versions"][0]["version"], "v1.0.0");
    }

    #[test]
    fn test_render_html_structure() {
        let cl = sample_changelog();
        let out = render_html(&cl);
        assert!(out.contains("<!DOCTYPE html>"));
        assert!(out.contains("<h1>Changelog</h1>"));
        assert!(out.contains("v1.0.0"));
        assert!(out.contains("<h3>Features</h3>"));
        assert!(out.contains("<li>"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_render_rst_structure() {
        let cl = sample_changelog();
        let out = render_rst(&cl);
        assert!(out.contains("Changelog"));
        assert!(out.contains("========="));
        assert!(out.contains("[v1.0.0] - 2024-01-01"));
        assert!(out.contains("Features"));
        assert!(out.contains("* add login"));
        assert!(out.contains("(``abc12345``)"));
    }

    #[test]
    fn test_render_asciidoc_structure() {
        let cl = sample_changelog();
        let out = render_asciidoc(&cl);
        assert!(out.starts_with("= Changelog"));
        assert!(out.contains("== [v1.0.0] - 2024-01-01"));
        assert!(out.contains("=== Features"));
        assert!(out.contains("* add login"));
        assert!(out.contains("(`abc12345`)"));
    }
}
