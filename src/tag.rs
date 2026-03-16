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

    let message = build_tag_message(ops, version, v_config, cl_config)?;

    if dry_run {
        return Ok((tag_name, message));
    }

    ops.create_tag(&tag_name, &message, sign)
        .map_err(|e| TagError::Git(e.to_string()))?;

    Ok((tag_name, message))
}
