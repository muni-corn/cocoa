use std::process;

use anyhow::Result;
use clap::Args;
use rust_i18n::t;

use crate::{
    Config,
    git_ops::Git2Ops,
    release,
    style::{
        goodbye_with_death, goodbye_with_success, print_error_bold, print_info, print_success_bold,
    },
    tag,
    version::{self, BumpType},
};

#[derive(Default, Args)]
pub struct ReleaseArgs {
    /// Bump type to apply.
    ///
    /// One of: major, minor, patch, or auto (default). Auto infers the
    /// bump type from conventional commits since the last version tag.
    #[arg(
        value_name = "BUMP_TYPE",
        help = "Bump type: major, minor, patch, or auto (default: auto)"
    )]
    pub bump_type: Option<BumpType>,

    /// Skip changelog generation and writing.
    #[arg(long, help = "Skip changelog generation and writing")]
    pub skip_changelog: bool,

    /// Skip staging files and creating the version commit.
    #[arg(long, help = "Skip staging files and creating the version commit")]
    pub skip_commit: bool,

    /// Skip tag creation.
    #[arg(long, help = "Skip tag creation")]
    pub skip_tag: bool,
}

/// Execute the full release workflow: bump version, update files, write
/// changelog, commit, and tag.
///
/// In dry-run mode the plan is shown without making any changes.
pub fn handle_release(
    config: &Config,
    args @ ReleaseArgs {
        skip_changelog,
        skip_commit,
        skip_tag,
        ..
    }: ReleaseArgs,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let v_config = config.version.clone().unwrap_or_default();
    let cl_config = config.changelog.clone().unwrap_or_default();

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    match release::execute(&git_ops, &v_config, &cl_config, &args, dry_run) {
        Ok(outcome) => {
            let bump_label = match outcome.bump_type {
                version::BumpType::Major => "major",
                version::BumpType::Minor => "minor",
                version::BumpType::Patch => "patch",
            };
            if json_output {
                let out = serde_json::json!({
                    "success": true,
                    "previous_version": outcome.previous_version,
                    "new_version": outcome.new_version,
                    "tag_name": outcome.tag_name,
                    "bump_type": bump_label,
                    "dry_run": dry_run,
                    "updated_files": outcome.updated_files,
                    "changelog_path": outcome.changelog_path,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else if dry_run {
                print_info(t!(
                    "main.release.dry_run_summary",
                    old = outcome.previous_version,
                    new = outcome.new_version,
                    bump_type = bump_label
                ));
                if !outcome.updated_files.is_empty() {
                    print_info(t!("main.release.dry_run_update_files"));
                    for f in &outcome.updated_files {
                        print_info(t!("main.release.dry_run_file", file = f));
                    }
                }
                if !skip_changelog {
                    print_info(t!(
                        "main.release.dry_run_changelog",
                        path = outcome.changelog_path
                    ));
                }
                if !skip_commit {
                    print_info(t!(
                        "main.release.dry_run_commit",
                        version = outcome.new_version
                    ));
                }
                if !skip_tag {
                    print_info(t!("main.release.dry_run_tag", name = outcome.tag_name));
                }
            } else {
                print_success_bold(t!(
                    "main.release.success",
                    old = outcome.previous_version,
                    new = outcome.new_version
                ));
                if !outcome.updated_files.is_empty() {
                    for f in &outcome.updated_files {
                        print_info(t!("main.release.updated_file", file = f));
                    }
                }
                if !skip_changelog {
                    print_info(t!(
                        "main.release.wrote_changelog",
                        path = outcome.changelog_path
                    ));
                }
                if !skip_tag {
                    print_info(t!("main.release.created_tag", name = outcome.tag_name));
                }
            }
            goodbye_with_success();
        }
        Err(e) => {
            if json_output {
                let (exit_code, error_msg) = match &e {
                    release::ReleaseError::Git(msg) => (5, msg.clone()),
                    _ => (1, e.to_string()),
                };
                let out = serde_json::json!({ "success": false, "error": error_msg });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(exit_code);
            }
            match e {
                release::ReleaseError::InvalidBumpType(s) => {
                    print_error_bold(t!("main.release.invalid_bump", bump_type = s));
                    goodbye_with_death(1);
                }
                release::ReleaseError::Tag(tag::TagError::AlreadyExists(name)) => {
                    print_error_bold(t!("main.release.tag_exists", name = name));
                    print_info(t!("main.release.tag_exists_hint"));
                    goodbye_with_death(1);
                }
                release::ReleaseError::Git(msg) => {
                    print_error_bold(t!("main.release.git_failed", error = msg));
                    goodbye_with_death(5);
                }
                _ => {
                    print_error_bold(t!("main.release.failed", error = e.to_string()));
                    goodbye_with_death(1);
                }
            }
        }
    }

    Ok(())
}
