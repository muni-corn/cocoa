//! Version management for cocoa.
//!
//! Provides semantic and calendar versioning engines, version detection from
//! git tags, automatic bump type determination from commit history, and atomic
//! version file updates.

pub mod calver;
pub mod cargo_lock;
pub mod cargo_manifest;
pub mod command;
pub mod handlers;
pub mod npm;
pub mod plain;
pub mod regex_handler;
pub mod semver;

use std::fmt::Display;

pub use calver::{CalVer, CalVerError};
use clap::ValueEnum;
pub use semver::{SemVer, SemVerError};
use thiserror::Error;

use crate::{
    commit::CommitMessage,
    git_ops::{CommitInfo, GitOperations, TagInfo},
};

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

/// Detect the latest semantic version from repository tags.
///
/// Tags must start with `prefix` (e.g. `"v"`). The version portion (after
/// stripping the prefix) must be a valid semver string. Returns `None` when no
/// matching tags exist.
///
/// # Example
/// ```ignore
/// let ops = Git2Ops::open()?;
/// if let Some(v) = detect_current_semver(&ops, "v")? {
///     println!("latest version: {v}");
/// }
/// ```
pub fn detect_current_semver(
    ops: &dyn GitOperations,
    prefix: &str,
) -> Result<Option<SemVer>, VersionError> {
    let tags = ops
        .get_tags()
        .map_err(|e| VersionError::Git(e.to_string()))?;

    let mut versions: Vec<SemVer> = tags
        .iter()
        .filter_map(|t| {
            let stripped = t.name.strip_prefix(prefix)?;
            SemVer::parse(stripped).ok()
        })
        .collect();

    versions.sort();
    Ok(versions.into_iter().last())
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

/// Return the `TagInfo` for the highest-versioned semver tag.
///
/// Scans all tags, strips `prefix`, parses each as semver, and returns the
/// tag record for the highest version. Returns `None` if there are no
/// versioned tags.
pub fn detect_latest_tag(
    ops: &dyn GitOperations,
    prefix: &str,
) -> Result<Option<TagInfo>, VersionError> {
    let tags = ops
        .get_tags()
        .map_err(|e| VersionError::Git(e.to_string()))?;

    let mut versioned: Vec<(SemVer, TagInfo)> = tags
        .into_iter()
        .filter_map(|t| {
            let stripped = t.name.strip_prefix(prefix)?;
            let v = SemVer::parse(stripped).ok()?;
            Some((v, t))
        })
        .collect();

    versioned.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(versioned.into_iter().last().map(|(_, t)| t))
}

/// Update the version string in each of the given files atomically.
///
/// Reads every file first, replaces all occurrences of `old_version` with
/// `new_version`, then writes them all. If any write fails the already-written
/// files are restored to their original content, so the set of files is either
/// fully updated or left completely unchanged.
///
/// Returns `VersionError::NotFound` if `old_version` does not appear in any
/// of the target files.
///
/// This function dispatches to the appropriate handler for each file.
/// Plain-text files use [`plain::PlainHandler`], which matches the historical
/// behavior of replacing every occurrence of the version string.
pub fn update_version_files(
    files: &[String],
    old_version: &str,
    new_version: &str,
) -> Result<(), VersionError> {
    use handlers::{Handler, apply_updates};
    use plain::PlainHandler;

    // phase 1: compute all updates via their handlers
    let mut pending = Vec::new();
    for path in files {
        let handler = PlainHandler::default();
        if let Some(update) = handler.prepare(path, old_version, new_version)? {
            pending.push(update);
        }
    }

    // phase 2: write atomically with rollback
    apply_updates(pending)?;
    Ok(())
}

/// Update version files using rich per-file entries from `[[version.files]]`.
///
/// Each entry specifies a path and an optional handler kind, strategy, and
/// extra options. Falls back to [`update_version_files`] for entries whose
/// kind resolves to `Plain`.
///
/// Returns a list of [`UpdatedFile`] records describing each update applied.
pub fn update_version_files_rich(
    entries: &[crate::config::VersionFileEntry],
    old_version: &str,
    new_version: &str,
) -> Result<Vec<UpdatedFile>, VersionError> {
    use cargo_lock::CargoLockHandler;
    use cargo_manifest::CargoManifestHandler;
    use command::run_command;
    use handlers::{Handler, apply_updates};
    use npm::{NpmLockHandler, NpmManifestHandler};
    use plain::PlainHandler;
    use regex_handler::RegexHandler;

    use crate::config::{FileEntryKind, FileEntryStrategy, Occurrences, OccurrencesNamed};

    let mut pending = Vec::new();

    for entry in entries {
        if entry.strategy == FileEntryStrategy::Skip {
            continue;
        }

        if entry.strategy == FileEntryStrategy::Command {
            // resolve the FileKind from the entry kind so the UpdatedFile
            // record carries the correct variant
            let kind = entry_kind_to_file_kind(&entry.kind);
            let cmd = entry.command.as_deref().unwrap_or(&[]);
            let update = run_command(&entry.path, cmd, kind, None)?;
            pending.push(update);
            continue;
        }

        let handler_result = match &entry.kind {
            FileEntryKind::Cargo => {
                CargoManifestHandler.prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::CargoLock => {
                CargoLockHandler::default().prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::Npm => {
                NpmManifestHandler.prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::NpmLock => {
                NpmLockHandler.prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::Regex => {
                let pattern = entry.pattern.as_deref().unwrap_or("");
                let occurrences = entry
                    .occurrences
                    .clone()
                    .unwrap_or(Occurrences::Named(OccurrencesNamed::First));
                RegexHandler::new(&entry.path, pattern, occurrences)?.prepare(
                    &entry.path,
                    old_version,
                    new_version,
                )?
            }
            // plain and auto use the plain text handler
            FileEntryKind::Plain | FileEntryKind::Auto => {
                let occurrences = entry
                    .occurrences
                    .clone()
                    .unwrap_or(Occurrences::Named(OccurrencesNamed::All));
                PlainHandler { occurrences }.prepare(&entry.path, old_version, new_version)?
            }
            // remaining structured handlers fall through to plain for now;
            // each will get its own handler in subsequent commits
            _ => PlainHandler::default().prepare(&entry.path, old_version, new_version)?,
        };

        if let Some(update) = handler_result {
            pending.push(update);
        }
    }

    apply_updates(pending)
}

/// Map a `FileEntryKind` config value to the corresponding `FileKind` enum.
fn entry_kind_to_file_kind(kind: &crate::config::FileEntryKind) -> FileKind {
    use crate::config::FileEntryKind;
    match kind {
        FileEntryKind::Cargo => FileKind::CargoManifest,
        FileEntryKind::CargoLock => FileKind::CargoLock,
        FileEntryKind::Npm => FileKind::NpmManifest,
        FileEntryKind::NpmLock => FileKind::NpmLock,
        FileEntryKind::PnpmLock => FileKind::PnpmLock,
        FileEntryKind::YarnLock => FileKind::YarnLock,
        FileEntryKind::Pyproject => FileKind::Pyproject,
        FileEntryKind::Regex => FileKind::Regex,
        FileEntryKind::Plain | FileEntryKind::Auto => FileKind::Plain,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::git_ops::{CommitInfo, MockGitOps};

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
    fn test_detect_current_semver_no_tags() {
        let ops = MockGitOps::default();
        let result = detect_current_semver(&ops, "v").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_current_semver_single_tag() {
        let ops = MockGitOps {
            tags: Ok(vec![TagInfo {
                name: "v1.2.3".to_string(),
                message: None,
                target: "abc".to_string(),
            }]),
            ..Default::default()
        };
        let result = detect_current_semver(&ops, "v").unwrap();
        assert_eq!(result.unwrap().to_string(), "1.2.3");
    }

    #[test]
    fn test_detect_current_semver_picks_latest() {
        let ops = MockGitOps {
            tags: Ok(vec![
                TagInfo {
                    name: "v1.0.0".to_string(),
                    message: None,
                    target: "a".to_string(),
                },
                TagInfo {
                    name: "v2.1.0".to_string(),
                    message: None,
                    target: "b".to_string(),
                },
                TagInfo {
                    name: "v1.5.3".to_string(),
                    message: None,
                    target: "c".to_string(),
                },
            ]),
            ..Default::default()
        };
        let result = detect_current_semver(&ops, "v").unwrap();
        assert_eq!(result.unwrap().to_string(), "2.1.0");
    }

    #[test]
    fn test_detect_current_semver_ignores_non_semver_tags() {
        let ops = MockGitOps {
            tags: Ok(vec![
                TagInfo {
                    name: "release-2024".to_string(),
                    message: None,
                    target: "a".to_string(),
                },
                TagInfo {
                    name: "v1.0.0".to_string(),
                    message: None,
                    target: "b".to_string(),
                },
            ]),
            ..Default::default()
        };
        let result = detect_current_semver(&ops, "v").unwrap();
        assert_eq!(result.unwrap().to_string(), "1.0.0");
    }

    #[test]
    fn test_detect_current_semver_wrong_prefix() {
        let ops = MockGitOps {
            tags: Ok(vec![TagInfo {
                name: "v1.0.0".to_string(),
                message: None,
                target: "a".to_string(),
            }]),
            ..Default::default()
        };
        // tags use "v" prefix but we look for "release/"
        let result = detect_current_semver(&ops, "release/").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_latest_tag_returns_tag_info() {
        let ops = MockGitOps {
            tags: Ok(vec![
                TagInfo {
                    name: "v1.0.0".to_string(),
                    message: Some("first release".to_string()),
                    target: "abc".to_string(),
                },
                TagInfo {
                    name: "v2.0.0".to_string(),
                    message: Some("second release".to_string()),
                    target: "def".to_string(),
                },
            ]),
            ..Default::default()
        };
        let tag = detect_latest_tag(&ops, "v").unwrap().unwrap();
        assert_eq!(tag.name, "v2.0.0");
        assert_eq!(tag.message, Some("second release".to_string()));
    }

    #[test]
    fn test_detect_latest_tag_no_tags() {
        let ops = MockGitOps::default();
        assert!(detect_latest_tag(&ops, "v").unwrap().is_none());
    }

    // ── update_version_files ──────────────────────────────────────────────────

    #[test]
    fn test_update_version_files_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(&path, "version = \"1.0.0\"\n").unwrap();

        let files = vec![path.to_string_lossy().into_owned()];
        update_version_files(&files, "1.0.0", "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "version = \"2.0.0\"\n");
    }

    #[test]
    fn test_update_version_files_multiple_occurrences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, "version 1.0.0 and also 1.0.0\n").unwrap();

        let files = vec![path.to_string_lossy().into_owned()];
        update_version_files(&files, "1.0.0", "1.1.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "version 1.1.0 and also 1.1.0\n");
    }

    #[test]
    fn test_update_version_files_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.toml");
        let b = dir.path().join("b.json");
        fs::write(&a, "version = \"0.1.0\"\n").unwrap();
        fs::write(&b, "{\"version\":\"0.1.0\"}\n").unwrap();

        let files = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        update_version_files(&files, "0.1.0", "0.2.0").unwrap();

        assert!(fs::read_to_string(&a).unwrap().contains("0.2.0"));
        assert!(fs::read_to_string(&b).unwrap().contains("0.2.0"));
    }

    #[test]
    fn test_update_version_files_not_found_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, "some content\n").unwrap();

        let files = vec![path.to_string_lossy().into_owned()];
        let err = update_version_files(&files, "9.9.9", "10.0.0").unwrap_err();

        assert!(matches!(err, VersionError::NotFound { .. }));
    }

    #[test]
    fn test_update_version_files_missing_file_returns_error() {
        let files = vec!["/nonexistent/path/file.txt".to_string()];
        let err = update_version_files(&files, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::File { .. }));
    }
}
