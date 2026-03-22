//! Full release workflow orchestration.
//!
//! Orchestrates the complete release pipeline:
//! 1. Detect current version from git tags
//! 2. Collect commits since the last tag
//! 3. Determine the bump type (explicit or auto-detected)
//! 4. Compute the new version
//! 5. Update configured version files
//! 6. Generate and write the changelog
//! 7. Stage modified files and create a version commit
//! 8. Create an annotated version tag

use thiserror::Error;

use crate::{
    changelog::{self, ChangelogError, OutputFormat},
    cmd::release::ReleaseArgs,
    config::{ChangelogConfig, VersionConfig},
    generate::GenerateError,
    git_ops::GitOperations,
    tag::{self, TagError},
    version::{self, BumpType, SemVer, VersionError},
};

/// Errors from the release orchestration.
#[derive(Debug, Error)]
pub enum ReleaseError {
    /// A version management operation failed.
    #[error(transparent)]
    Version(#[from] VersionError),

    /// Changelog generation or rendering failed.
    #[error(transparent)]
    Changelog(#[from] ChangelogError),

    /// Tag creation failed.
    #[error(transparent)]
    Tag(#[from] TagError),

    /// A git operation failed.
    #[error("git error: {0}")]
    Git(String),

    /// A file could not be written.
    #[error("file error: {0}")]
    File(String),

    /// An invalid bump type string was provided.
    #[error("unknown bump type '{0}' — use: major, minor, patch, or auto")]
    InvalidBumpType(String),
}

impl From<GenerateError> for ReleaseError {
    fn from(e: GenerateError) -> Self {
        ReleaseError::Git(e.to_string())
    }
}

/// The outcome of a completed (or simulated) release.
#[derive(Debug)]
pub struct ReleaseOutcome {
    /// Previous version string (e.g. `"1.2.3"`).
    pub previous_version: String,
    /// New version string (e.g. `"1.3.0"`).
    pub new_version: String,
    /// Full tag name created (e.g. `"v1.3.0"`).
    pub tag_name: String,
    /// Changelog file path that was written (or would be written).
    pub changelog_path: String,
    /// Version files that were updated (or would be updated).
    pub updated_files: Vec<String>,
    /// Bump type that was applied.
    pub bump_type: BumpType,
}

/// Execute the full release workflow.
///
/// Steps performed (each respects `dry_run`):
///
/// 1. Detect the current version from git tags (defaults to `0.0.0`).
/// 2. Collect commits since the last tag.
/// 3. Resolve the bump type from `opts.bump_type` or auto-detect.
/// 4. Compute the new version.
/// 5. Update version files listed in `[version].commit_version_files`.
/// 6. Generate and write the changelog to `[changelog].output_file`.
/// 7. Stage modified files and create a version commit.
/// 8. Create an annotated version tag.
pub fn execute<G: GitOperations>(
    ops: &G,
    v_config: &VersionConfig,
    cl_config: &ChangelogConfig,
    opts: &ReleaseArgs,
    dry_run: bool,
) -> Result<ReleaseOutcome, ReleaseError> {
    // ── step 1: detect current version ───────────────────────────────────────
    let current = match version::detect_current_semver(ops, &v_config.tag_prefix)? {
        Some(v) => v,
        None => SemVer::parse("0.0.0").expect("0.0.0 is always valid semver"),
    };

    // ── step 2: collect commits since last tag ────────────────────────────────
    let latest_tag = version::detect_latest_tag(ops, &v_config.tag_prefix)?;
    let commits = match &latest_tag {
        Some(tag) => ops
            .get_commits_in_range(&tag.target, "HEAD")
            .unwrap_or_default(),
        None => ops.get_commits_in_range("", "HEAD").unwrap_or_default(),
    };

    // ── step 3: resolve bump type ─────────────────────────────────────────────
    let bump_type = if let Some(bt) = opts.bump_type {
        bt
    } else {
        version::detect_bump_type(&commits)
    };

    // ── step 4: compute new version ───────────────────────────────────────────
    let new_version = match bump_type {
        BumpType::Major => current.bump_major(),
        BumpType::Minor => current.bump_minor(),
        BumpType::Patch => current.bump_patch(),
    };

    let old_str = current.to_string();
    let new_str = new_version.to_string();
    let tag_name = format!("{}{}", v_config.tag_prefix, new_version);

    // ── step 5: update version files ─────────────────────────────────────────
    let files: Vec<String> = v_config.commit_version_files.clone().unwrap_or_default();

    if !files.is_empty() && !dry_run {
        version::update_version_files(&files, &old_str, &new_str)?;
    }

    // ── step 6: generate and write changelog ──────────────────────────────────
    let changelog_path = cl_config.output_file.clone();

    if !opts.skip_changelog {
        let range = latest_tag.as_ref().map(|t| format!("{}..HEAD", t.name));
        let cl = changelog::parser::parse_history(ops, range.as_deref(), cl_config)?;

        if !dry_run {
            let rendered = changelog::renderer::render(&cl, &OutputFormat::Markdown, cl_config)?;
            std::fs::write(&changelog_path, rendered).map_err(|e| {
                ReleaseError::File(format!("failed to write '{}': {}", changelog_path, e))
            })?;
        }
    }

    // ── step 7: stage files and create version commit ─────────────────────────
    if !opts.skip_commit && !dry_run {
        // stage version files and changelog with git add
        let repo_root = ops
            .get_repo_root()
            .map_err(|e| ReleaseError::Git(e.to_string()))?;

        let mut to_stage: Vec<String> = files.clone();
        if !opts.skip_changelog {
            to_stage.push(changelog_path.clone());
        }

        for path in &to_stage {
            std::process::Command::new("git")
                .args(["add", path])
                .current_dir(&repo_root)
                .output()
                .map_err(|e| ReleaseError::Git(format!("git add '{}' failed: {}", path, e)))?;
        }

        let commit_message = format!(
            "chore(release): bump version to {}{}",
            v_config.tag_prefix, new_version
        );
        ops.create_commit(&commit_message)?;
    }

    // ── step 8: create version tag ────────────────────────────────────────────
    if !opts.skip_tag {
        tag::create_version_tag(ops, &new_version, v_config, cl_config, dry_run)?;
    }

    Ok(ReleaseOutcome {
        previous_version: old_str,
        new_version: new_str,
        tag_name,
        changelog_path,
        updated_files: files,
        bump_type,
    })
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

    fn make_commit(id: &str, message: &str) -> CommitInfo {
        CommitInfo {
            id: id.to_string(),
            summary: message.to_string(),
            author: "Test User".to_string(),
            timestamp: 1_000_000,
        }
    }

    fn make_tag(name: &str, target: &str) -> TagInfo {
        TagInfo {
            name: name.to_string(),
            message: Some(format!("release {}", name)),
            target: target.to_string(),
        }
    }

    // ── dry-run with no prior tags ────────────────────────────────────────────

    #[test]
    fn test_execute_dry_run_no_tags_feat_commit() {
        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![make_commit("a1", "feat: add widget")]),
            ..Default::default()
        };

        let opts = ReleaseArgs::default();

        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();

        assert_eq!(outcome.previous_version, "0.0.0");
        assert_eq!(outcome.new_version, "0.1.0");
        assert_eq!(outcome.tag_name, "v0.1.0");
        assert!(matches!(outcome.bump_type, BumpType::Minor));
    }

    #[test]
    fn test_execute_dry_run_breaking_bumps_major() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v1.2.3", "abc")]),
            commits_in_range: Ok(vec![make_commit("b1", "feat!: breaking change")]),
            ..Default::default()
        };
        let opts = ReleaseArgs::default();

        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();

        assert_eq!(outcome.previous_version, "1.2.3");
        assert_eq!(outcome.new_version, "2.0.0");
        assert!(matches!(outcome.bump_type, BumpType::Major));
    }

    // ── explicit bump type ────────────────────────────────────────────────────

    #[test]
    fn test_execute_explicit_major_bump() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v1.0.0", "abc")]),
            commits_in_range: Ok(vec![make_commit("c1", "fix: patch bug")]),
            ..Default::default()
        };
        let opts = ReleaseArgs {
            bump_type: Some(BumpType::Major),
            ..Default::default()
        };

        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();

        assert_eq!(outcome.new_version, "2.0.0");
        assert!(matches!(outcome.bump_type, BumpType::Major));
    }

    #[test]
    fn test_execute_explicit_minor_bump() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v1.0.0", "abc")]),
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };
        let opts = ReleaseArgs {
            bump_type: BumpType::Minor.into(),
            ..Default::default()
        };

        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();

        assert_eq!(outcome.new_version, "1.1.0");
    }

    #[test]
    fn test_execute_explicit_patch_bump() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v2.3.4", "abc")]),
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };
        let opts = ReleaseArgs {
            bump_type: BumpType::Patch.into(),
            ..Default::default()
        };

        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();

        assert_eq!(outcome.new_version, "2.3.5");
    }

    // ── tag name format ───────────────────────────────────────────────────────

    #[test]
    fn test_execute_tag_name_uses_prefix() {
        let mut vc = v_config();
        vc.tag_prefix = "release/".to_string();

        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![make_commit("d1", "fix: typo")]),
            ..Default::default()
        };
        let opts = ReleaseArgs::default();

        let outcome = execute(&ops, &vc, &cl_config(), &opts, true).unwrap();

        assert_eq!(outcome.tag_name, "release/0.0.1");
    }

    // ── skip flags ────────────────────────────────────────────────────────────

    #[test]
    fn test_execute_dry_run_skip_tag_succeeds() {
        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![make_commit("e1", "feat: something")]),
            ..Default::default()
        };
        let opts = ReleaseArgs {
            skip_tag: true,
            ..Default::default()
        };

        assert!(execute(&ops, &v_config(), &cl_config(), &opts, true).is_ok());
    }

    #[test]
    fn test_execute_dry_run_duplicate_tag_fails() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.0.1", "abc")]),
            commits_in_range: Ok(vec![make_commit("f1", "fix: small")]),
            ..Default::default()
        };
        let opts = ReleaseArgs::default();

        // detected version would be 0.0.2, not 0.0.1 — so this should succeed
        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();
        assert_eq!(outcome.new_version, "0.0.2");
    }
}
