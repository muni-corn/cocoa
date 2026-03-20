use std::process;

use anyhow::Result;
use rust_i18n::t;

use crate::{
    Config,
    git_ops::Git2Ops,
    style::{
        goodbye_with_death, goodbye_with_success, print_error_bold, print_info, print_success_bold,
    },
    tag,
};

/// Create an annotated version tag with the changelog as its message.
///
/// Resolves the target version (from the argument or by auto-detecting the
/// appropriate bump from commits since the last tag), verifies uniqueness, and
/// creates the tag. In dry-run mode the tag name and message are printed
/// without writing to git.
pub fn handle_tag(
    config: &Config,
    version_str: Option<&str>,
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

    let version = match tag::resolve_version(&git_ops, version_str, &v_config) {
        Ok(v) => v,
        Err(tag::TagError::Version(e)) => {
            print_error_bold(t!("main.tag.invalid_version", error = e.to_string()));
            goodbye_with_death(1);
        }
        Err(e) => {
            print_error_bold(t!("main.tag.resolve_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    };

    if !json_output {
        let tag_name = format!("{}{}", v_config.tag_prefix, version);
        print_info(t!("main.tag.preparing", name = tag_name));
    }

    match tag::create_version_tag(&git_ops, &version, &v_config, &cl_config, dry_run) {
        Ok((name, message)) => {
            if json_output {
                let out = serde_json::json!({
                    "tag_name": name,
                    "version": version.to_string(),
                    "dry_run": dry_run,
                    "message": message,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else if dry_run {
                print_info(t!("main.tag.dry_run", name = name));
                println!("\n{}\n", message);
                goodbye_with_success();
            } else {
                print_success_bold(t!("main.tag.created", name = name));
                goodbye_with_success();
            }
        }
        Err(tag::TagError::AlreadyExists(name)) => {
            if json_output {
                let out = serde_json::json!({
                    "success": false,
                    "error": format!("tag '{}' already exists", name)
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(1);
            }
            print_error_bold(t!("main.tag.already_exists", name = name));
            print_info(t!("main.tag.already_exists_hint"));
            goodbye_with_death(1);
        }
        Err(tag::TagError::Git(msg)) => {
            if json_output {
                let out = serde_json::json!({ "success": false, "error": msg });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(5);
            }
            print_error_bold(t!("main.tag.git_failed", error = msg));
            goodbye_with_death(5);
        }
        Err(e) => {
            if json_output {
                let out = serde_json::json!({ "success": false, "error": e.to_string() });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(1);
            }
            print_error_bold(t!("main.tag.failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}
