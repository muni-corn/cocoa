//! Commitlint configuration migration.
//!
//! Parses `.commitlintrc.json`, `.commitlintrc.yaml`, `.commitlintrc.yml`, and
//! `.commitlintrc` (JSON) files and converts them to a cocoa [`Config`].
//!
//! JavaScript and TypeScript variants (`commitlint.config.js`, etc.) are
//! listed for file detection but cannot be statically parsed; they will
//! produce a [`MigrateError::Parse`] with an explanatory message.

use std::{collections::HashSet, path::Path};

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::{
    config::{CommitConfig, CommitRules, Config, RuleLevel},
    migrate::MigrateError,
};

/// All file names this migrator recognises (checked in order).
pub const SUPPORTED_FILES: &[&str] = &[
    ".commitlintrc",
    ".commitlintrc.json",
    ".commitlintrc.yaml",
    ".commitlintrc.yml",
    "commitlint.config.js",
    "commitlint.config.cjs",
    "commitlint.config.mjs",
    "commitlint.config.ts",
];

/// Loose representation of a commitlint config for deserialization.
#[derive(Debug, Default, Deserialize)]
struct CommitlintConfig {
    #[serde(default)]
    rules: std::collections::HashMap<String, JsonValue>,
}

/// Parse a commitlint configuration file and convert it to a cocoa [`Config`].
///
/// Supported formats: JSON and YAML.
/// JavaScript/TypeScript files are rejected with a descriptive error.
pub fn parse(path: &Path) -> Result<Config, MigrateError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // reject JS/TS — we can't statically evaluate these
    if matches!(ext.as_str(), "js" | "cjs" | "mjs" | "ts") {
        return Err(MigrateError::Parse(format!(
            "'{}' is a JavaScript/TypeScript file — cocoa cannot statically parse it.\n\
             export your config as JSON (.commitlintrc.json) or YAML (.commitlintrc.yaml) \
             and run `cocoa migrate` again.",
            path.display()
        )));
    }

    let content = std::fs::read_to_string(path).map_err(|e| MigrateError::Read {
        path: path.display().to_string(),
        source: e,
    })?;

    let raw: CommitlintConfig = if matches!(ext.as_str(), "yaml" | "yml") {
        serde_yml::from_str(&content).map_err(|e| {
            MigrateError::Parse(format!("YAML parse error in '{}': {}", path.display(), e))
        })?
    } else if ext == "json" || file_name == ".commitlintrc" {
        // .commitlintrc with no extension is typically JSON
        serde_json::from_str(&content).map_err(|e| {
            MigrateError::Parse(format!("JSON parse error in '{}': {}", path.display(), e))
        })?
    } else {
        // fall back to JSON
        serde_json::from_str(&content).map_err(|e| {
            MigrateError::Parse(format!("parse error in '{}': {}", path.display(), e))
        })?
    };

    convert(raw)
}

/// Convert a parsed [`CommitlintConfig`] into a cocoa [`Config`].
fn convert(raw: CommitlintConfig) -> Result<Config, MigrateError> {
    let mut types: HashSet<String> = HashSet::new();
    let mut scopes: Option<HashSet<String>> = None;
    let mut deny_subject_length: Option<usize> = None;
    let mut deny_body_length: Option<usize> = None;

    // extract values from the rules map
    for (rule, value) in &raw.rules {
        match rule.as_str() {
            "type-enum" => {
                // format: [severity, "always"|"never", [...types]]
                if let Some(arr) = value.as_array()
                    && let Some(list) = arr.get(2).and_then(|v| v.as_array())
                {
                    for t in list {
                        if let Some(s) = t.as_str() {
                            types.insert(s.to_string());
                        }
                    }
                }
            }
            "scope-enum" => {
                // format: [severity, "always"|"never", [...scopes]]
                if let Some(arr) = value.as_array()
                    && let Some(list) = arr.get(2).and_then(|v| v.as_array())
                {
                    let mut scope_set = HashSet::new();
                    for s in list {
                        if let Some(name) = s.as_str() {
                            scope_set.insert(name.to_string());
                        }
                    }
                    if !scope_set.is_empty() {
                        scopes = Some(scope_set);
                    }
                }
            }
            "header-max-length" => {
                // format: [severity, "always", maxLen]
                if let Some(arr) = value.as_array()
                    && let Some(n) = arr.get(2).and_then(|v| v.as_u64())
                {
                    deny_subject_length = Some(n as usize);
                }
            }
            "body-max-line-length" => {
                // format: [severity, "always", maxLen]
                if let Some(arr) = value.as_array()
                    && let Some(n) = arr.get(2).and_then(|v| v.as_u64())
                {
                    deny_body_length = Some(n as usize);
                }
            }
            // other rules are not mapped — cocoa has its own rule model
            _ => {}
        }
    }

    // use cocoa defaults if no types were found in the source config
    let commit_types = if types.is_empty() {
        CommitConfig::default().types
    } else {
        types
    };

    let commit_config = CommitConfig {
        types: commit_types,
        scopes,
        rules: CommitRules {
            enabled: true,
            deny: RuleLevel {
                subject_length: deny_subject_length,
                body_length: deny_body_length,
                ..Default::default()
            },
            ..Default::default()
        },
    };

    Ok(Config {
        commit: commit_config,
        ai: None,
        changelog: None,
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
    fn test_parse_json_type_enum() {
        let content = r#"{
            "rules": {
                "type-enum": [2, "always", ["feat", "fix", "chore"]],
                "header-max-length": [2, "always", 72]
            }
        }"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        assert!(config.commit.types.contains("feat"));
        assert!(config.commit.types.contains("fix"));
        assert!(config.commit.types.contains("chore"));
        assert_eq!(config.commit.rules.deny.subject_length, Some(72));
    }

    #[test]
    fn test_parse_json_scope_enum() {
        let content = r#"{
            "rules": {
                "type-enum": [2, "always", ["feat"]],
                "scope-enum": [2, "always", ["api", "auth"]]
            }
        }"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        let scopes = config.commit.scopes.unwrap();
        assert!(scopes.contains("api"));
        assert!(scopes.contains("auth"));
    }

    #[test]
    fn test_parse_yaml_type_enum() {
        let content = "rules:\n  type-enum:\n    - 2\n    - always\n    - [feat, fix, docs]\n  header-max-length:\n    - 2\n    - always\n    - 100\n";
        let f = write_temp(content, ".yaml");
        let config = parse(f.path()).unwrap();
        assert!(config.commit.types.contains("feat"));
        assert_eq!(config.commit.rules.deny.subject_length, Some(100));
    }

    #[test]
    fn test_parse_js_returns_error() {
        let content = "module.exports = { rules: {} };";
        let f = write_temp(content, ".js");
        let err = parse(f.path()).unwrap_err();
        assert!(matches!(err, MigrateError::Parse(_)));
    }

    #[test]
    fn test_empty_config_uses_defaults() {
        let content = r#"{"rules": {}}"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        // should fall back to cocoa defaults
        assert!(!config.commit.types.is_empty());
    }
}
