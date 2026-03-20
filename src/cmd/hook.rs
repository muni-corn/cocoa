use anyhow::Result;
use rust_i18n::t;

use crate::{
    Config,
    git_ops::{Git2Ops, GitOperations},
    hook,
    style::{
        goodbye_with_death, goodbye_with_success, print_error_bold, print_info, print_success_bold,
        print_warning,
    },
};

/// Installs the cocoa `commit-msg` git hook.
///
/// Resolves the hooks directory from the current git repository and delegates
/// to [`hook::install`]. Reports the outcome to the user and exits with an
/// appropriate code.
pub fn handle_hook(_config: &Config, dry_run: bool) -> Result<()> {
    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let hooks_dir = match git_ops.get_hook_path() {
        Ok(p) => p,
        Err(e) => {
            print_error_bold(t!("main.git.hook_path_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    match hook::install(&hooks_dir, dry_run) {
        Ok(hook::InstallOutcome::Installed { hook_path }) => {
            if dry_run {
                print_info(t!(
                    "main.hook.dry_run_install",
                    path = hook_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.hook.installed",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::InstallOutcome::Updated { hook_path }) => {
            if dry_run {
                print_info(t!(
                    "main.hook.dry_run_update",
                    path = hook_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.hook.updated",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::InstallOutcome::Replaced {
            hook_path,
            backup_path,
        }) => {
            if dry_run {
                print_info(t!(
                    "main.hook.dry_run_replace",
                    backup = backup_path.display().to_string()
                ));
            } else {
                print_warning(t!(
                    "main.hook.replaced_backup",
                    path = backup_path.display().to_string()
                ));
                print_success_bold(t!(
                    "main.hook.installed",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Err(hook::HookError::NotAGitRepo) => {
            print_error_bold(t!("main.hook.not_git_repo"));
            goodbye_with_death(5);
        }
        Err(e) => {
            print_error_bold(t!("main.hook.install_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}
