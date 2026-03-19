use anyhow::Result;
use cocoa::{Config, git_ops::Git2Ops, interactive};
use rust_i18n::t;

use crate::style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error, print_error_bold,
    print_info, print_success_bold, print_warning,
};

/// Runs the interactive commit wizard and performs the commit.
///
/// Opens the configured git repository, collects commit details via
/// interactive prompts, validates the assembled message, and creates the
/// commit. In dry-run mode the message is printed but not committed.
pub fn handle_commit(config: &Config, dry_run: bool) -> Result<()> {
    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    match interactive::run(config, &git_ops, dry_run) {
        Ok(message) => {
            if dry_run {
                print_info(t!("main.commit.dry_run_done"));
                println!("\n{}\n", message);
            } else {
                print_success_bold(t!("main.commit.success"));
            }
            goodbye_with_success();
        }
        Err(interactive::InteractiveError::Aborted) => {
            print_warning(t!("main.commit.cancelled"));
            goodbye_with_warning();
        }
        Err(interactive::InteractiveError::Lint(msg)) => {
            print_error_bold(t!("main.commit.lint_failed"));
            print_error(&msg);
            goodbye_with_death(3);
        }
        Err(interactive::InteractiveError::Commit(msg)) => {
            print_error_bold(t!("main.commit.git_failed", error = msg));
            goodbye_with_death(5);
        }
        Err(e) => {
            print_error_bold(t!("main.commit.failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}
