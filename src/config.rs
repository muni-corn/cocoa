use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::Value as TomlValue;

use crate::{ai::config::AiConfig, style::print_error_bold};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Read(#[from] std::io::Error),

    #[error("failed to parse config file: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("failed to serialize config for merging: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("configuration validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub commit: CommitConfig,
    pub ai: Option<AiConfig>,
    pub changelog: Option<ChangelogConfig>,
    pub version: Option<VersionConfig>,
}

/// Configuration for changelog generation.
///
/// Maps to the `[changelog]` section in `.cocoa.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangelogConfig {
    /// Path where the changelog file is written.
    #[serde(default = "default_changelog_output_file")]
    pub output_file: String,

    /// Whether to include merge commits in the changelog.
    #[serde(default)]
    pub include_merge_commits: bool,

    /// Whether to include revert commits in the changelog.
    #[serde(default = "default_true")]
    pub include_reverts: bool,

    /// Date format string used for version headings (strftime syntax).
    #[serde(default = "default_date_format")]
    pub date_format: String,

    /// Mapping of commit type to human-readable section title.
    ///
    /// Only types listed here appear in the changelog. The special key
    /// `"breaking"` controls the section heading for breaking changes.
    pub sections: Option<HashMap<String, String>>,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            output_file: default_changelog_output_file(),
            include_merge_commits: false,
            include_reverts: true,
            date_format: default_date_format(),
            sections: None,
        }
    }
}

fn default_changelog_output_file() -> String {
    "CHANGELOG.md".to_string()
}

fn default_date_format() -> String {
    "%Y-%m-%d".to_string()
}

/// Versioning strategy.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionStrategy {
    /// Semantic versioning (MAJOR.MINOR.PATCH).
    Semver,
    /// Calendar versioning based on dates.
    Calver,
}

/// Configuration for version management.
///
/// Maps to the `[version]` section in `.cocoa.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionConfig {
    /// Versioning strategy to use.
    #[serde(default = "default_version_strategy")]
    pub strategy: VersionStrategy,

    /// Prefix prepended to version tags (e.g. `"v"` produces `v1.2.3`).
    #[serde(default = "default_tag_prefix")]
    pub tag_prefix: String,

    /// Whether to GPG-sign version tags.
    #[serde(default)]
    pub sign_tags: bool,

    /// Files to search and update when bumping the version.
    pub commit_version_files: Option<Vec<String>>,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            strategy: default_version_strategy(),
            tag_prefix: default_tag_prefix(),
            sign_tags: false,
            commit_version_files: None,
        }
    }
}

fn default_version_strategy() -> VersionStrategy {
    VersionStrategy::Semver
}

fn default_tag_prefix() -> String {
    "v".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitConfig {
    #[serde(default = "default_commit_types")]
    pub types: HashSet<String>,
    pub scopes: Option<HashSet<String>>,
    #[serde(default)]
    pub rules: CommitRules,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitRules {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub ignore_fixup_commits: bool,
    #[serde(default = "default_true")]
    pub ignore_amend_commits: bool,
    #[serde(default = "default_true")]
    pub ignore_squash_commits: bool,
    #[serde(default = "default_true")]
    pub ignore_merge_commits: bool,
    #[serde(default = "default_true")]
    pub ignore_revert_commits: bool,
    #[serde(default = "default_warn_rule_level")]
    pub warn: RuleLevel,
    #[serde(default = "default_deny_rule_level")]
    pub deny: RuleLevel,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RuleLevel {
    pub subject_length: Option<usize>,
    pub body_length: Option<usize>,
    pub no_scope: Option<bool>,
    pub no_body: Option<bool>,
    pub no_type: Option<bool>,
    pub no_breaking_change_footer: Option<bool>,
    pub regex_patterns: Option<Vec<String>>,
}

// --- serde default helpers ---

fn default_true() -> bool {
    true
}

fn default_commit_types() -> HashSet<String> {
    [
        "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style",
        "test",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_warn_rule_level() -> RuleLevel {
    RuleLevel {
        subject_length: Some(50),
        body_length: Some(250),
        no_scope: Some(true),
        no_body: Some(false),
        no_type: Some(true),
        no_breaking_change_footer: Some(true),
        regex_patterns: Some(vec![]),
    }
}

fn default_deny_rule_level() -> RuleLevel {
    RuleLevel {
        subject_length: Some(72),
        body_length: Some(500),
        no_scope: Some(false),
        no_body: Some(false),
        no_type: Some(true),
        no_breaking_change_footer: Some(false),
        regex_patterns: Some(vec![]),
    }
}

// --- Default impls ---

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            types: default_commit_types(),
            scopes: None,
            rules: CommitRules::default(),
        }
    }
}

impl Default for CommitRules {
    fn default() -> Self {
        Self {
            enabled: true,
            ignore_fixup_commits: true,
            ignore_amend_commits: true,
            ignore_squash_commits: true,
            ignore_merge_commits: true,
            ignore_revert_commits: true,
            warn: default_warn_rule_level(),
            deny: default_deny_rule_level(),
        }
    }
}

/// Deep-merges two TOML values, with `override_val` taking precedence over
/// `base`.
///
/// Tables are merged recursively. All other value types (arrays, strings,
/// integers, etc.) are replaced entirely by the override value.
fn merge_toml_values(base: TomlValue, override_val: TomlValue) -> TomlValue {
    match (base, override_val) {
        (TomlValue::Table(mut base_map), TomlValue::Table(override_map)) => {
            for (key, val) in override_map {
                let merged_val = match base_map.remove(&key) {
                    Some(base_val) => merge_toml_values(base_val, val),
                    None => val,
                };
                base_map.insert(key, merged_val);
            }
            TomlValue::Table(base_map)
        }
        // for all other types, the override value wins
        (_, override_val) => override_val,
    }
}

impl Config {
    /// Discovers config file paths in priority order, from lowest to highest.
    ///
    /// The order is:
    /// 1. `/etc/cocoa/cocoa.toml` (system, lowest priority)
    /// 2. `$XDG_CONFIG_HOME/cocoa/cocoa.toml` or `~/.config/cocoa/cocoa.toml`
    ///    (user)
    /// 3. `.cocoa.toml` in the current directory (repository, highest priority)
    pub fn discover() -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();

        // system config (lowest priority)
        paths.push(PathBuf::from("/etc/cocoa/cocoa.toml"));

        // user config via XDG or ~/.config fallback
        let user_config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|base| base.join("cocoa").join("cocoa.toml"));

        if let Some(user_path) = user_config {
            paths.push(user_path);
        }

        // repository config (highest priority)
        paths.push(PathBuf::from(".cocoa.toml"));

        paths
    }

    /// Loads and deep-merges config from the given paths in order.
    ///
    /// Paths are processed from lowest to highest priority, so later
    /// paths override earlier ones at the key level. Arrays and scalar
    /// values are replaced entirely (not appended).
    ///
    /// Returns `Config::default()` if none of the paths exist.
    pub fn load_merged(paths: &[PathBuf]) -> Result<Self, ConfigError> {
        let mut merged: Option<TomlValue> = None;

        for path in paths {
            if !path.exists() {
                continue;
            }

            let content = fs::read_to_string(path)?;
            let value: TomlValue = toml::from_str(&content)?;

            merged = Some(match merged {
                None => value,
                Some(base) => merge_toml_values(base, value),
            });
        }

        match merged {
            None => Ok(Self::default()),
            Some(value) => {
                let config: Config =
                    serde::Deserialize::deserialize(value).map_err(ConfigError::Parse)?;
                config.validate()?;
                Ok(config)
            }
        }
    }

    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default<P: AsRef<std::path::Path>>(path: P) -> Self {
        match Self::load(path) {
            Ok(config) => config,
            Err(ConfigError::Validation(msg)) => {
                print_error_bold(format!("configuration validation failed: {}", msg));
                Self::default()
            }
            Err(_) => Self::default(),
        }
    }

    /// Discovers and cascades all config files, returning the merged result.
    ///
    /// Loads from system → user → repository, with each layer overriding the
    /// previous. Falls back to `Config::default()` if no config files exist or
    /// if a validation error occurs.
    pub fn load_discovered_or_default() -> Self {
        let paths = Self::discover();
        match Self::load_merged(&paths) {
            Ok(config) => config,
            Err(ConfigError::Validation(msg)) => {
                print_error_bold(format!("configuration validation failed: {}", msg));
                Self::default()
            }
            Err(e) => {
                print_error_bold(format!("failed to load configuration: {}", e));
                Self::default()
            }
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.commit.rules.validate()
    }

    pub fn get_allowed_types(&self) -> HashSet<String> {
        self.commit.types.iter().cloned().collect()
    }

    pub fn get_allowed_scopes(&self) -> Option<HashSet<String>> {
        self.commit
            .scopes
            .as_ref()
            .map(|scopes| scopes.iter().cloned().collect())
    }
}

impl CommitRules {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // check if deny values are less than or equal to warn values
        if let (Some(warn_subject), Some(deny_subject)) =
            (self.warn.subject_length, self.deny.subject_length)
            && deny_subject <= warn_subject
        {
            return Err(ConfigError::Validation(format!(
                "deny.subject_length ({}) must be greater than warn.subject_length ({})",
                deny_subject, warn_subject
            )));
        }

        if let (Some(warn_body), Some(deny_body)) = (self.warn.body_length, self.deny.body_length)
            && deny_body <= warn_body
        {
            return Err(ConfigError::Validation(format!(
                "deny.body_length ({}) must be greater than warn.body_length ({})",
                deny_body, warn_body
            )));
        }

        Ok(())
    }
}

impl RuleLevel {
    pub fn get_subject_length(&self) -> Option<usize> {
        self.subject_length
    }

    pub fn get_body_length(&self) -> Option<usize> {
        self.body_length
    }

    pub fn get_no_scope(&self) -> bool {
        self.no_scope.unwrap_or(false)
    }

    pub fn get_no_body(&self) -> bool {
        self.no_body.unwrap_or(false)
    }

    pub fn get_no_type(&self) -> bool {
        self.no_type.unwrap_or(true)
    }

    pub fn get_no_breaking_change_footer(&self) -> bool {
        self.no_breaking_change_footer.unwrap_or(true)
    }

    pub fn get_regex_patterns(&self) -> Vec<String> {
        self.regex_patterns.as_ref().cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.warn.subject_length, Some(50));
        assert_eq!(config.commit.rules.deny.subject_length, Some(72));
        assert!(config.commit.types.contains("feat"));
        assert!(config.commit.types.contains("fix"));
    }

    #[test]
    fn test_load_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[commit]
types = ["feat", "fix", "test"]
scopes = ["api", "ui"]

[commit.rules]
enabled = true
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true

[commit.rules.warn]
subject_length = 50
body_length = 200
no_scope = true
no_body = false
no_type = true
no_breaking_change_footer = false
regex_patterns = []

[commit.rules.deny]
subject_length = 100
body_length = 400
no_scope = false
no_body = false
no_type = true
no_breaking_change_footer = false
regex_patterns = []
"#
        )
        .unwrap();

        let config = Config::load(file.path()).unwrap();
        assert_eq!(config.commit.types.len(), 3);
        assert!(config.commit.types.contains("feat"));
        assert_eq!(config.commit.rules.warn.subject_length, Some(50));
        assert_eq!(config.commit.rules.deny.subject_length, Some(100));
        assert!(config.commit.rules.warn.get_no_scope());

        let scopes = config.get_allowed_scopes().unwrap();
        assert!(scopes.contains("api"));
        assert!(scopes.contains("ui"));
    }

    #[test]
    fn test_load_or_default_fallback() {
        let config = Config::load_or_default("nonexistent.toml");
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.warn.subject_length, Some(50));
    }

    #[test]
    fn test_config_validation_fails() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[commit]
types = ["feat", "fix"]

[commit.rules]
enabled = true
ignore_fixup_commits = true
ignore_amend_commits = true
ignore_squash_commits = true
ignore_merge_commits = true
ignore_revert_commits = true

[commit.rules.warn]
subject_length = 100

[commit.rules.deny]
subject_length = 50
"#
        )
        .unwrap();

        let result = Config::load(file.path());
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains(
                "deny.subject_length (50) must be greater than warn.subject_length (100)"
            ));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_discover_returns_paths_in_priority_order() {
        let paths = Config::discover();

        // should have at least 2 paths (system + repo), 3 if HOME is set
        assert!(paths.len() >= 2);

        // first path should be system config
        assert_eq!(paths[0], PathBuf::from("/etc/cocoa/cocoa.toml"));

        // last path should be repo config
        assert_eq!(paths[paths.len() - 1], PathBuf::from(".cocoa.toml"));
    }

    #[test]
    fn test_discover_uses_xdg_config_home() {
        // temporarily override XDG_CONFIG_HOME
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", "/tmp/test-xdg");
        }

        let paths = Config::discover();

        let user_path = paths
            .iter()
            .find(|p| p.to_string_lossy().contains("test-xdg"));
        assert!(
            user_path.is_some(),
            "should include XDG_CONFIG_HOME-based path"
        );

        // clean up
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        let mut file = NamedTempFile::new().unwrap();
        // minimal config - only specify a few fields
        writeln!(
            file,
            r#"
[commit]
types = ["feat", "fix"]
"#
        )
        .unwrap();

        let config = Config::load(file.path()).unwrap();
        // explicitly set field is used
        assert_eq!(config.commit.types.len(), 2);
        assert!(config.commit.types.contains("feat"));
        // rules should use defaults
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.warn.subject_length, Some(50));
        assert_eq!(config.commit.rules.deny.subject_length, Some(72));
    }

    // --- load_merged ---

    #[test]
    fn test_load_merged_empty_paths_returns_default() {
        let config = Config::load_merged(&[]).unwrap();
        // should be the same as default
        assert!(config.commit.rules.enabled);
        assert_eq!(config.commit.rules.warn.subject_length, Some(50));
    }

    #[test]
    fn test_load_merged_single_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[commit]
types = ["feat", "fix", "docs"]
"#
        )
        .unwrap();

        let config = Config::load_merged(&[file.path().to_path_buf()]).unwrap();
        assert_eq!(config.commit.types.len(), 3);
        assert!(config.commit.types.contains("docs"));
    }

    #[test]
    fn test_load_merged_later_file_overrides_earlier() {
        let mut base = NamedTempFile::new().unwrap();
        writeln!(
            base,
            r#"
[commit]
types = ["feat"]
[commit.rules.warn]
subject_length = 40
"#
        )
        .unwrap();

        let mut override_file = NamedTempFile::new().unwrap();
        writeln!(
            override_file,
            r#"
[commit.rules.warn]
subject_length = 60
"#
        )
        .unwrap();

        let config = Config::load_merged(&[
            base.path().to_path_buf(),
            override_file.path().to_path_buf(),
        ])
        .unwrap();

        // later file wins for warn.subject_length
        assert_eq!(config.commit.rules.warn.subject_length, Some(60));
    }

    #[test]
    fn test_load_merged_skips_nonexistent_paths() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[commit]\ntypes = [\"feat\"]").unwrap();
        let paths = vec![
            PathBuf::from("/does/not/exist.toml"),
            file.path().to_path_buf(),
        ];
        let config = Config::load_merged(&paths).unwrap();
        assert!(config.commit.types.contains("feat"));
    }

    // --- get_allowed_types / get_allowed_scopes ---

    #[test]
    fn test_get_allowed_types_returns_configured_types() {
        let config = Config::default();
        let types = config.get_allowed_types();
        assert!(types.contains("feat"));
        assert!(types.contains("fix"));
    }

    #[test]
    fn test_get_allowed_scopes_returns_none_when_not_configured() {
        let config = Config::default();
        assert!(config.get_allowed_scopes().is_none());
    }

    // --- load_or_default with validation error ---

    #[test]
    fn test_load_or_default_returns_default_on_validation_error() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[commit.rules.warn]
subject_length = 100
[commit.rules.deny]
subject_length = 50
"#
        )
        .unwrap();

        // validation error: deny must be > warn
        let config = Config::load_or_default(file.path());
        // should silently fall back to defaults
        assert_eq!(config.commit.rules.warn.subject_length, Some(50));
    }

    // --- ChangelogConfig defaults ---

    #[test]
    fn test_changelog_config_default_values() {
        let cfg = ChangelogConfig::default();
        assert_eq!(cfg.output_file, "CHANGELOG.md");
        assert!(!cfg.include_merge_commits);
        assert!(cfg.include_reverts);
        assert_eq!(cfg.date_format, "%Y-%m-%d");
        assert!(cfg.sections.is_none());
    }

    // --- VersionConfig defaults ---

    #[test]
    fn test_version_config_default_values() {
        let cfg = VersionConfig::default();
        assert_eq!(cfg.strategy, VersionStrategy::Semver);
        assert_eq!(cfg.tag_prefix, "v");
        assert!(!cfg.sign_tags);
        assert!(cfg.commit_version_files.is_none());
    }

    // --- CommitRules::validate body_length ---

    #[test]
    fn test_config_validation_fails_body_length() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[commit.rules.warn]
body_length = 300

[commit.rules.deny]
body_length = 100
"#
        )
        .unwrap();

        let result = Config::load(file.path());
        assert!(result.is_err());
        if let Err(ConfigError::Validation(msg)) = result {
            assert!(msg.contains("body_length"));
        } else {
            panic!("expected Validation error");
        }
    }

    // --- load_or_default with parse (non-validation) error ---

    #[test]
    fn test_load_or_default_returns_default_on_parse_error() {
        let mut file = NamedTempFile::new().unwrap();
        // invalid TOML syntax - use a literal string to avoid format issues
        file.write_all(b"this is = [not valid toml").unwrap();
        let config = Config::load_or_default(file.path());
        // falls back to defaults on parse error
        assert!(config.commit.rules.enabled);
    }
}
