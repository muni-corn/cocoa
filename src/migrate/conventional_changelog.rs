//! Conventional-changelog configuration migration.
//!
//! Parses `changelog.config.js`, `.changelog.config.js`, and related files,
//! extracting commit type definitions and section headings.
//!
//! Since these are JavaScript files, parsing is done via regex heuristics on
//! common patterns. A `types: [...]` array with `type` and `section` fields is
//! expected. JSON variants are also supported.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use regex::Regex;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::{
    config::{ChangelogConfig, CommitConfig, Config},
    migrate::MigrateError,
};

/// All file names this migrator recognises (checked in order).
pub const SUPPORTED_FILES: &[&str] = &[
    "changelog.config.js",
    "changelog.config.cjs",
    "changelog.config.mjs",
    ".changelog.config.js",
    ".changelog.config.cjs",
    ".changelog.config.mjs",
    "conventional-changelog.config.js",
    "conventional-changelog.config.cjs",
    "changelog.config.json",
    ".changelog.config.json",
];

/// A single type entry as parsed from a conventional-changelog config.
#[derive(Debug, Deserialize)]
struct TypeEntry {
    #[serde(rename = "type")]
    commit_type: String,
    section: Option<String>,
    #[serde(default)]
    hidden: bool,
}

/// Parse a conventional-changelog configuration file and convert it to a
/// cocoa [`Config`].
///
/// JSON files are parsed directly. JavaScript files are parsed using regex
/// heuristics for the standard `types: [...]` pattern.
pub fn parse(path: &Path) -> Result<Config, MigrateError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = std::fs::read_to_string(path).map_err(|e| MigrateError::Read {
        path: path.display().to_string(),
        source: e,
    })?;

    let entries = if ext == "json" {
        parse_json_entries(&content, path)?
    } else {
        // js/cjs/mjs: use regex heuristics
        parse_js_entries(&content, path)?
    };

    convert(entries)
}

/// Parse type entries from a JSON config file.
fn parse_json_entries(content: &str, path: &Path) -> Result<Vec<TypeEntry>, MigrateError> {
    let root: JsonValue = serde_json::from_str(content).map_err(|e| {
        MigrateError::Parse(format!("JSON parse error in '{}': {}", path.display(), e))
    })?;

    let types_arr = root
        .get("types")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            MigrateError::Parse(format!("no 'types' array found in '{}'", path.display()))
        })?;

    let entries: Vec<TypeEntry> = types_arr
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    Ok(entries)
}

/// Parse type entries from a JavaScript config file using regex heuristics.
///
/// Looks for patterns like:
/// ```js
/// { type: 'feat', section: 'Features', hidden: false },
/// { type: 'chore', hidden: true },
/// ```
fn parse_js_entries(content: &str, path: &Path) -> Result<Vec<TypeEntry>, MigrateError> {
    // match individual type objects: { type: '...', section: '...', hidden: ... }
    let type_re = Regex::new(r#"\{\s*type:\s*['"](\w+)['"]([^}]*)\}"#).unwrap();
    let section_re = Regex::new(r#"section:\s*['"]([^'"]+)['"]"#).unwrap();
    let hidden_re = Regex::new(r"hidden:\s*(true|false)").unwrap();

    let mut entries = Vec::new();

    for cap in type_re.captures_iter(content) {
        let commit_type = cap[1].to_string();
        let rest = &cap[2];

        let section = section_re.captures(rest).map(|c| c[1].to_string());

        let hidden = hidden_re
            .captures(rest)
            .map(|c| &c[1] == "true")
            .unwrap_or(false);

        entries.push(TypeEntry {
            commit_type,
            section,
            hidden,
        });
    }

    if entries.is_empty() {
        return Err(MigrateError::Parse(format!(
            "no commit type definitions found in '{}'\n\
             expected a pattern like: {{ type: 'feat', section: 'Features' }}",
            path.display()
        )));
    }

    Ok(entries)
}

/// Convert parsed type entries into a cocoa [`Config`].
fn convert(entries: Vec<TypeEntry>) -> Result<Config, MigrateError> {
    let mut types: HashSet<String> = HashSet::new();
    let mut sections: HashMap<String, String> = HashMap::new();

    for entry in entries {
        // include all types (even hidden ones) in commit.types so linting works
        types.insert(entry.commit_type.clone());

        // only map non-hidden types with sections to changelog sections
        if !entry.hidden
            && let Some(section_title) = entry.section
        {
            sections.insert(entry.commit_type, section_title);
        }
    }

    let commit_config = CommitConfig {
        types,
        ..Default::default()
    };

    let changelog_config = if sections.is_empty() {
        None
    } else {
        Some(ChangelogConfig {
            sections,
            ..Default::default()
        })
    };

    Ok(Config {
        commit: commit_config,
        ai: None,
        changelog: changelog_config,
        version: None,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn write_temp(content: &str, suffix: &str) -> NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_parse_js_types() {
        let content = r#"
module.exports = {
  types: [
    { type: 'feat', section: 'Features', hidden: false },
    { type: 'fix', section: 'Bug Fixes', hidden: false },
    { type: 'chore', hidden: true },
  ]
};
"#;
        let f = write_temp(content, ".js");
        let config = parse(f.path()).unwrap();
        assert!(config.commit.types.contains("feat"));
        assert!(config.commit.types.contains("fix"));
        assert!(config.commit.types.contains("chore"));
        let sections = config.changelog.unwrap().sections;
        assert_eq!(sections["feat"], "Features");
        assert_eq!(sections["fix"], "Bug Fixes");
        // hidden types have no section
        assert!(!sections.contains_key("chore"));
    }

    #[test]
    fn test_parse_json_types() {
        let content = r#"{
            "types": [
                {"type": "feat", "section": "Features", "hidden": false},
                {"type": "fix", "section": "Bug Fixes", "hidden": false}
            ]
        }"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        assert!(config.commit.types.contains("feat"));
        let sections = config.changelog.unwrap().sections;
        assert_eq!(sections["feat"], "Features");
    }

    #[test]
    fn test_parse_js_no_types_returns_error() {
        let content = "module.exports = { preset: 'angular' };";
        let f = write_temp(content, ".js");
        let err = parse(f.path()).unwrap_err();
        assert!(matches!(err, MigrateError::Parse(_)));
    }
}
