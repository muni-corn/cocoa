use anyhow::Result;
use clap::Args;
use rust_i18n::t;

use crate::{
    Config,
    git_ops::{Git2Ops, GitOperations},
    hook::{self, HookKind},
    style::{
        goodbye_with_death, goodbye_with_success, print_error_bold, print_info, print_success_bold,
        print_warning,
    },
};

/// Arguments for the `cocoa hook` subcommand.
#[derive(Args, Debug)]
pub struct HookArgs {
    /// Which hooks to install.
    ///
    /// - `lint`:      `commit-msg` hook (lints messages with `cocoa lint`)
    /// - `generate`:  `prepare-commit-msg` hook (generates messages with AI)
    /// - `all`:       both hooks (default)
    #[arg(value_enum, default_value_t = HookKind::All)]
    pub kind: HookKind,
}

/// Installs the cocoa git hooks selected by `args`.
///
/// Resolves the hooks directory from the current git repository and delegates
/// to [`hook::install`]. Reports the outcome to the user and exits with an
/// appropriate code.
pub fn handle_hook(config: &Config, args: HookArgs, dry_run: bool) -> Result<()> {
    // validate AI is configured before installing the generate hook
    let needs_ai = matches!(args.kind, HookKind::Generate | HookKind::All);
    if needs_ai && config.ai.is_none() {
        print_error_bold(t!("main.hook.no_ai"));
        print_info(t!("main.hook.no_ai_hint"));
        goodbye_with_death(2);
    }

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

    match hook::install(&hooks_dir, args.kind, dry_run) {
        Ok(outcomes) => {
            for outcome in outcomes {
                match outcome {
                    hook::InstallOutcome::Installed { hook_path } => {
                        if dry_run {
                            print_info(t!(
                                "main.hook.dry_run_install",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        } else {
                            print_success_bold(t!(
                                "main.hook.installed",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        }
                    }
                    hook::InstallOutcome::Updated { hook_path } => {
                        if dry_run {
                            print_info(t!(
                                "main.hook.dry_run_update",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        } else {
                            print_success_bold(t!(
                                "main.hook.updated",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        }
                    }
                    hook::InstallOutcome::Replaced {
                        hook_path,
                        backup_path,
                    } => {
                        if dry_run {
                            print_info(t!(
                                "main.hook.dry_run_replace",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                backup = backup_path.display().to_string()
                            ));
                        } else {
                            print_warning(t!(
                                "main.hook.replaced_backup",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = backup_path.display().to_string()
                            ));
                            print_success_bold(t!(
                                "main.hook.installed",
                                hook = hook_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                path = hook_path.display().to_string()
                            ));
                        }
                    }
                }
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
