use std::process;

use anyhow::Result;
use cocoa::{
    Config,
    git_ops::{Git2Ops, GitOperations},
    version,
};
use rust_i18n::t;

use crate::style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error_bold, print_info,
    print_success_bold, print_warning,
};

/// Bump the project version and update configured version files.
///
/// Accepts an explicit bump type (major, minor, patch) or "auto" to detect
/// the appropriate bump from commits since the last version tag. In dry-run
/// mode the new version is displayed but no files are written.
pub fn handle_bump(
    config: &Config,
    bump_type_str: Option<&str>,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let v_config = config.version.clone().unwrap_or_default();

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    // detect the current version from git tags; default to 0.0.0 if none exist
    let current_version = match version::detect_current_semver(&git_ops, &v_config.tag_prefix) {
        Ok(Some(v)) => v,
        Ok(None) => {
            print_info(t!("main.bump.no_tags"));
            version::SemVer::parse("0.0.0").expect("0.0.0 is always valid semver")
        }
        Err(e) => {
            print_error_bold(t!("main.bump.detect_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    // resolve the bump type from the argument or auto-detect from commits
    let bump_type = match bump_type_str {
        Some("major") => version::BumpType::Major,
        Some("minor") => version::BumpType::Minor,
        Some("patch") => version::BumpType::Patch,
        Some("auto") | None => {
            // collect commits since the last version tag for analysis
            let commits = match version::detect_latest_tag(&git_ops, &v_config.tag_prefix) {
                Ok(Some(tag)) => git_ops
                    .get_commits_in_range(&tag.target, "HEAD")
                    .unwrap_or_default(),
                // no tags yet — scan all commits reachable from HEAD
                Ok(None) | Err(_) => git_ops.get_commits_in_range("", "HEAD").unwrap_or_default(),
            };
            let detected = version::detect_bump_type(&commits);
            let label = match detected {
                version::BumpType::Major => "major",
                version::BumpType::Minor => "minor",
                version::BumpType::Patch => "patch",
            };
            print_info(t!("main.bump.auto_detected", bump_type = label));
            detected
        }
        Some(other) => {
            print_error_bold(t!("main.bump.unknown_type", bump_type = other));
            goodbye_with_death(1);
        }
    };

    let new_version = match bump_type {
        version::BumpType::Major => current_version.bump_major(),
        version::BumpType::Minor => current_version.bump_minor(),
        version::BumpType::Patch => current_version.bump_patch(),
    };

    let old_str = current_version.to_string();
    let new_str = new_version.to_string();

    let bump_label = match bump_type {
        version::BumpType::Major => "major",
        version::BumpType::Minor => "minor",
        version::BumpType::Patch => "patch",
    };

    if !json_output {
        print_info(t!("main.bump.version_arrow", old = old_str, new = new_str));
    }

    let files: &[String] = v_config.commit_version_files.as_deref().unwrap_or(&[]);

    if files.is_empty() {
        if json_output {
            let out = serde_json::json!({
                "old_version": old_str,
                "new_version": new_str,
                "bump_type": bump_label,
                "dry_run": dry_run,
                "files": serde_json::Value::Array(vec![]),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else if dry_run {
            print_info(t!("main.bump.dry_run_no_files"));
            goodbye_with_warning();
        } else {
            print_warning(t!("main.bump.no_files"));
            goodbye_with_warning();
        }
        return Ok(());
    }

    if dry_run {
        if json_output {
            let out = serde_json::json!({
                "old_version": old_str,
                "new_version": new_str,
                "bump_type": bump_label,
                "dry_run": true,
                "files": files,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            print_info(t!("main.bump.dry_run_would_update", count = files.len()));
            for f in files {
                print_info(t!("main.bump.dry_run_file", file = f));
            }
            goodbye_with_success();
        }
    } else {
        match version::update_version_files(files, &old_str, &new_str) {
            Ok(()) => {
                if json_output {
                    let out = serde_json::json!({
                        "old_version": old_str,
                        "new_version": new_str,
                        "bump_type": bump_label,
                        "dry_run": false,
                        "files": files,
                    });
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else {
                    print_success_bold(t!("main.bump.bumped", old = old_str, new = new_str));
                    for f in files {
                        print_info(t!("main.bump.updated_file", file = f));
                    }
                    goodbye_with_success();
                }
            }
            Err(e) => {
                if json_output {
                    let out = serde_json::json!({
                        "success": false,
                        "error": e.to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&out)?);
                    process::exit(1);
                }
                print_error_bold(t!("main.bump.update_failed", error = e.to_string()));
                goodbye_with_death(1);
            }
        }
    }

    Ok(())
}
