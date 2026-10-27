//! Shared minijinja environment for all template-based changelog renderers.

use minijinja::{Environment, syntax::SyntaxConfig};

use super::filters;

/// Builds the environment used for user-supplied templates.
///
/// Autoescaping follows minijinja's default rules, which key off the template
/// name: names ending in `.html`, `.htm` or `.xml` (optionally followed by
/// `.j2`, `.jinja` or `.jinja2`) are escaped and everything else is not.
pub fn build_environment<'source>() -> Environment<'source> {
    environment(false)
}

/// Builds the environment used for the built-in templates.
///
/// Same as [`build_environment`], but block tags on their own line leave no
/// blank line behind, which keeps the built-in templates readable. User
/// templates don't get this so that their existing whitespace is preserved.
pub fn build_builtin_environment<'source>() -> Environment<'source> {
    environment(true)
}

fn environment<'source>(tidy_blocks: bool) -> Environment<'source> {
    let mut env = Environment::new();
    let syntax = SyntaxConfig::builder()
        // changelogs are files, so a trailing newline should survive rendering
        .keep_trailing_newline(true)
        .trim_blocks(tidy_blocks)
        .lstrip_blocks(tidy_blocks)
        .build()
        .expect("default delimiters are always valid");
    env.set_syntax(syntax);
    env.add_filter("heading", filters::heading);
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_keeps_trailing_newline() {
        let mut env = build_environment();
        env.add_template("t.txt", "hello\n").unwrap();
        let out = env.get_template("t.txt").unwrap().render(()).unwrap();
        assert_eq!(out, "hello\n");
    }

    #[test]
    fn test_builtin_environment_trims_block_lines() {
        let src = "{% for n in [1, 2] %}\n{{ n }}\n{% endfor %}\n";
        let mut user = build_environment();
        user.add_template("t.txt", src).unwrap();
        let mut builtin = build_builtin_environment();
        builtin.add_template("t.txt", src).unwrap();
        assert_eq!(
            user.get_template("t.txt").unwrap().render(()).unwrap(),
            "\n1\n\n2\n\n"
        );
        assert_eq!(
            builtin.get_template("t.txt").unwrap().render(()).unwrap(),
            "1\n2\n"
        );
    }

    #[test]
    fn test_environment_registers_heading_filter() {
        let mut env = build_environment();
        env.add_template("t.rst", "{{ title|heading('~') }}")
            .unwrap();
        let ctx = minijinja::context! { title => "Hi" };
        let out = env.get_template("t.rst").unwrap().render(&ctx).unwrap();
        assert_eq!(out, "Hi\n~~");
    }

    #[test]
    fn test_environment_escapes_html_names_only() {
        let mut env = build_environment();
        env.add_template("t.html", "{{ v }}").unwrap();
        env.add_template("t.md", "{{ v }}").unwrap();
        let ctx = minijinja::context! { v => "<b>" };
        assert_eq!(
            env.get_template("t.html").unwrap().render(&ctx).unwrap(),
            "&lt;b&gt;"
        );
        assert_eq!(
            env.get_template("t.md").unwrap().render(&ctx).unwrap(),
            "<b>"
        );
    }
}
