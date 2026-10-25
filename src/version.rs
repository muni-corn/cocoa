//! Version management for cocoa.
//!
//! Provides semantic and calendar versioning engines, version detection from
//! git tags, automatic bump type determination from commit history, and atomic
//! version file updates.

pub mod calver;
pub mod cargo_lock;
pub mod cargo_manifest;
pub mod command;
pub mod detect;
pub mod handlers;
pub mod npm;
pub mod plain;
pub mod pnpm;
pub mod pyproject;
pub mod regex_handler;
pub mod semver;
pub mod tags;
pub mod update;

use std::fmt::Display;

pub use calver::{CalVer, CalVerError};
use clap::ValueEnum;
pub use semver::{SemVer, SemVerError};
pub use tags::{VersionTags, detect_current_semver, detect_latest_tag, scan_version_tags};
use thiserror::Error;
pub use update::{
    UpdateOptions, entries_from_paths, plan_version_files, update_version_files,
    update_version_files_rich,
};

use crate::{commit::CommitMessage, git_ops::CommitInfo};

/// Errors from version management operations.
#[derive(Debug, Error)]
pub enum VersionError {
    /// A semantic version string could not be parsed.
    #[error(transparent)]
    Semver(#[from] SemVerError),

    /// A calendar version string could not be parsed.
    #[error(transparent)]
    Calver(#[from] CalVerError),

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

    /// A regex pattern did not match any text in the file.
    #[error("pattern '{pattern}' matched nothing in '{path}'")]
    PatternNoMatch { pattern: String, path: String },

    /// A regex pattern is missing the required named capture group `v`.
    #[error(
        "pattern '{pattern}' in '{path}' must contain a named capture group `v` (e.g. \
         `(?P<v>...)`)"
    )]
    PatternMissingGroup { pattern: String, path: String },

    /// A regex pattern could not be compiled.
    #[error("invalid pattern '{pattern}' in '{path}': {source}")]
    PatternInvalid {
        pattern: String,
        path: String,
        #[source]
        source: regex::Error,
    },

    /// A TOML or JSON manifest could not be parsed.
    #[error("could not parse '{path}': {message}")]
    ManifestParse { path: String, message: String },

    /// A required field was not found in a manifest.
    #[error("'{field}' not found in '{path}'")]
    ManifestFieldMissing { field: String, path: String },

    /// A file holds a different version than the one cocoa expected to replace.
    ///
    /// This usually means the version files and the git tags have drifted
    /// apart, for example after a reset or a manual edit.
    #[error(
        "'{path}': {what} is {found}, but {expected} was expected from the latest tag; are the \
         version files out of sync with your tags?"
    )]
    VersionMismatch {
        path: String,
        what: String,
        found: String,
        expected: String,
    },

    /// A toolchain command was not found on PATH.
    #[error("toolchain command '{tool}' not found on PATH; {hint}")]
    ToolchainNotFound { tool: String, hint: String },

    /// A toolchain command exited with a non-zero status.
    #[error("command '{command}' failed with status {status}:\n{stderr}")]
    ToolchainFailed {
        command: String,
        status: i32,
        stderr: String,
    },
}

/// The kind of file handler used to update a version.
///
/// Determines how cocoa replaces the version string in a file — from a
/// fully naive plain-text replace up to structured TOML or JSON parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    /// Plain text: all occurrences of the old version string are replaced.
    ///
    /// This matches the historical behavior of `update_version_files`.
    Plain,
    /// Regex pattern targeting a named capture group `v`.
    Regex,
    /// Structured update of a Cargo.toml `[package].version` field.
    CargoManifest,
    /// Workspace-aware update of a Cargo.lock lockfile.
    CargoLock,
    /// Structured update of a package.json `"version"` field.
    NpmManifest,
    /// Root-entry-only update of a package-lock.json lockfile.
    NpmLock,
    /// pnpm lockfile update (typically via command strategy).
    PnpmLock,
    /// Yarn lockfile update (typically via command strategy).
    YarnLock,
    /// Structured update of a pyproject.toml version field.
    Pyproject,
}

/// A record of one file updated (or that would be updated) during a release.
#[derive(Debug, Clone)]
pub struct UpdatedFile {
    /// Relative or absolute path of the file.
    pub path: String,
    /// Handler kind that was used.
    pub kind: FileKind,
    /// Number of textual replacements made (0 for command-driven updates).
    pub replacements: usize,
}

/// The type of bump to apply to a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BumpType {
    /// Increment the major version (breaking change).
    Major,
    /// Increment the minor version (new feature).
    Minor,
    /// Increment the patch version (bug fix or other).
    Patch,
}

impl Display for BumpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
            BumpType::Patch => "patch",
        })
    }
}

/// Analyze a list of commits and determine the appropriate bump type.
///
/// The rules follow the Conventional Commits spec:
///
/// - Any commit with a breaking change → `Major`
/// - Any `feat` commit → `Minor`
/// - All other commits → `Patch`
///
/// Commits whose messages cannot be parsed as conventional commits are
/// treated as `Patch`-level changes.
pub fn detect_bump_type(commits: &[CommitInfo]) -> BumpType {
    let mut has_feat = false;

    for c in commits {
        if let Ok(msg) = CommitMessage::parse(&c.summary) {
            if msg.breaking {
                return BumpType::Major;
            }
            if msg.commit_type.is_some_and(|t| t == "feat") {
                has_feat = true;
            }
        }
    }

    if has_feat {
        BumpType::Minor
    } else {
        BumpType::Patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_ops::CommitInfo;

    fn make_commit(message: &str) -> CommitInfo {
        CommitInfo {
            id: "abc123".to_string(),
            summary: message.to_string(),
            author: "test".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn test_detect_bump_type_breaking_change() {
        let commits = vec![make_commit("feat!: remove deprecated API")];
        assert_eq!(detect_bump_type(&commits), BumpType::Major);
    }

    #[test]
    fn test_detect_bump_type_breaking_wins_over_feat() {
        let commits = vec![
            make_commit("feat: add feature"),
            make_commit("fix!: breaking fix"),
        ];
        assert_eq!(detect_bump_type(&commits), BumpType::Major);
    }

    #[test]
    fn test_detect_bump_type_feat_gives_minor() {
        let commits = vec![
            make_commit("fix: patch bug"),
            make_commit("feat: add thing"),
        ];
        assert_eq!(detect_bump_type(&commits), BumpType::Minor);
    }

    #[test]
    fn test_detect_bump_type_only_fixes_gives_patch() {
        let commits = vec![
            make_commit("fix: fix bug"),
            make_commit("chore: update deps"),
        ];
        assert_eq!(detect_bump_type(&commits), BumpType::Patch);
    }

    #[test]
    fn test_detect_bump_type_empty_gives_patch() {
        assert_eq!(detect_bump_type(&[]), BumpType::Patch);
    }

    #[test]
    fn test_detect_bump_type_unparsable_treated_as_patch() {
        let commits = vec![make_commit("WIP not conventional")];
        assert_eq!(detect_bump_type(&commits), BumpType::Patch);
    }

    #[test]
    fn test_version_mismatch_message_names_file_found_and_expected() {
        let err = VersionError::VersionMismatch {
            path: "Cargo.lock".to_string(),
            what: "'cocoa'".to_string(),
            found: "0.5.1".to_string(),
            expected: "0.5.3".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Cargo.lock"));
        assert!(msg.contains("'cocoa' is 0.5.1"));
        assert!(msg.contains("0.5.3 was expected"));
        assert!(msg.contains("out of sync"));
    }
}
