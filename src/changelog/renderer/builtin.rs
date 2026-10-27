//! Built-in changelog templates that ship inside the binary.

use super::{environment::build_builtin_environment, template::render_source};
use crate::{
    changelog::{Changelog, ChangelogError},
    config::ChangelogConfig,
};

/// A template embedded at compile time.
///
/// The name carries the file extension because it decides autoescaping.
pub struct BuiltinTemplate {
    pub name: &'static str,
    pub source: &'static str,
}

/// The GitHub-flavored Markdown template.
pub const MARKDOWN: BuiltinTemplate = BuiltinTemplate {
    name: "changelog.md.jinja",
    source: include_str!("../templates/changelog.md.jinja"),
};

/// The AsciiDoc template.
pub const ASCIIDOC: BuiltinTemplate = BuiltinTemplate {
    name: "changelog.adoc.jinja",
    source: include_str!("../templates/changelog.adoc.jinja"),
};

/// Renders the changelog with a built-in template.
///
/// # Errors
///
/// Returns [`ChangelogError::Template`] if the template fails to render.
pub fn render_builtin(
    changelog: &Changelog,
    config: &ChangelogConfig,
    template: &BuiltinTemplate,
) -> Result<String, ChangelogError> {
    render_source(
        build_builtin_environment(),
        template.name,
        template.source,
        changelog,
        config,
    )
}
