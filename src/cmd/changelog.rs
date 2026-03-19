use anyhow::Result;
use cocoa::{
    Config,
    changelog::{self, OutputFormat},
    git_ops::Git2Ops,
};
use rust_i18n::t;

use crate::style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error_bold, print_info,
    print_success_bold, print_warning,
};

/// Generate a changelog from git history and write or print it.
///
/// In dry-run mode the output is printed to stdout instead of being written to
/// the configured file. When no format is specified, Markdown is used.
pub fn handle_changelog(
    config: &Config,
    range: Option<&str>,
    format_str: Option<&str>,
    output_path: Option<&str>,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let cl_config = config.changelog.clone().unwrap_or_default();

    let format = match format_str {
        Some(s) => match OutputFormat::parse(s) {
            Some(f) => f,
            None => {
                print_error_bold(t!("main.changelog.unknown_format", format = s));
                goodbye_with_death(1);
            }
        },
        None => OutputFormat::Markdown,
    };

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let cl = match changelog::parser::parse_history(&git_ops, range, &cl_config) {
        Ok(c) => c,
        Err(changelog::ChangelogError::Git(msg)) => {
            print_error_bold(t!("main.changelog.git_failed", error = msg));
            goodbye_with_death(5);
        }
        Err(e) => {
            print_error_bold(t!("main.changelog.failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    };

    if cl.versions.is_empty() {
        print_warning(t!("main.changelog.empty"));
        goodbye_with_warning();
        return Ok(());
    }

    let rendered = match changelog::renderer::render(&cl, &format, &cl_config) {
        Ok(s) => s,
        Err(e) => {
            print_error_bold(t!("main.changelog.render_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    };

    let dest = output_path.unwrap_or(&cl_config.output_file);

    if json_output {
        // wrap the rendered content in a JSON envelope for machine consumption
        let out = serde_json::json!({
            "content": rendered,
            "format": format_str.unwrap_or("markdown"),
            "path": dest,
            "dry_run": dry_run,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else if dry_run {
        print_info(t!("main.changelog.dry_run", path = dest));
        println!("\n{}", rendered);
        goodbye_with_success();
    } else {
        std::fs::write(dest, &rendered)
            .map_err(|e| anyhow::anyhow!("failed to write '{}': {}", dest, e))?;
        print_success_bold(t!("main.changelog.wrote", path = dest));
        goodbye_with_success();
    }

    Ok(())
}
