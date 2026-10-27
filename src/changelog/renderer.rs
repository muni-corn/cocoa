//! Changelog output format renderers.

use crate::{
    changelog::{Changelog, ChangelogEntry, ChangelogError, ChangelogVersion, OutputFormat},
    config::ChangelogConfig,
};

mod builtin;
mod environment;
mod filters;
mod json;
mod template;

#[cfg(test)]
mod snapshot_tests;

use builtin::render_builtin;
use json::render_json;
use template::render_template;

/// Render a `Changelog` in the requested output format.
///
/// Template-based formats can read `config` (for example the `breaking` entry
/// of `config.sections`) as the `config` template variable.
pub fn render(
    changelog: &Changelog,
    config: &ChangelogConfig,
    format: &OutputFormat,
) -> Result<String, ChangelogError> {
    match format {
        OutputFormat::Markdown => render_builtin(changelog, config, &builtin::MARKDOWN),
        OutputFormat::Json => render_json(changelog),
        OutputFormat::Html => Ok(render_html(changelog)),
        OutputFormat::ReStructuredText => render_builtin(changelog, config, &builtin::RST),
        OutputFormat::AsciiDoc => render_builtin(changelog, config, &builtin::ASCIIDOC),
        OutputFormat::Template(path) => render_template(changelog, config, path),
    }
}

// HTML

/// Render the changelog as a standalone HTML document.
pub fn render_html(changelog: &Changelog) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>
        <html lang=\"en\">
        <head>
        <meta charset=\"UTF-8\">
        <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
        <title>Changelog</title>
        </head>
        <body>
        <h1>Changelog</h1>",
    );

    for version in &changelog.versions {
        out.push_str("<section>\n");
        out.push_str(&format!("<h2>{}</h2>\n", html_version_heading(version)));

        if !version.breaking_changes.is_empty() {
            out.push_str("<h3>Breaking changes</h3>\n<ul>\n");
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
    let name = html_escape(&version.version);
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
                version: "v1.0.0".to_string(),
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
                        title: "Bug fixes".to_string(),
                        entries: vec![sample_entry("def67890", "fix", "fix crash")],
                    },
                ],
            }],
        }
    }

    #[test]
    fn test_render_markdown_contains_version() {
        let cl = sample_changelog();
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::Markdown).unwrap();
        assert!(out.contains("# Changelog"));
        assert!(out.contains("## **v1.0.0**"));
        assert!(out.contains("### Features"));
        assert!(out.contains("### Bug fixes"));
        assert!(out.contains("add login"));
    }

    #[test]
    fn test_render_markdown_unreleased() {
        let cl = Changelog {
            versions: vec![ChangelogVersion {
                version: "Next version".to_string(),
                date: None,
                breaking_changes: vec![],
                sections: vec![ChangelogSection {
                    commit_type: "feat".to_string(),
                    title: "Features".to_string(),
                    entries: vec![sample_entry("abc12345", "feat", "new thing")],
                }],
            }],
        };
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::Markdown).unwrap();
        assert!(out.contains("## **Next version**"));
    }

    #[test]
    fn test_render_markdown_breaking_changes() {
        let mut cl = sample_changelog();
        cl.versions[0].breaking_changes = vec![{
            let mut e = sample_entry("brk12345", "feat", "api overhaul");
            e.breaking = true;
            e
        }];
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::Markdown).unwrap();
        assert!(out.contains("### **Breaking changes**"));
        assert!(out.contains("api overhaul"));
    }

    #[test]
    fn test_render_markdown_scope() {
        let mut cl = sample_changelog();
        cl.versions[0].sections[0].entries[0].scope = Some("auth".to_string());
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::Markdown).unwrap();
        assert!(out.contains("**auth:**"));
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
        let out = render(
            &cl,
            &ChangelogConfig::default(),
            &OutputFormat::ReStructuredText,
        )
        .unwrap();
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
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::AsciiDoc).unwrap();
        assert!(out.starts_with("= Changelog"));
        assert!(out.contains("== [v1.0.0] - 2024-01-01"));
        assert!(out.contains("=== Features"));
        assert!(out.contains("* add login"));
        assert!(out.contains("(`abc12345`)"));
    }

    // --- render() dispatch function ---

    #[test]
    fn test_render_dispatch_markdown() {
        let cl = sample_changelog();
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::Markdown).unwrap();
        assert!(out.contains("# Changelog"));
    }

    #[test]
    fn test_render_dispatch_json() {
        let cl = sample_changelog();
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::Json).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["versions"].is_array());
    }

    #[test]
    fn test_render_dispatch_html() {
        let cl = sample_changelog();
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::Html).unwrap();
        assert!(out.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_render_dispatch_rst() {
        let cl = sample_changelog();
        let out = render(
            &cl,
            &ChangelogConfig::default(),
            &OutputFormat::ReStructuredText,
        )
        .unwrap();
        assert!(out.contains("Changelog"));
    }

    #[test]
    fn test_render_dispatch_asciidoc() {
        let cl = sample_changelog();
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::AsciiDoc).unwrap();
        assert!(out.starts_with("= Changelog"));
    }

    // --- unreleased versions (no date) ---

    fn unreleased_changelog() -> Changelog {
        Changelog {
            versions: vec![ChangelogVersion {
                version: "Next version".to_string(),
                date: None,
                breaking_changes: vec![{
                    let mut e = sample_entry("brk00001", "feat", "renamed endpoint");
                    e.breaking = true;
                    e
                }],
                sections: vec![],
            }],
        }
    }

    #[test]
    fn test_render_html_unreleased_no_date() {
        let cl = unreleased_changelog();
        let out = render_html(&cl);
        // should contain "Next version" as the heading text
        assert!(out.contains("Next version"));
    }

    #[test]
    fn test_render_html_breaking_changes() {
        let cl = unreleased_changelog();
        let out = render_html(&cl);
        assert!(out.contains("<h3>Breaking changes</h3>"));
        assert!(out.contains("renamed endpoint"));
    }

    #[test]
    fn test_render_html_entry_with_scope() {
        let mut cl = sample_changelog();
        cl.versions[0].sections[0].entries[0].scope = Some("api & ui".to_string());
        let out = render_html(&cl);
        // scope is HTML-escaped
        assert!(out.contains("api &amp; ui"));
    }

    #[test]
    fn test_render_rst_unreleased_no_date() {
        let cl = unreleased_changelog();
        let out = render(
            &cl,
            &ChangelogConfig::default(),
            &OutputFormat::ReStructuredText,
        )
        .unwrap();
        assert!(out.contains("[Next version]"));
    }

    #[test]
    fn test_render_rst_breaking_changes() {
        let cl = unreleased_changelog();
        let out = render(
            &cl,
            &ChangelogConfig::default(),
            &OutputFormat::ReStructuredText,
        )
        .unwrap();
        assert!(out.contains("Breaking changes"));
        assert!(out.contains("renamed endpoint"));
    }

    #[test]
    fn test_render_rst_entry_with_scope() {
        let mut cl = sample_changelog();
        cl.versions[0].sections[0].entries[0].scope = Some("core".to_string());
        let out = render(
            &cl,
            &ChangelogConfig::default(),
            &OutputFormat::ReStructuredText,
        )
        .unwrap();
        assert!(out.contains("* **core:** add login"));
    }

    #[test]
    fn test_render_asciidoc_unreleased_no_date() {
        let cl = unreleased_changelog();
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::AsciiDoc).unwrap();
        assert!(out.contains("[Next version]"));
    }

    #[test]
    fn test_render_asciidoc_breaking_changes() {
        let cl = unreleased_changelog();
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::AsciiDoc).unwrap();
        assert!(out.contains("=== Breaking changes"));
        assert!(out.contains("renamed endpoint"));
    }

    #[test]
    fn test_render_asciidoc_entry_with_scope() {
        let mut cl = sample_changelog();
        cl.versions[0].sections[0].entries[0].scope = Some("core".to_string());
        let out = render(&cl, &ChangelogConfig::default(), &OutputFormat::AsciiDoc).unwrap();
        assert!(out.contains("* *core:* add login"));
    }
}
