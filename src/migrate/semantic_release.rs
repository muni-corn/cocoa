//! Semantic-release configuration migration.
//!
//! Parses `.releaserc`, `.releaserc.json`, `.releaserc.yaml`, and related
//! files, extracting tag format and commit analyzer configuration.
//!
//! The `tagFormat` field maps to `version.tag_prefix`. The commit-analyzer
//! plugin's `releaseRules` are used to determine which commit types the
//! project uses; standard angular/conventional-commits presets are recognised
//! without explicit `releaseRules`.

use std::{collections::HashSet, path::Path};

use regex::Regex;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::{
    config::{CommitConfig, Config, VersionConfig},
    migrate::MigrateError,
};

/// All file names this migrator recognises (checked in order).
pub const SUPPORTED_FILES: &[&str] = &[
    ".releaserc",
    ".releaserc.json",
    ".releaserc.yaml",
    ".releaserc.yml",
    ".releaserc.toml",
    "release.config.js",
    "release.config.cjs",
    "release.config.mjs",
];

/// Parsed representation of a semantic-release config.
///
/// Only the fields relevant to migration are captured.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SemanticReleaseConfig {
    /// Tag format string, e.g. `"v${version}"` or `"${version}"`.
    tag_format: Option<String>,
    /// Plugins array (may contain strings or [plugin, options] tuples).
    plugins: Vec<JsonValue>,
}

/// Parse a semantic-release configuration file and convert it to a cocoa
/// [`Config`].
///
/// Supported formats: JSON, YAML, and TOML. JavaScript files are parsed using
/// regex heuristics.
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

    let content = std::fs::read_to_string(path).map_err(|e| MigrateError::Read {
        path: path.display().to_string(),
        source: e,
    })?;

    let raw: SemanticReleaseConfig = match ext.as_str() {
        "yaml" | "yml" => serde_yml::from_str(&content).map_err(|e| {
            MigrateError::Parse(format!("YAML parse error in '{}': {}", path.display(), e))
        })?,
        "toml" => toml::from_str(&content).map_err(|e| {
            MigrateError::Parse(format!("TOML parse error in '{}': {}", path.display(), e))
        })?,
        "js" | "cjs" | "mjs" => parse_js(&content)?,
        _ if file_name == ".releaserc" => {
            // bare .releaserc is JSON by convention
            serde_json::from_str(&content).map_err(|e| {
                MigrateError::Parse(format!("JSON parse error in '{}': {}", path.display(), e))
            })?
        }
        _ => serde_json::from_str(&content).map_err(|e| {
            MigrateError::Parse(format!("JSON parse error in '{}': {}", path.display(), e))
        })?,
    };

    convert(raw)
}

/// Parse a JavaScript semantic-release config using regex heuristics.
///
/// Extracts `tagFormat` from common assignment patterns. Plugin arrays are
/// not parsed — the JS AST would be required for reliable extraction.
fn parse_js(content: &str) -> Result<SemanticReleaseConfig, MigrateError> {
    let mut cfg = SemanticReleaseConfig::default();

    // match: tagFormat: "v${version}" or tagFormat: '${version}'
    let tag_re = Regex::new(r#"tagFormat:\s*['"]([^'"]+)['"]"#).unwrap();
    if let Some(cap) = tag_re.captures(content) {
        cfg.tag_format = Some(cap[1].to_string());
    }

    Ok(cfg)
}

/// Convert a [`SemanticReleaseConfig`] into a cocoa [`Config`].
fn convert(raw: SemanticReleaseConfig) -> Result<Config, MigrateError> {
    // derive tag prefix from tagFormat: "v${version}" → "v", "${version}" → ""
    let tag_prefix = raw
        .tag_format
        .as_deref()
        .map(extract_tag_prefix)
        .unwrap_or_else(|| "v".to_string());

    // collect commit types from commit-analyzer releaseRules if present;
    // fall back to the standard angular/conventional-commits preset list
    let types = extract_types_from_plugins(&raw.plugins).unwrap_or_else(default_angular_types);

    let commit_config = CommitConfig {
        types,
        ..Default::default()
    };

    let version_config = Some(VersionConfig {
        tag_prefix,
        ..Default::default()
    });

    Ok(Config {
        commit: commit_config,
        ai: None,
        changelog: None,
        version: version_config,
    })
}

/// Extract the literal prefix from a semantic-release tagFormat string.
///
/// `"v${version}"` → `"v"`, `"${version}"` → `""`, `"v"` → `"v"`
fn extract_tag_prefix(tag_format: &str) -> String {
    if let Some(idx) = tag_format.find("${version}") {
        tag_format[..idx].to_string()
    } else if let Some(idx) = tag_format.find("{version}") {
        tag_format[..idx].to_string()
    } else {
        // treat the whole string as the prefix (unusual but valid)
        tag_format.to_string()
    }
}

/// Attempt to extract commit types from a plugins array.
///
/// Looks for `@semantic-release/commit-analyzer` plugin options and reads
/// `releaseRules[].type` entries. Returns `None` if no types can be
/// determined from the plugins.
fn extract_types_from_plugins(plugins: &[JsonValue]) -> Option<HashSet<String>> {
    for plugin in plugins {
        // plugins can be a string or [name, options] array
        let options = match plugin {
            JsonValue::Array(arr) => {
                let name = arr.first().and_then(|v| v.as_str()).unwrap_or("");
                if !name.contains("commit-analyzer") {
                    continue;
                }
                arr.get(1)
            }
            _ => continue,
        };

        if let Some(opts) = options
            && let Some(rules) = opts.get("releaseRules").and_then(|r| r.as_array())
        {
            let mut types = HashSet::new();
            for rule in rules {
                if let Some(t) = rule.get("type").and_then(|v| v.as_str()) {
                    types.insert(t.to_string());
                }
            }
            if !types.is_empty() {
                return Some(types);
            }
        }
    }

    None
}

/// The standard angular/conventional-commits commit types used as a fallback
/// when no explicit release rules are configured.
fn default_angular_types() -> HashSet<String> {
    [
        "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore",
        "revert",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
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
    fn test_extract_tag_prefix_with_v() {
        assert_eq!(extract_tag_prefix("v${version}"), "v");
    }

    #[test]
    fn test_extract_tag_prefix_no_prefix() {
        assert_eq!(extract_tag_prefix("${version}"), "");
    }

    #[test]
    fn test_extract_tag_prefix_custom() {
        assert_eq!(extract_tag_prefix("release-${version}"), "release-");
    }

    #[test]
    fn test_parse_json_tag_format() {
        let content = r#"{"tagFormat": "v${version}", "branches": ["main"]}"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        assert_eq!(config.version.unwrap().tag_prefix, "v");
    }

    #[test]
    fn test_parse_json_no_tag_format_defaults_v() {
        let content = r#"{"branches": ["main"]}"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        assert_eq!(config.version.unwrap().tag_prefix, "v");
    }

    #[test]
    fn test_parse_yaml_tag_format() {
        let content = "tagFormat: v${version}\nbranches:\n  - main\n";
        let f = write_temp(content, ".yaml");
        let config = parse(f.path()).unwrap();
        assert_eq!(config.version.unwrap().tag_prefix, "v");
    }

    #[test]
    fn test_parse_json_release_rules_types() {
        let content = r#"{
            "plugins": [
                ["@semantic-release/commit-analyzer", {
                    "releaseRules": [
                        {"type": "feat", "release": "minor"},
                        {"type": "fix", "release": "patch"},
                        {"type": "perf", "release": "patch"}
                    ]
                }]
            ]
        }"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        let types = &config.commit.types;
        assert!(types.contains("feat"));
        assert!(types.contains("fix"));
        assert!(types.contains("perf"));
    }

    #[test]
    fn test_parse_js_tag_format() {
        let content = r#"module.exports = { tagFormat: 'v${version}', branches: ['main'] };"#;
        let f = write_temp(content, ".js");
        let config = parse(f.path()).unwrap();
        assert_eq!(config.version.unwrap().tag_prefix, "v");
    }

    #[test]
    fn test_default_angular_types_fallback() {
        // when no plugins with releaseRules are present, angular defaults apply
        let content = r#"{"branches": ["main"]}"#;
        let f = write_temp(content, ".json");
        let config = parse(f.path()).unwrap();
        assert!(config.commit.types.contains("feat"));
        assert!(config.commit.types.contains("fix"));
        assert!(config.commit.types.contains("chore"));
    }
}
