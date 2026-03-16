//! Version management for cocoa.
//!
//! Provides semantic and calendar versioning engines, version detection from
//! git tags, automatic bump type determination from commit history, and atomic
//! version file updates.

pub mod calver;
pub mod semver;

pub use calver::{CalVer, CalVerError};
pub use semver::{SemVer, SemVerError};
use thiserror::Error;

use crate::git_ops::{GitOperations, TagInfo};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_ops::MockGitOps;

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
}
