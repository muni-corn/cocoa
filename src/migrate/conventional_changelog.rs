//! Conventional-changelog configuration migration.
//!
//! Parses `changelog.config.js`, `.changelog.config.js`, and related files,
//! extracting commit type definitions and section headings.

use std::path::Path;

use crate::{config::Config, migrate::MigrateError};

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

/// Parse a conventional-changelog configuration file and convert it to a
/// cocoa [`Config`].
pub fn parse(_path: &Path) -> Result<Config, MigrateError> {
    Err(MigrateError::Parse(
        "conventional-changelog migration not yet implemented".to_string(),
    ))
}
