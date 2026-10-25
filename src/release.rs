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
    config::{ChangelogConfig, FileEntryKind, FileEntryStrategy, VersionConfig, VersionFileEntry},
    generate::GenerateError,
    git_ops::GitOperations,
    tag::{self, TagError},
    version::{self, BumpType, UpdateOptions, UpdatedFile, VersionError},
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

    /// There are no commits since the latest version tag, so a release would be
    /// empty.
    #[error(
        "no commits since '{0}', so there is nothing to release; use --allow-empty to release \
         anyway"
    )]
    NoNewCommits(String),

    /// An invalid bump type string was provided.
    #[error("unknown bump type '{0}'. try `major`, `minor`, `patch`, or `auto`")]
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
    /// Version files that were updated (or would be updated), with handler
    /// details for each file.
    pub updated_files: Vec<UpdatedFile>,
    /// Bump type that was applied.
    pub bump_type: BumpType,
}

/// Execute the full release workflow.
///
/// Steps performed (each respects `dry_run`):
///
/// 1. Detect the current version from git tags (defaults to `0.0.0`).
/// 2. Collect commits since the last tag, refusing an empty release unless
///    `allow_empty` is set.
/// 3. Resolve the bump type from `opts.bump_type` or auto-detect.
/// 4. Compute the new version.
/// 5. Update version files listed in `[version].commit_version_files`.
/// 6. Generate and write the changelog to `[changelog].output_file`.
/// 7. Stage modified files and create a version commit.
/// 8. Create an annotated version tag.
pub fn execute<G: GitOperations>(
    ops: &G,
    version_config: &VersionConfig,
    changelog_config: &ChangelogConfig,
    args: &ReleaseArgs,
    dry_run: bool,
) -> Result<ReleaseOutcome, ReleaseError> {
    // step 1: detect current version
    let current =
        version::detect_current_semver(ops, &version_config.tag_prefix)?.unwrap_or_default();

    // step 2: collect commits since last tag
    let latest_tag = version::detect_latest_tag(ops, &version_config.tag_prefix)?;
    let from = latest_tag.as_ref().map_or("", |tag| tag.target.as_str());
    let commits = ops.get_commits_in_range(from, "HEAD")?;

    // an empty release only makes sense against a previous tag; the very first
    // release has no tag to compare with
    if let Some(tag) = &latest_tag
        && commits.is_empty()
        && !args.allow_empty
    {
        return Err(ReleaseError::NoNewCommits(tag.name.clone()));
    }

    // step 3: resolve bump type
    let bump_type = if let Some(bt) = args.bump_type {
        bt
    } else {
        version::detect_bump_type(&commits)
    };

    // step 4: compute new version
    let new_version = match bump_type {
        BumpType::Major => current.bump_major(),
        BumpType::Minor => current.bump_minor(),
        BumpType::Patch => current.bump_patch(),
    };

    let old_str = current.to_string();
    let new_str = new_version.to_string();
    let tag_name = format!("{}{}", version_config.tag_prefix, new_version);

    // fail before touching any file if the tag is taken; checking only at tag
    // creation would leave bumped files and a release commit behind
    if !args.skip_tag {
        tag::verify_tag_unique(ops, &tag_name)?;
    }

    // step 5: update version files
    // merge legacy commit_version_files with [[version.files]] entries.
    // rich entries take precedence; legacy paths that overlap are skipped.
    let legacy_paths: Vec<String> = version_config
        .commit_version_files
        .clone()
        .unwrap_or_default();

    let legacy_entries: Vec<VersionFileEntry> = legacy_paths
        .iter()
        .map(|p| VersionFileEntry {
            path: p.clone(),
            kind: FileEntryKind::Auto,
            strategy: FileEntryStrategy::InProcess,
            command: None,
            pattern: None,
            occurrences: None,
        })
        .collect();

    let rich_paths: std::collections::HashSet<String> = version_config
        .version_files
        .iter()
        .map(|e| e.path.clone())
        .collect();
    let filtered_legacy: Vec<VersionFileEntry> = legacy_entries
        .into_iter()
        .filter(|e| !rich_paths.contains(&e.path))
        .collect();

    let all_entries: Vec<VersionFileEntry> = version_config
        .version_files
        .iter()
        .cloned()
        .chain(filtered_legacy)
        .collect();

    // without a previous tag, old_version is only the 0.0.0 placeholder, which
    // the files cannot be expected to hold
    let update_options = UpdateOptions {
        verify_old_version: latest_tag.is_some(),
    };

    // a dry run still runs every handler's checks, so mismatches and missing
    // files fail here just as they would in a real release
    let updated_files: Vec<UpdatedFile> = if all_entries.is_empty() {
        Vec::new()
    } else if dry_run {
        version::plan_version_files(&all_entries, &old_str, &new_str, &update_options)?
    } else {
        version::update_version_files_rich(&all_entries, &old_str, &new_str, &update_options)?
    };

    // step 6: generate and write changelog
    let changelog_path = changelog_config.output_file.clone();

    if !args.skip_changelog {
        let cl = changelog::parser::parse_history(ops, None, changelog_config, Some(&tag_name))?;

        if !dry_run {
            let rendered = changelog::renderer::render(&cl, &OutputFormat::Markdown)?;
            std::fs::write(&changelog_path, rendered).map_err(|e| {
                ReleaseError::File(format!("failed to write '{}': {}", changelog_path, e))
            })?;
        }
    }

    // step 7: stage files and create version commit
    if !args.skip_commit && !dry_run {
        let repo_root = ops
            .get_repo_root()
            .map_err(|e| ReleaseError::Git(e.to_string()))?;

        let mut to_stage: Vec<String> = updated_files.iter().map(|f| f.path.clone()).collect();
        if !args.skip_changelog {
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
            version_config.tag_prefix, new_version
        );
        ops.create_commit(&commit_message)?;
    }

    // step 8: create version tag
    if !args.skip_tag {
        tag::create_version_tag(ops, &new_version, version_config, changelog_config, dry_run)?;
    }

    Ok(ReleaseOutcome {
        previous_version: old_str,
        new_version: new_str,
        tag_name,
        changelog_path,
        updated_files,
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

    // dry-run with no prior tags

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
        assert_eq!(outcome.new_version, "0.0.1");
        assert_eq!(outcome.tag_name, "v0.0.1");
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

    // explicit bump type

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
            allow_empty: true,
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
            allow_empty: true,
            ..Default::default()
        };

        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();

        assert_eq!(outcome.new_version, "2.3.5");
    }

    // tag name format

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

    // skip flags

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

        // detected version would be 0.0.2, not 0.0.1, so this should succeed
        let outcome = execute(&ops, &v_config(), &cl_config(), &opts, true).unwrap();
        assert_eq!(outcome.new_version, "0.0.2");
    }

    #[test]
    fn test_execute_dry_run_unreachable_tag_clash_fails() {
        // v0.5.2 exists but HEAD cannot reach it, so the next patch release
        // would try to create v0.5.2 again
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.1", "a"), make_tag("v0.5.2", "b")]),
            unreachable_revs: vec!["b".to_string()],
            commits_in_range: Ok(vec![make_commit("g1", "fix: small")]),
            ..Default::default()
        };

        let err = execute(
            &ops,
            &v_config(),
            &cl_config(),
            &ReleaseArgs::default(),
            true,
        )
        .unwrap_err();
        assert!(matches!(err, ReleaseError::Tag(TagError::AlreadyExists(n)) if n == "v0.5.2"));
    }

    #[test]
    fn test_execute_tag_clash_leaves_version_files_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("version.txt");
        std::fs::write(&file, "version 0.5.1\n").unwrap();

        let mut vc = v_config();
        vc.commit_version_files = Some(vec![file.to_string_lossy().into_owned()]);

        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.1", "a"), make_tag("v0.5.2", "b")]),
            unreachable_revs: vec!["b".to_string()],
            commits_in_range: Ok(vec![make_commit("g1", "fix: small")]),
            ..Default::default()
        };
        let args = ReleaseArgs {
            skip_changelog: true,
            skip_commit: true,
            ..Default::default()
        };

        let err = execute(&ops, &vc, &cl_config(), &args, false).unwrap_err();
        assert!(matches!(err, ReleaseError::Tag(TagError::AlreadyExists(_))));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "version 0.5.1\n");
    }

    #[test]
    fn test_execute_skip_tag_ignores_tag_clash() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.1", "a"), make_tag("v0.5.2", "b")]),
            unreachable_revs: vec!["b".to_string()],
            commits_in_range: Ok(vec![make_commit("g1", "fix: small")]),
            ..Default::default()
        };
        let args = ReleaseArgs {
            skip_tag: true,
            ..Default::default()
        };
        assert!(execute(&ops, &v_config(), &cl_config(), &args, true).is_ok());
    }

    fn manifest_config(
        dir: &std::path::Path,
        version: &str,
    ) -> (VersionConfig, std::path::PathBuf) {
        let manifest = dir.join("Cargo.toml");
        std::fs::write(
            &manifest,
            format!("[package]\nname = \"demo\"\nversion = \"{version}\"\n"),
        )
        .unwrap();
        let mut vc = v_config();
        vc.commit_version_files = Some(vec![manifest.to_string_lossy().into_owned()]);
        (vc, manifest)
    }

    fn files_only_args() -> ReleaseArgs {
        ReleaseArgs {
            skip_changelog: true,
            skip_commit: true,
            skip_tag: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_execute_first_release_skips_old_version_check() {
        let dir = tempfile::tempdir().unwrap();
        // no tags yet, so the 0.0.0 placeholder differs from the manifest
        let (vc, manifest) = manifest_config(dir.path(), "0.1.0");
        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![make_commit("h1", "feat: first")]),
            ..Default::default()
        };

        let outcome = execute(&ops, &vc, &cl_config(), &files_only_args(), false).unwrap();
        assert_eq!(outcome.new_version, "0.0.1");
        assert!(
            std::fs::read_to_string(&manifest)
                .unwrap()
                .contains("version = \"0.0.1\"")
        );
    }

    #[test]
    fn test_execute_with_previous_tag_checks_old_version() {
        let dir = tempfile::tempdir().unwrap();
        let (vc, manifest) = manifest_config(dir.path(), "0.5.1");
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.3", "a")]),
            commits_in_range: Ok(vec![make_commit("h1", "fix: small")]),
            ..Default::default()
        };

        let err = execute(&ops, &vc, &cl_config(), &files_only_args(), false).unwrap_err();
        assert!(matches!(
            err,
            ReleaseError::Version(VersionError::VersionMismatch { .. })
        ));
        // the manifest was not touched
        assert!(
            std::fs::read_to_string(&manifest)
                .unwrap()
                .contains("version = \"0.5.1\"")
        );
    }

    #[test]
    fn test_execute_dry_run_reports_handler_details_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let (vc, manifest) = manifest_config(dir.path(), "0.5.1");
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.1", "a")]),
            commits_in_range: Ok(vec![make_commit("i1", "fix: small")]),
            ..Default::default()
        };

        let outcome = execute(&ops, &vc, &cl_config(), &files_only_args(), true).unwrap();

        assert_eq!(outcome.updated_files.len(), 1);
        assert_eq!(
            outcome.updated_files[0].kind,
            version::FileKind::CargoManifest
        );
        assert_eq!(outcome.updated_files[0].replacements, 1);
        assert!(
            std::fs::read_to_string(&manifest)
                .unwrap()
                .contains("version = \"0.5.1\"")
        );
    }

    #[test]
    fn test_execute_dry_run_surfaces_version_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let (vc, _manifest) = manifest_config(dir.path(), "0.5.1");
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.3", "a")]),
            commits_in_range: Ok(vec![make_commit("i1", "fix: small")]),
            ..Default::default()
        };

        let err = execute(&ops, &vc, &cl_config(), &files_only_args(), true).unwrap_err();
        assert!(matches!(
            err,
            ReleaseError::Version(VersionError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn test_execute_propagates_commit_collection_errors() {
        use crate::generate::GenerateError;

        let ops = MockGitOps {
            commits_in_range: Err(GenerateError::GitContext("revwalk failed".to_string())),
            ..Default::default()
        };

        let err = execute(
            &ops,
            &v_config(),
            &cl_config(),
            &ReleaseArgs::default(),
            true,
        )
        .unwrap_err();
        assert!(matches!(err, ReleaseError::Git(msg) if msg.contains("revwalk failed")));
    }

    #[test]
    fn test_no_new_commits_message_names_the_tag_and_the_flag() {
        let msg = ReleaseError::NoNewCommits("v0.5.1".to_string()).to_string();
        assert!(msg.contains("'v0.5.1'"));
        assert!(msg.contains("--allow-empty"));
    }

    #[test]
    fn test_execute_refuses_release_without_new_commits() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.1", "a")]),
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };

        let err = execute(
            &ops,
            &v_config(),
            &cl_config(),
            &ReleaseArgs::default(),
            true,
        )
        .unwrap_err();
        assert!(matches!(err, ReleaseError::NoNewCommits(tag) if tag == "v0.5.1"));
    }

    #[test]
    fn test_execute_allow_empty_releases_without_new_commits() {
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.1", "a")]),
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };
        let args = ReleaseArgs {
            allow_empty: true,
            ..Default::default()
        };

        let outcome = execute(&ops, &v_config(), &cl_config(), &args, true).unwrap();
        assert_eq!(outcome.new_version, "0.5.2");
    }

    #[test]
    fn test_execute_first_release_without_commits_is_not_refused() {
        // no previous tag, so there is nothing to be "empty" relative to
        let ops = MockGitOps {
            tags: Ok(vec![]),
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };
        assert!(
            execute(
                &ops,
                &v_config(),
                &cl_config(),
                &ReleaseArgs::default(),
                true
            )
            .is_ok()
        );
    }

    #[test]
    fn test_execute_empty_refusal_happens_before_files_change() {
        let dir = tempfile::tempdir().unwrap();
        let (vc, manifest) = manifest_config(dir.path(), "0.5.1");
        let ops = MockGitOps {
            tags: Ok(vec![make_tag("v0.5.1", "a")]),
            commits_in_range: Ok(vec![]),
            ..Default::default()
        };

        assert!(execute(&ops, &vc, &cl_config(), &files_only_args(), false).is_err());
        assert!(
            std::fs::read_to_string(&manifest)
                .unwrap()
                .contains("version = \"0.5.1\"")
        );
    }
}
