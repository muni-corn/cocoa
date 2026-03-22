use anyhow::Result;
use clap::Args;
use rust_i18n::t;

use crate::{
    Config,
    git_ops::{Git2Ops, GitOperations},
    hook::{self, HookKind, UninstallOutcome},
    style::{
        goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error_bold,
        print_info, print_success_bold, print_warning,
    },
};

/// Arguments for the `cocoa unhook` subcommand.
#[derive(Args, Debug)]
pub struct UnhookArgs {
    /// Which hooks to remove.
    ///
    /// - `lint`     — `commit-msg` hook
    /// - `generate` — `prepare-commit-msg` hook
    /// - `all`      — both hooks (default)
    #[arg(value_enum, default_value_t = HookKind::All)]
    pub kind: HookKind,
}

/// Removes the cocoa git hooks selected by `args`.
///
/// Resolves the hooks directory from the current git repository and delegates
/// to [`hook::uninstall`]. Reports the outcome to the user and exits with an
/// appropriate code.
pub fn handle_unhook(_config: &Config, args: UnhookArgs, dry_run: bool) -> Result<()> {
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

    match hook::uninstall(&hooks_dir, args.kind, dry_run) {
        Ok(outcomes) => {
            // count how many were actually installed — if none, report "nothing to do"
            let any_present = outcomes
                .iter()
                .any(|o| !matches!(o, UninstallOutcome::NotInstalled));

            if !any_present {
                print_warning(t!("main.unhook.not_installed"));
                goodbye_with_warning();
            }

            for outcome in outcomes {
                match outcome {
                    UninstallOutcome::Removed { hook_path } => {
                        if dry_run {
                            print_info(t!(
                                "main.unhook.dry_run_remove",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        } else {
                            print_success_bold(t!(
                                "main.unhook.removed",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        }
                    }
                    UninstallOutcome::Restored {
                        hook_path,
                        backup_path,
                    } => {
                        if dry_run {
                            print_info(t!(
                                "main.unhook.dry_run_restore",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                backup = backup_path.display().to_string()
                            ));
                        } else {
                            print_success_bold(t!(
                                "main.unhook.restored",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        }
                    }
                    UninstallOutcome::NotInstalled => {
                        // silently skip individual not-installed hooks when at
                        // least one hook was present
                    }
                }
            }
            goodbye_with_success();
        }
        Err(hook::HookError::NotAGitRepo) => {
            print_error_bold(t!("main.hook.not_git_repo"));
            goodbye_with_death(5);
        }
        Err(hook::HookError::NotManagedByCocoa { hook_name }) => {
            print_error_bold(t!("main.unhook.not_managed", hook = hook_name));
            goodbye_with_death(1);
        }
        Err(e) => {
            print_error_bold(t!("main.unhook.remove_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}
