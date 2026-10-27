//! Snapshot tests that pin the exact output of every changelog format.

use crate::{
    changelog::{
        Changelog, ChangelogEntry, ChangelogSection, ChangelogVersion, OutputFormat,
        renderer::render,
    },
    config::ChangelogConfig,
};

fn entry(id: &str, scope: Option<&str>, breaking: bool, subject: &str) -> ChangelogEntry {
    ChangelogEntry {
        id: id.to_string(),
        commit_type: "feat".to_string(),
        scope: scope.map(str::to_string),
        breaking,
        subject: subject.to_string(),
        body: None,
        author: "Author".to_string(),
        timestamp: 1000,
        date: "2024-01-01".to_string(),
    }
}

fn section(commit_type: &str, title: &str, entries: Vec<ChangelogEntry>) -> ChangelogSection {
    ChangelogSection {
        commit_type: commit_type.to_string(),
        title: title.to_string(),
        entries,
    }
}

/// Builds a changelog that exercises dates, scopes, breaking changes,
/// html-special characters, slashes, and a non-ascii section title.
fn fixture() -> Changelog {
    let breaking = entry(
        "brk12345",
        Some("api"),
        true,
        "overhaul <routes> and \"auth\"",
    );
    Changelog {
        versions: vec![
            ChangelogVersion {
                version: "Next version".to_string(),
                date: None,
                breaking_changes: vec![],
                sections: vec![section("fix", "Bug fixes", vec![entry(
                    "fff00000",
                    None,
                    false,
                    "handle empty input",
                )])],
            },
            ChangelogVersion {
                version: "v1.0.0".to_string(),
                date: Some("2024-01-01".to_string()),
                breaking_changes: vec![breaking.clone()],
                sections: vec![
                    section("feat", "Features", vec![
                        breaking,
                        entry("abc12345", Some("ui/forms"), false, "add login & signup"),
                        entry("abc67890", None, false, "it's a 'quoted' subject"),
                    ]),
                    section("docs", "\u{1f4da} Documentation", vec![entry(
                        "ddd11111",
                        Some("readme"),
                        false,
                        "explain usage",
                    )]),
                ],
            },
        ],
    }
}

fn rendered(format: OutputFormat) -> String {
    render(&fixture(), &ChangelogConfig::default(), &format).expect("fixture should render")
}

#[test]
fn snapshot_markdown() {
    insta::assert_snapshot!(rendered(OutputFormat::Markdown));
}

#[test]
fn snapshot_markdown_custom_breaking_header() {
    let config = ChangelogConfig {
        breaking_changes_header: "\u{1f4a5} Breaking changes".to_string(),
        ..ChangelogConfig::default()
    };
    let out = render(&fixture(), &config, &OutputFormat::Markdown).expect("fixture should render");
    insta::assert_snapshot!(out);
}

#[test]
fn snapshot_json() {
    insta::assert_snapshot!(rendered(OutputFormat::Json));
}

#[test]
fn snapshot_html() {
    insta::assert_snapshot!(rendered(OutputFormat::Html));
}

#[test]
fn snapshot_rst() {
    insta::assert_snapshot!(rendered(OutputFormat::ReStructuredText));
}

#[test]
fn snapshot_asciidoc() {
    insta::assert_snapshot!(rendered(OutputFormat::AsciiDoc));
}
