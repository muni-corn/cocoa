use anyhow::Result;
use cocoa::{
    Config,
    git_ops::{Git2Ops, GitOperations},
    hook,
};
use rust_i18n::t;

use crate::style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error_bold, print_info,
    print_success_bold, print_warning,
};

/// Removes the cocoa `commit-msg` git hook, restoring a backup if present.
///
/// Resolves the hooks directory from the current git repository and delegates
/// to [`hook::uninstall`]. Reports the outcome to the user and exits with an
/// appropriate code.
pub fn handle_unhook(_config: &Config, dry_run: bool) -> Result<()> {
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

    match hook::uninstall(&hooks_dir, dry_run) {
        Ok(hook::UninstallOutcome::Removed { hook_path }) => {
            if dry_run {
                print_info(t!(
                    "main.unhook.dry_run_remove",
                    path = hook_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.unhook.removed",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::UninstallOutcome::Restored {
            hook_path,
            backup_path,
        }) => {
            if dry_run {
                print_info(t!(
                    "main.unhook.dry_run_restore",
                    hook = hook_path.display().to_string(),
                    backup = backup_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.unhook.restored",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::UninstallOutcome::NotInstalled) => {
            print_warning(t!("main.unhook.not_installed"));
            goodbye_with_warning();
        }
        Err(hook::HookError::NotAGitRepo) => {
            print_error_bold(t!("main.hook.not_git_repo"));
            goodbye_with_death(5);
        }
        Err(hook::HookError::NotManagedByCocoa) => {
            print_error_bold(t!("main.unhook.not_managed"));
            goodbye_with_death(1);
        }
        Err(e) => {
            print_error_bold(t!("main.unhook.remove_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}
