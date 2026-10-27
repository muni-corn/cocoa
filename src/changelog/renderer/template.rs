//! User-supplied Jinja template changelog renderer.

use std::path::Path;

use minijinja::value::Serde;

use super::environment::build_environment;
use crate::changelog::{Changelog, ChangelogError};

/// Renders using a Jinja2-compatible template loaded from `template_path`.
///
/// The template receives a single variable `changelog` containing the full
/// serialized changelog data. It is registered under its file name, so the
/// extension decides whether output is autoescaped (see [`build_environment`]).
///
/// # Errors
///
/// Returns [`ChangelogError::Template`] if the file can't be read, parsed or
/// rendered.
pub fn render_template(
    changelog: &Changelog,
    template_path: &str,
) -> Result<String, ChangelogError> {
    let template_content = std::fs::read_to_string(template_path).map_err(|e| {
        ChangelogError::Template(format!(
            "failed to read template '{}': {}",
            template_path, e
        ))
    })?;

    let name = Path::new(template_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("changelog");

    let mut env = build_environment();
    env.add_template(name, &template_content)
        .map_err(|e| ChangelogError::Template(format!("failed to parse template: {}", e)))?;

    let tmpl = env
        .get_template(name)
        .map_err(|e| ChangelogError::Template(format!("failed to load template: {}", e)))?;

    let ctx = minijinja::Value::from(Serde(changelog));
    tmpl.render(minijinja::context! { changelog => ctx })
        .map_err(|e| ChangelogError::Template(format!("failed to render template: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::ChangelogVersion;

    fn changelog_with_version(version: &str) -> Changelog {
        Changelog {
            versions: vec![ChangelogVersion {
                version: version.to_string(),
                date: None,
                breaking_changes: vec![],
                sections: vec![],
            }],
        }
    }

    fn write_template(dir: &tempfile::TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn test_render_template_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_template(
            &dir,
            "changelog.jinja",
            "{% for v in changelog.versions %}{{ v.version }}{% endfor %}",
        );
        let out = render_template(&changelog_with_version("v1.0.0"), &path).unwrap();
        assert_eq!(out, "v1.0.0");
    }

    #[test]
    fn test_render_template_keeps_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_template(
            &dir,
            "changelog.md",
            "{{ changelog.versions[0].version }}\n",
        );
        let out = render_template(&changelog_with_version("v1.0.0"), &path).unwrap();
        assert_eq!(out, "v1.0.0\n");
    }

    #[test]
    fn test_render_template_escapes_html_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_template(
            &dir,
            "changelog.html",
            "{{ changelog.versions[0].version }}",
        );
        let out = render_template(&changelog_with_version("<v1>"), &path).unwrap();
        assert_eq!(out, "&lt;v1&gt;");
    }

    #[test]
    fn test_render_template_does_not_escape_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_template(&dir, "changelog.md", "{{ changelog.versions[0].version }}");
        let out = render_template(&changelog_with_version("<v1>"), &path).unwrap();
        assert_eq!(out, "<v1>");
    }

    #[test]
    fn test_render_template_missing_file() {
        let result = render_template(&changelog_with_version("v1"), "/nonexistent/template.jinja");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("template"));
    }

    #[test]
    fn test_render_template_syntax_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_template(&dir, "changelog.jinja", "{% for %}");
        let err = render_template(&changelog_with_version("v1"), &path)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to parse template"));
    }
}
