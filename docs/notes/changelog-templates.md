# Changelog templates

Notes on how cocoa renders changelogs, for future maintainers and contributors.

## Overview

Every changelog format except JSON is rendered from a Jinja template through minijinja. JSON stays a
`serde_json` serializer because escaping is better left to serde than done by hand in a template.

| Format           | Template                                       | Heading underlines |
| ---------------- | ---------------------------------------------- | ------------------ |
| Markdown         | `src/changelog/templates/changelog.md.jinja`   | none               |
| HTML             | `src/changelog/templates/changelog.html.jinja` | none               |
| reStructuredText | `src/changelog/templates/changelog.rst.jinja`  | `heading` filter   |
| AsciiDoc         | `src/changelog/templates/changelog.adoc.jinja` | none               |

The built-in templates are embedded with `include_str!` in `src/changelog/renderer/builtin.rs`. To
add a format, add a template file, add a `BuiltinTemplate` const next to the others, and dispatch to
it from `render()` in `src/changelog/renderer.rs`.

## Module layout

All of this lives under `src/changelog/renderer/`:

- `builtin.rs` holds the embedded templates and `render_builtin`.
- `template.rs` holds `render_template` (user templates) and the shared `render_source` helper.
- `environment.rs` builds the minijinja environments.
- `filters.rs` holds custom filters.
- `json.rs` holds the JSON renderer.

## Template context

Every template, built-in or user-supplied, receives two variables:

- `changelog`: the serialized `Changelog`, shaped as `changelog.versions[]`, where each version has
  `version`, `date` (null when unreleased), `breaking_changes[]` and `sections[]`. Each section has
  `commit_type`, `title` and `entries[]`. Each entry has `id`, `commit_type`, `scope`, `breaking`,
  `subject`, `body`, `author`, `timestamp` and `date`.
- `config`: the serialized `[changelog]` configuration, for example `config.sections` and
  `config.date_format`.

Breaking entries appear twice: once in `breaking_changes` and once in their own commit type section.

### Breaking changes heading

The heading for breaking changes comes from the special `breaking` key of `[changelog.sections]`. The
built-in templates read it with:

```jinja
{{ config.sections.breaking|default("Breaking changes") }}
```

The `default` filter covers configs whose `sections` map has no `breaking` key, which includes the
built-in defaults.

## Environments

There are two environments, both built in `environment.rs`:

- `build_environment()` is used for user templates (`--format template:<path>`). It keeps the
  trailing newline of the template and otherwise leaves whitespace alone, so existing user templates
  render as they always have.
- `build_builtin_environment()` is used for the built-in templates. It adds `trim_blocks` and
  `lstrip_blocks`, so a block tag on its own line leaves no blank line behind. This keeps the
  templates readable.

Both register the custom filters.

### Whitespace gotcha

With `trim_blocks` on, the newline after a block tag is removed. If a block tag ends a line whose
newline you want to keep (for example an inline `{% if %}` in a heading), close it with `+%}`:

```jinja
== [{{ version.version }}]{% if version.date %} - {{ version.date }}{% endif +%}
```

## Autoescaping

Templates are registered under their file name, and minijinja decides autoescaping from that name.
Names ending in `.html`, `.htm` or `.xml` are escaped, with an optional trailing `.j2`, `.jinja` or
`.jinja2` ignored. Everything else is not escaped.

This means:

- `changelog.html.jinja` is escaped, so the HTML template doesn't need `|e` anywhere.
- A user template named `foo.html` is escaped too. A user template named `foo.md` is not.

minijinja's HTML escaper turns `/` into `&#x2f;` and `'` into `&#x27;`. Both are valid HTML.

## Filters

### `heading(fill)`

Returns the text, a newline, and a line of `fill` repeated once per character. The fill must be a
single character. It counts characters, not bytes, so non-ASCII titles get matching underlines.

```jinja
{{ section.title|heading("~") }}
```

Note that docutils measures titles by display width, so wide characters such as emoji count as two
columns and may still produce an underline that's too short. Switching the filter to the
`unicode-width` crate would fix that if it becomes a problem.

## Testing

Output is pinned by `insta` snapshots in `src/changelog/renderer/snapshot_tests.rs`, which render a
fixture covering dates, scopes, breaking changes, HTML-special characters and a non-ASCII title.
Review snapshot diffs carefully. They are the contract for each format's exact output.
