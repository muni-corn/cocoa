//! Git hook management for cocoa.
//!
//! Implements `cocoa hook` and `cocoa unhook`, which install and remove git
//! hooks. The `commit-msg` hook lints messages through `cocoa lint`, and the
//! `prepare-commit-msg` hook pre-fills messages via `cocoa generate`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use thiserror::Error;

/// Marker embedded in cocoa-managed hook scripts.
///
/// Used to detect whether an existing hook was installed by cocoa, which
/// determines whether it is safe to replace or restore a backup.
const COCOA_MARKER: &str = "# managed by cocoa";

/// Shell script written to `.git/hooks/commit-msg`.
///
/// The hook pipes the commit message file (passed as `$1` by git) into
/// `cocoa lint --stdin`, failing the commit when lint errors are found.
const LINT_HOOK_SCRIPT: &str = r#"\
#!/bin/sh
# managed by cocoa - do not edit
cocoa lint "$1""#;

/// Shell script written to `.git/hooks/prepare-commit-msg`.
///
/// The hook calls `cocoa generate --hook "$1"` to pre-fill the commit message
/// with an AI-generated suggestion before the editor opens. It skips
/// invocation when git already supplies a message source (amend, merge,
/// squash, or `-m`), so the hook only fires for fresh interactive commits.
const GENERATE_HOOK_SCRIPT: &str = "\
#!/bin/sh
# managed by cocoa - do not edit
# skip when git already has a message source (amend, merge, squash, or -m)
case \"$2\" in
  message|merge|squash|commit) exit 0 ;;
esac
cocoa generate \"$1\" \"$2\" \"$3\"
";

/// Name of the backup file saved when a pre-existing non-cocoa hook is found.
const BACKUP_SUFFIX: &str = ".cocoa-backup";

/// Selects which git hooks to install or remove.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum HookKind {
    /// The `commit-msg` hook. Lints commit messages with `cocoa lint`.
    Lint,
    /// The `prepare-commit-msg` hook. Generates messages with `cocoa
    /// generate`.
    Generate,
    /// Both hooks (default).
    All,
}

impl HookKind {
    /// Returns `(hook_filename, script_content)` pairs for this kind.
    fn hooks(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            HookKind::Lint => vec![("commit-msg", LINT_HOOK_SCRIPT)],
            HookKind::Generate => vec![("prepare-commit-msg", GENERATE_HOOK_SCRIPT)],
            HookKind::All => vec![
                ("commit-msg", LINT_HOOK_SCRIPT),
                ("prepare-commit-msg", GENERATE_HOOK_SCRIPT),
            ],
        }
    }
}

impl std::fmt::Display for HookKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no skipped variants")
            .get_name()
            .fmt(f)
    }
}

/// Errors from hook install and uninstall operations.
#[derive(Debug, Error)]
pub enum HookError {
    /// A filesystem operation failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// `cocoa hook` was run outside a git repository.
    #[error("no .git/hooks directory found. are you inside a git repository?")]
    NotAGitRepo,

    /// An existing non-cocoa hook was found during uninstall and we have no
    /// backup to restore.
    #[error("the existing {hook_name} hook is not managed by cocoa; remove it manually")]
    NotManagedByCocoa { hook_name: String },
}

/// Outcome reported back to the caller after a successful install of one hook.
#[derive(Debug)]
pub enum InstallOutcome {
    /// Hook was freshly written for the first time.
    Installed { hook_path: PathBuf },

    /// An existing cocoa-managed hook was overwritten in place (idempotent).
    Updated { hook_path: PathBuf },

    /// An existing non-cocoa hook was backed up and replaced.
    Replaced {
        hook_path: PathBuf,
        backup_path: PathBuf,
    },
}

/// Outcome reported back to the caller after a successful uninstall of one
/// hook.
#[derive(Debug)]
pub enum UninstallOutcome {
    /// Hook was removed and a backup was restored in its place.
    Restored {
        hook_path: PathBuf,
        backup_path: PathBuf,
    },

    /// Hook was removed with no backup to restore.
    Removed { hook_path: PathBuf },

    /// No cocoa-managed hook was present; nothing to do.
    NotInstalled,
}

/// Installs cocoa hooks of the given `kind` into `hooks_dir`.
///
/// Each installation is idempotent: if a cocoa-managed hook already exists it
/// is silently overwritten. If a non-cocoa hook exists it is backed up to
/// `<name>.cocoa-backup` before being replaced.
///
/// Returns one [`InstallOutcome`] per hook managed by `kind`. When `dry_run`
/// is `true` the function returns the outcomes that *would* have occurred
/// without writing or modifying any files.
pub fn install(
    hooks_dir: &Path,
    kind: HookKind,
    dry_run: bool,
) -> Result<Vec<InstallOutcome>, HookError> {
    if !hooks_dir.exists() {
        return Err(HookError::NotAGitRepo);
    }

    let mut outcomes = Vec::new();

    for (hook_name, script) in kind.hooks() {
        let hook_path = hooks_dir.join(hook_name);
        let backup_path = hooks_dir.join(format!("{hook_name}{BACKUP_SUFFIX}"));

        let outcome = if hook_path.exists() {
            let existing = fs::read_to_string(&hook_path)?;

            if existing.contains(COCOA_MARKER) {
                // already ours. Safe to overwrite
                if !dry_run {
                    write_hook(&hook_path, script)?;
                }
                InstallOutcome::Updated { hook_path }
            } else {
                // belongs to someone else. Back it up first
                if !dry_run {
                    fs::copy(&hook_path, &backup_path)?;
                    write_hook(&hook_path, script)?;
                }
                InstallOutcome::Replaced {
                    hook_path,
                    backup_path,
                }
            }
        } else {
            if !dry_run {
                // ensure the hooks directory exists (git init may not create it)
                fs::create_dir_all(hooks_dir)?;
                write_hook(&hook_path, script)?;
            }
            InstallOutcome::Installed { hook_path }
        };

        outcomes.push(outcome);
    }

    Ok(outcomes)
}

/// Uninstalls cocoa hooks of the given `kind` from `hooks_dir`.
///
/// For each hook, if a backup created by [`install`] is present it is
/// restored. Otherwise the hook file is removed. Yields
/// [`UninstallOutcome::NotInstalled`] for hooks that are not present.
///
/// Returns an error if any hook file exists but is not managed by cocoa.
/// When `dry_run` is `true` no files are modified.
pub fn uninstall(
    hooks_dir: &Path,
    kind: HookKind,
    dry_run: bool,
) -> Result<Vec<UninstallOutcome>, HookError> {
    if !hooks_dir.exists() {
        return Err(HookError::NotAGitRepo);
    }

    let mut outcomes = Vec::new();

    for (hook_name, _script) in kind.hooks() {
        let hook_path = hooks_dir.join(hook_name);
        let backup_path = hooks_dir.join(format!("{hook_name}{BACKUP_SUFFIX}"));

        if !hook_path.exists() {
            outcomes.push(UninstallOutcome::NotInstalled);
            continue;
        }

        let existing = fs::read_to_string(&hook_path)?;

        if !existing.contains(COCOA_MARKER) {
            return Err(HookError::NotManagedByCocoa {
                hook_name: hook_name.to_string(),
            });
        }

        let outcome = if backup_path.exists() {
            if !dry_run {
                fs::copy(&backup_path, &hook_path)?;
                fs::remove_file(&backup_path)?;
            }
            UninstallOutcome::Restored {
                hook_path,
                backup_path,
            }
        } else {
            if !dry_run {
                fs::remove_file(&hook_path)?;
            }
            UninstallOutcome::Removed { hook_path }
        };

        outcomes.push(outcome);
    }

    Ok(outcomes)
}

/// Writes `script` to `path` and makes it executable.
fn write_hook(path: &Path, script: &str) -> Result<(), HookError> {
    fs::write(path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn make_hooks_dir() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("hooks")).unwrap();
        tmp
    }

    // --- install (lint only) ---

    #[test]
    fn test_install_lint_fresh() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcomes = install(&hooks_dir, HookKind::Lint, false).unwrap();
        assert_eq!(outcomes.len(), 1);

        let hook_path = hooks_dir.join("commit-msg");
        assert!(matches!(outcomes[0], InstallOutcome::Installed { .. }));
        assert!(hook_path.exists());
        let contents = fs::read_to_string(&hook_path).unwrap();
        assert!(contents.contains(COCOA_MARKER));
        assert!(contents.contains("cocoa lint --stdin"));
    }

    #[test]
    fn test_install_lint_idempotent() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        install(&hooks_dir, HookKind::Lint, false).unwrap();
        let outcomes = install(&hooks_dir, HookKind::Lint, false).unwrap();

        assert!(matches!(outcomes[0], InstallOutcome::Updated { .. }));
    }

    #[test]
    fn test_install_lint_backs_up_existing_hook() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");
        let backup_path = hooks_dir.join(format!("commit-msg{}", BACKUP_SUFFIX));

        fs::write(&hook_path, "#!/bin/sh\necho 'existing hook'\n").unwrap();

        let outcomes = install(&hooks_dir, HookKind::Lint, false).unwrap();

        assert!(matches!(outcomes[0], InstallOutcome::Replaced { .. }));
        assert!(backup_path.exists(), "backup should have been created");
        let hook_contents = fs::read_to_string(&hook_path).unwrap();
        assert!(hook_contents.contains(COCOA_MARKER));
    }

    #[test]
    fn test_install_lint_dry_run_writes_nothing() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcomes = install(&hooks_dir, HookKind::Lint, true).unwrap();

        assert!(matches!(outcomes[0], InstallOutcome::Installed { .. }));
        assert!(!hooks_dir.join("commit-msg").exists());
    }

    // --- install (generate only) ---

    #[test]
    fn test_install_generate_fresh() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcomes = install(&hooks_dir, HookKind::Generate, false).unwrap();
        assert_eq!(outcomes.len(), 1);

        let hook_path = hooks_dir.join("prepare-commit-msg");
        assert!(matches!(outcomes[0], InstallOutcome::Installed { .. }));
        assert!(hook_path.exists());
        let contents = fs::read_to_string(&hook_path).unwrap();
        assert!(contents.contains(COCOA_MARKER));
        assert!(contents.contains("cocoa generate --hook"));
    }

    // --- install (all) ---

    #[test]
    fn test_install_all_creates_both_hooks() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcomes = install(&hooks_dir, HookKind::All, false).unwrap();
        assert_eq!(outcomes.len(), 2);

        assert!(hooks_dir.join("commit-msg").exists());
        assert!(hooks_dir.join("prepare-commit-msg").exists());
        assert!(matches!(outcomes[0], InstallOutcome::Installed { .. }));
        assert!(matches!(outcomes[1], InstallOutcome::Installed { .. }));
    }

    #[test]
    fn test_install_all_dry_run_writes_nothing() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        install(&hooks_dir, HookKind::All, true).unwrap();

        assert!(!hooks_dir.join("commit-msg").exists());
        assert!(!hooks_dir.join("prepare-commit-msg").exists());
    }

    #[test]
    fn test_install_no_hooks_dir() {
        let tmp = TempDir::new().unwrap();
        let hooks_dir = tmp.path().join("nonexistent");

        let result = install(&hooks_dir, HookKind::Lint, false);
        assert!(matches!(result, Err(HookError::NotAGitRepo)));
    }

    // --- uninstall (lint only) ---

    #[test]
    fn test_uninstall_lint_removes_hook() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");

        install(&hooks_dir, HookKind::Lint, false).unwrap();
        assert!(hook_path.exists());

        let outcomes = uninstall(&hooks_dir, HookKind::Lint, false).unwrap();

        assert!(matches!(outcomes[0], UninstallOutcome::Removed { .. }));
        assert!(!hook_path.exists());
    }

    #[test]
    fn test_uninstall_lint_restores_backup() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");
        let backup_path = hooks_dir.join(format!("commit-msg{}", BACKUP_SUFFIX));

        let original = "#!/bin/sh\necho 'original hook'\n";
        fs::write(&hook_path, original).unwrap();
        install(&hooks_dir, HookKind::Lint, false).unwrap();
        assert!(backup_path.exists());

        let outcomes = uninstall(&hooks_dir, HookKind::Lint, false).unwrap();

        assert!(matches!(outcomes[0], UninstallOutcome::Restored { .. }));
        assert!(!backup_path.exists());
        let restored = fs::read_to_string(&hook_path).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_uninstall_lint_not_installed() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcomes = uninstall(&hooks_dir, HookKind::Lint, false).unwrap();
        assert!(matches!(outcomes[0], UninstallOutcome::NotInstalled));
    }

    #[test]
    fn test_uninstall_lint_refuses_non_cocoa_hook() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");

        fs::write(&hook_path, "#!/bin/sh\necho 'not cocoa'\n").unwrap();

        let result = uninstall(&hooks_dir, HookKind::Lint, false);
        assert!(matches!(result, Err(HookError::NotManagedByCocoa { .. })));
    }

    #[test]
    fn test_uninstall_lint_dry_run_changes_nothing() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");

        install(&hooks_dir, HookKind::Lint, false).unwrap();
        assert!(hook_path.exists());

        let outcomes = uninstall(&hooks_dir, HookKind::Lint, true).unwrap();

        assert!(matches!(outcomes[0], UninstallOutcome::Removed { .. }));
        assert!(hook_path.exists(), "dry-run must not remove files");
    }

    // --- uninstall (all) ---

    #[test]
    fn test_uninstall_all_removes_both_hooks() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        install(&hooks_dir, HookKind::All, false).unwrap();
        assert!(hooks_dir.join("commit-msg").exists());
        assert!(hooks_dir.join("prepare-commit-msg").exists());

        let outcomes = uninstall(&hooks_dir, HookKind::All, false).unwrap();

        assert_eq!(outcomes.len(), 2);
        assert!(!hooks_dir.join("commit-msg").exists());
        assert!(!hooks_dir.join("prepare-commit-msg").exists());
    }

    #[test]
    fn test_uninstall_all_not_installed_returns_not_installed_for_each() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcomes = uninstall(&hooks_dir, HookKind::All, false).unwrap();

        assert_eq!(outcomes.len(), 2);
        assert!(
            outcomes
                .iter()
                .all(|o| matches!(o, UninstallOutcome::NotInstalled))
        );
    }

    #[test]
    fn test_uninstall_lint_only_leaves_generate_hook() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        install(&hooks_dir, HookKind::All, false).unwrap();
        uninstall(&hooks_dir, HookKind::Lint, false).unwrap();

        assert!(!hooks_dir.join("commit-msg").exists());
        assert!(
            hooks_dir.join("prepare-commit-msg").exists(),
            "generate hook should remain"
        );
    }

    // --- hook script content ---

    #[test]
    fn test_hook_script_is_executable_after_install() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let tmp = make_hooks_dir();
            let hooks_dir = tmp.path().join("hooks");

            install(&hooks_dir, HookKind::All, false).unwrap();

            for hook_name in ["commit-msg", "prepare-commit-msg"] {
                let hook_path = hooks_dir.join(hook_name);
                let mode = fs::metadata(&hook_path).unwrap().permissions().mode();
                assert_ne!(mode & 0o100, 0, "{hook_name} must be executable");
            }
        }
    }

    #[test]
    fn test_generate_hook_skips_known_message_sources() {
        // verify the script content contains the case statement guard
        assert!(
            GENERATE_HOOK_SCRIPT.contains("message|merge|squash|commit"),
            "generate hook must skip amend/merge/squash/-m sources"
        );
    }
}
