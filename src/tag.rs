//! Tag creation for version releases.
//!
//! Provides annotated tag creation with changelog-based messages and
//! auto-detection of the next version from commit history.

use thiserror::Error;

use crate::{
    changelog::{self, ChangelogError, OutputFormat},
    config::{ChangelogConfig, VersionConfig},
    generate::GenerateError,
    git_ops::GitOperations,
    version::{self, BumpType, SemVer, VersionError},
};

/// Errors from tag operations.
#[derive(Debug, Error)]
pub enum TagError {
    /// A tag with the given name already exists.
    #[error("tag '{0}' already exists")]
    AlreadyExists(String),

    /// A version operation failed.
    #[error(transparent)]
    Version(#[from] VersionError),

    /// A git operation failed.
    #[error("git error: {0}")]
    Git(String),

    /// Changelog generation failed.
    #[error(transparent)]
    Changelog(#[from] ChangelogError),
}

impl From<GenerateError> for TagError {
    fn from(e: GenerateError) -> Self {
        TagError::Git(e.to_string())
    }
}

/// Verify a tag name does not already exist in the repository.
///
/// Returns `TagError::AlreadyExists` when a tag with the given name is found,
/// allowing callers to abort before attempting a duplicate write.
pub fn verify_tag_unique<G: GitOperations>(ops: &G, tag_name: &str) -> Result<(), TagError> {
    let tags = ops.get_tags().map_err(|e| TagError::Git(e.to_string()))?;
    if tags.iter().any(|t| t.name == tag_name) {
        return Err(TagError::AlreadyExists(tag_name.to_string()));
    }
    Ok(())
}

/// Determine the next version to tag.
///
/// If `version_str` is provided it is parsed directly (with the configured
/// tag prefix stripped when present). Otherwise the current version is read
/// from existing tags and an appropriate bump is applied based on commits
/// since the last tag.
pub fn resolve_version<G: GitOperations>(
    ops: &G,
    version_str: Option<&str>,
    v_config: &VersionConfig,
) -> Result<SemVer, TagError> {
    if let Some(s) = version_str {
        // strip the configured tag prefix if the user included it
        let bare = s.strip_prefix(v_config.tag_prefix.as_str()).unwrap_or(s);
        return SemVer::parse(bare).map_err(|e| TagError::Version(e.into()));
    }

    // detect the current version from existing tags; default to 0.0.0
    let current = match version::detect_current_semver(ops, &v_config.tag_prefix)? {
        Some(v) => v,
        None => SemVer::parse("0.0.0").expect("0.0.0 is always valid semver"),
    };

    // collect commits since the last tag to determine the bump type
    let latest_tag = version::detect_latest_tag(ops, &v_config.tag_prefix)?;
    let commits = match &latest_tag {
        Some(tag) => ops
            .get_commits_in_range(&tag.target, "HEAD")
            .unwrap_or_default(),
        None => ops.get_commits_in_range("", "HEAD").unwrap_or_default(),
    };

    let bump = version::detect_bump_type(&commits);
    let new_version = match bump {
        BumpType::Major => current.bump_major(),
        BumpType::Minor => current.bump_minor(),
        BumpType::Patch => current.bump_patch(),
    };

    Ok(new_version)
}

/// Build the annotation message for a version tag from the changelog.
///
/// Generates a Markdown changelog for commits since the previous version tag.
/// Falls back to a plain release line when there are no notable commits.
pub fn build_tag_message<G: GitOperations>(
    ops: &G,
    version: &SemVer,
    v_config: &VersionConfig,
    cl_config: &ChangelogConfig,
) -> Result<String, TagError> {
    let latest_tag = version::detect_latest_tag(ops, &v_config.tag_prefix)?;
    let range = latest_tag.as_ref().map(|t| format!("{}..HEAD", t.name));

    let cl = changelog::parser::parse_history(ops, range.as_deref(), cl_config)?;

    let has_content = cl
        .versions
        .iter()
        .any(|v| !v.sections.is_empty() || !v.breaking_changes.is_empty());

    if !has_content {
        return Ok(format!("Release {}{}", v_config.tag_prefix, version));
    }

    let rendered = changelog::renderer::render(&cl, &OutputFormat::Markdown, cl_config)?;
    Ok(rendered)
}

/// Create an annotated git tag for the given version using changelog content
/// as the tag message.
///
/// In dry-run mode the tag name and message are returned without writing to
/// git. Returns `(tag_name, message)` on success.
pub fn create_version_tag<G: GitOperations>(
    ops: &G,
    version: &SemVer,
    v_config: &VersionConfig,
    cl_config: &ChangelogConfig,
    dry_run: bool,
) -> Result<(String, String), TagError> {
    let tag_name = format!("{}{}", v_config.tag_prefix, version);
    let sign = v_config.sign_tags;

    verify_tag_unique(ops, &tag_name)?;

    let message = build_tag_message(ops, version, v_config, cl_config)?;

    if dry_run {
        return Ok((tag_name, message));
    }

    ops.create_tag(&tag_name, &message, sign)
        .map_err(|e| TagError::Git(e.to_string()))?;

    Ok((tag_name, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{ChangelogConfig, VersionConfig},
        git_ops::{CommitInfo, MockGitOps, TagInfo},
    };

    fn v_config() -> VersionConfig {
        VersionConfig::default()
    }

    fn cl_config() -> ChangelogConfig {
        ChangelogConfig::default()
    }

    fn make_tag(name: &str) -> TagInfo {
        TagInfo {
            name: name.to_string(),
            message: Some(format!("release {}", name)),
            target: "abc123".to_string(),
        }
    }

    fn make_commit(id: &str, message: &str) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            summary: message.to_string(),
            author: "Test User".to_string(),
            timestamp: 1_000_000,
        }
    }

    // ── verify_tag_unique ─────────────────────────────────────────────────────

    #[test]
    fn test_verify_tag_unique_no_tags() {
        let ops = MockGitOps::default();
        assert!(verify_tag_unique(&ops, "v1.0.0").is_ok());
    }

    #[test]
    fn test_verify_tag_unique_different_tags() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.9.0")]),
            ..Default::default()
        };
        assert!(verify_tag_unique(&ops, "v1.0.0").is_ok());
    }

    #[test]
    fn test_verify_tag_unique_conflict() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v1.0.0")]),
            ..Default::default()
        };
        let err = verify_tag_unique(&ops, "v1.0.0").unwrap_err();
        assert!(matches!(err, TagError::AlreadyExists(name) if name == "v1.0.0"));
    }

    // ── resolve_version ───────────────────────────────────────────────────────

    #[test]
    fn test_resolve_version_explicit_bare() {
        let ops = MockGitOps::default();
        let v = resolve_version(&ops, Some("2.3.4"), &v_config()).unwrap();
        assert_eq!(v.to_string(), "2.3.4");
    }

    #[test]
    fn test_resolve_version_explicit_with_prefix() {
        let ops = MockGitOps::default();
        // user passes "v2.3.4" — the "v" prefix should be stripped
        let v = resolve_version(&ops, Some("v2.3.4"), &v_config()).unwrap();
        assert_eq!(v.to_string(), "2.3.4");
    }

    #[test]
    fn test_resolve_version_invalid_string() {
        let ops = MockGitOps::default();
        assert!(resolve_version(&ops, Some("not-a-version"), &v_config()).is_err());
    }

    #[test]
    fn test_resolve_version_auto_no_tags_feat_commit() {
        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![make_commit("a1", "feat: add thing")]),
            ..Default::default()
        };
        // starts from 0.0.0; feat commit → minor bump → 0.1.0
        let v = resolve_version(&ops, None, &v_config()).unwrap();
        assert_eq!(v.to_string(), "0.1.0");
    }

    #[test]
    fn test_resolve_version_auto_existing_tag_patch_commit() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v1.2.3")]),
            commits_in_range: Ok(vec![make_commit("a1", "fix: patch bug")]),
            ..Default::default()
        };
        let v = resolve_version(&ops, None, &v_config()).unwrap();
        assert_eq!(v.to_string(), "1.2.4");
    }

    #[test]
    fn test_resolve_version_auto_breaking_change() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v1.2.3")]),
            commits_in_range: Ok(vec![make_commit("a1", "feat!: breaking api change")]),
            ..Default::default()
        };
        let v = resolve_version(&ops, None, &v_config()).unwrap();
        assert_eq!(v.to_string(), "2.0.0");
    }

    // ── create_version_tag dry-run ────────────────────────────────────────────

    #[test]
    fn test_create_version_tag_dry_run_returns_name_and_message() {
        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![make_commit("a1", "feat: new thing")]),
            ..Default::default()
        };
        let version = SemVer::parse("1.0.0").unwrap();
        let (name, message) =
            create_version_tag(&ops, &version, &v_config(), &cl_config(), true).unwrap();
        assert_eq!(name, "v1.0.0");
        // message should be non-empty
        assert!(!message.is_empty());
    }

    #[test]
    fn test_create_version_tag_dry_run_duplicate_fails() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v1.0.0")]),
            ..Default::default()
        };
        let version = SemVer::parse("1.0.0").unwrap();
        let err = create_version_tag(&ops, &version, &v_config(), &cl_config(), true).unwrap_err();
        assert!(matches!(err, TagError::AlreadyExists(_)));
    }

    #[test]
    fn test_create_version_tag_no_commits_uses_fallback_message() {
        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };
        let version = SemVer::parse("1.0.0").unwrap();
        let (name, message) =
            create_version_tag(&ops, &version, &v_config(), &cl_config(), true).unwrap();
        assert_eq!(name, "v1.0.0");
        // no notable commits — falls back to plain release message
        assert!(message.contains("Release"));
        assert!(message.contains("1.0.0"));
    }
}
