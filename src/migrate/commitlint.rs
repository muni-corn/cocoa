//! Commitlint configuration migration.
//!
//! Parses `.commitlintrc.json`, `.commitlintrc.yaml`, `.commitlintrc.yml`, and
//! `.commitlintrc` (JSON) files and converts them to a cocoa [`Config`].
//!
//! JavaScript and TypeScript variants (`commitlint.config.js`, etc.) are
//! listed for file detection but cannot be statically parsed; they will
//! produce a [`MigrateError::Parse`] with an explanatory message.

use std::path::Path;

use crate::{config::Config, migrate::MigrateError};

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

/// Parse a commitlint configuration file and convert it to a cocoa [`Config`].
///
/// Supported formats: JSON and YAML.
/// JavaScript/TypeScript files are rejected with a descriptive error.
pub fn parse(_path: &Path) -> Result<Config, MigrateError> {
    Err(MigrateError::Parse(
        "commitlint migration not yet implemented".to_string(),
    ))
}
