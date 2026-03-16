//! Version management for cocoa.
//!
//! Provides semantic and calendar versioning engines, version detection from
//! git tags, automatic bump type determination from commit history, and atomic
//! version file updates.

pub mod calver;
pub mod semver;

use thiserror::Error;

/// Errors from version management operations.
#[derive(Debug, Error)]
pub enum VersionError {
    /// A git operation failed.
    #[error("git operation failed: {0}")]
    Git(String),

    /// A file could not be read or written.
    #[error("file error for '{path}': {source}")]
    File {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The version string was not found in a target file.
    #[error("version string '{version}' not found in '{path}'")]
    NotFound { version: String, path: String },
}

/// The type of bump to apply to a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpType {
    /// Increment the major version (breaking change).
    Major,
    /// Increment the minor version (new feature).
    Minor,
    /// Increment the patch version (bug fix or other).
    Patch,
}
