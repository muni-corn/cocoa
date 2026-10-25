//! Version detection from git tags.
//!
//! Finds the current version and the latest version tag of a repository by
//! scanning its tags for semantic versions that start with the configured
//! prefix.

use crate::{
    git_ops::{GitOperations, TagInfo},
    version::{SemVer, VersionError},
};

/// Version tags in a repository, split by whether HEAD can reach them.
///
/// Tags orphaned by a reset or rebase stay in the repository but are not part
/// of the current history, so they must not influence the next version. Both
/// lists are sorted from the lowest to the highest version.
#[derive(Debug, Clone, Default)]
pub struct VersionTags {
    /// Tags whose commit is HEAD or one of its ancestors.
    pub reachable: Vec<(SemVer, TagInfo)>,
    /// Tags whose commit HEAD cannot reach.
    pub unreachable: Vec<(SemVer, TagInfo)>,
}

impl VersionTags {
    /// Return the highest-versioned tag that HEAD can reach.
    pub fn latest_reachable(&self) -> Option<&(SemVer, TagInfo)> {
        self.reachable.last()
    }

    /// Return unreachable tags that are newer than the latest reachable tag.
    ///
    /// These are the tags that would have been picked as the current version
    /// if reachability were not considered, so they are worth warning about.
    /// When no reachable tag exists, every unreachable tag is returned.
    pub fn ignored_newer(&self) -> Vec<&TagInfo> {
        let floor = self.latest_reachable().map(|(v, _)| v);
        self.unreachable
            .iter()
            .filter(|(v, _)| floor.is_none_or(|f| v > f))
            .map(|(_, t)| t)
            .collect()
    }
}

/// Scan the repository for semantic version tags and split them by
/// reachability from HEAD.
///
/// Tags must start with `prefix` and the remainder must be valid semver. A tag
/// that cannot be resolved to a commit is treated as unreachable, so that one
/// odd tag never blocks a release.
pub fn scan_version_tags(
    ops: &dyn GitOperations,
    prefix: &str,
) -> Result<VersionTags, VersionError> {
    let tags = ops
        .get_tags()
        .map_err(|e| VersionError::Git(e.to_string()))?;

    let mut found = VersionTags::default();

    for tag in tags {
        let Some(version) = tag
            .name
            .strip_prefix(prefix)
            .and_then(|v| SemVer::parse(v).ok())
        else {
            continue;
        };

        if ops.is_reachable_from_head(&tag.target).unwrap_or(false) {
            found.reachable.push((version, tag));
        } else {
            found.unreachable.push((version, tag));
        }
    }

    found.reachable.sort_by(|a, b| a.0.cmp(&b.0));
    found.unreachable.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(found)
}

/// Detect the latest semantic version from repository tags.
///
/// Tags must start with `prefix` (e.g. `"v"`). The version portion (after
/// stripping the prefix) must be a valid semver string. Only tags reachable
/// from HEAD are considered, so tags left behind by a reset are ignored.
/// Returns `None` when no matching tags exist.
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
    Ok(scan_version_tags(ops, prefix)?
        .reachable
        .pop()
        .map(|(v, _)| v))
}

/// Return the `TagInfo` for the highest-versioned semver tag.
///
/// Scans the tags reachable from HEAD, strips `prefix`, parses each as semver,
/// and returns the tag record for the highest version. Returns `None` if there
/// are no versioned tags.
pub fn detect_latest_tag(
    ops: &dyn GitOperations,
    prefix: &str,
) -> Result<Option<TagInfo>, VersionError> {
    Ok(scan_version_tags(ops, prefix)?
        .reachable
        .pop()
        .map(|(_, t)| t))
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

    fn tag(name: &str, target: &str) -> TagInfo {
        TagInfo {
            name: name.to_string(),
            message: None,
            target: target.to_string(),
        }
    }

    #[test]
    fn test_scan_version_tags_splits_by_reachability() {
        let ops = MockGitOps {
            tags: Ok(vec![
                tag("v0.5.1", "a"),
                tag("v0.5.3", "c"),
                tag("v0.5.2", "b"),
            ]),
            unreachable_revs: vec!["b".to_string(), "c".to_string()],
            ..Default::default()
        };
        let found = scan_version_tags(&ops, "v").unwrap();

        let reachable: Vec<_> = found.reachable.iter().map(|(_, t)| &t.name).collect();
        let unreachable: Vec<_> = found.unreachable.iter().map(|(_, t)| &t.name).collect();
        assert_eq!(reachable, ["v0.5.1"]);
        // sorted by version, not by input order
        assert_eq!(unreachable, ["v0.5.2", "v0.5.3"]);
    }

    #[test]
    fn test_scan_version_tags_skips_non_version_tags() {
        let ops = MockGitOps {
            tags: Ok(vec![tag("release-2024", "a"), tag("v1.0.0", "b")]),
            ..Default::default()
        };
        let found = scan_version_tags(&ops, "v").unwrap();
        assert_eq!(found.reachable.len(), 1);
        assert!(found.unreachable.is_empty());
    }

    #[test]
    fn test_latest_reachable_picks_highest() {
        let ops = MockGitOps {
            tags: Ok(vec![tag("v1.0.0", "a"), tag("v1.2.0", "b")]),
            ..Default::default()
        };
        let found = scan_version_tags(&ops, "v").unwrap();
        assert_eq!(found.latest_reachable().unwrap().1.name, "v1.2.0");
    }

    #[test]
    fn test_ignored_newer_only_lists_tags_above_latest_reachable() {
        let ops = MockGitOps {
            tags: Ok(vec![
                tag("v0.4.0", "old"),
                tag("v0.5.1", "a"),
                tag("v0.5.2", "b"),
            ]),
            // an old unreachable tag is harmless; only newer ones matter
            unreachable_revs: vec!["old".to_string(), "b".to_string()],
            ..Default::default()
        };
        let found = scan_version_tags(&ops, "v").unwrap();
        let ignored: Vec<_> = found.ignored_newer().iter().map(|t| &t.name).collect();
        assert_eq!(ignored, ["v0.5.2"]);
    }

    #[test]
    fn test_ignored_newer_without_reachable_tags_lists_all() {
        let ops = MockGitOps {
            tags: Ok(vec![tag("v0.1.0", "a")]),
            unreachable_revs: vec!["a".to_string()],
            ..Default::default()
        };
        let found = scan_version_tags(&ops, "v").unwrap();
        assert_eq!(found.ignored_newer().len(), 1);
    }

    #[test]
    fn test_detect_current_semver_ignores_unreachable_tags() {
        let ops = MockGitOps {
            tags: Ok(vec![tag("v0.5.1", "a"), tag("v0.5.3", "c")]),
            unreachable_revs: vec!["c".to_string()],
            ..Default::default()
        };
        let result = detect_current_semver(&ops, "v").unwrap();
        assert_eq!(result.unwrap().to_string(), "0.5.1");
    }

    #[test]
    fn test_detect_latest_tag_ignores_unreachable_tags() {
        let ops = MockGitOps {
            tags: Ok(vec![tag("v0.5.1", "a"), tag("v0.5.3", "c")]),
            unreachable_revs: vec!["c".to_string()],
            ..Default::default()
        };
        let latest = detect_latest_tag(&ops, "v").unwrap().unwrap();
        assert_eq!(latest.name, "v0.5.1");
    }

    #[test]
    fn test_detect_current_semver_none_when_only_unreachable_tags() {
        let ops = MockGitOps {
            tags: Ok(vec![tag("v0.5.3", "c")]),
            unreachable_revs: vec!["c".to_string()],
            ..Default::default()
        };
        assert_eq!(detect_current_semver(&ops, "v").unwrap(), None);
    }
}
