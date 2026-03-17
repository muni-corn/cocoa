//! Semantic-release configuration migration.
//!
//! Parses `.releaserc`, `.releaserc.json`, `.releaserc.yaml`, and related
//! files, extracting tag format and commit analyzer configuration.

use std::path::Path;

use crate::{config::Config, migrate::MigrateError};

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

/// Parse a semantic-release configuration file and convert it to a cocoa
/// [`Config`].
///
/// Supported formats: JSON, YAML, and TOML. JavaScript files are parsed using
/// regex heuristics.
pub fn parse(_path: &Path) -> Result<Config, MigrateError> {
    Err(MigrateError::Parse(
        "semantic-release migration not yet implemented".to_string(),
    ))
}
