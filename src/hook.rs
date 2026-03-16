//! Git hook management for cocoa.
//!
//! Implements `cocoa hook` and `cocoa unhook`, which install and remove a
//! `commit-msg` git hook that pipes the commit message through `cocoa lint`.

use std::{
    fs,
    path::{Path, PathBuf},
};

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
const HOOK_SCRIPT: &str =
    "#!/bin/sh\n# managed by cocoa - do not edit\ncocoa lint --stdin < \"$1\"\n";

/// Name of the backup file saved when a pre-existing non-cocoa hook is found.
const BACKUP_SUFFIX: &str = ".cocoa-backup";

/// Errors from hook install and uninstall operations.
#[derive(Debug, Error)]
pub enum HookError {
    /// A filesystem operation failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// `cocoa hook` was run outside a git repository.
    #[error("no .git/hooks directory found — are you inside a git repository?")]
    NotAGitRepo,

    /// An existing non-cocoa hook was found during uninstall and we have no
    /// backup to restore.
    #[error("the existing commit-msg hook is not managed by cocoa; remove it manually")]
    NotManagedByCocoa,
}

/// Outcome reported back to the caller after a successful install.
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

/// Outcome reported back to the caller after a successful uninstall.
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

/// Installs the cocoa `commit-msg` hook into `hooks_dir`.
///
/// The installation is idempotent: if a cocoa-managed hook already exists it
/// is silently overwritten. If a non-cocoa hook exists it is backed up to
/// `<name>.cocoa-backup` before being replaced.
///
/// When `dry_run` is `true` the function returns the outcome that *would* have
/// occurred without writing or modifying any files.
pub fn install(hooks_dir: &Path, dry_run: bool) -> Result<InstallOutcome, HookError> {
    if !hooks_dir.exists() {
        return Err(HookError::NotAGitRepo);
    }

    let hook_path = hooks_dir.join("commit-msg");
    let backup_path = hooks_dir.join(format!("commit-msg{}", BACKUP_SUFFIX));

    let outcome = if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;

        if existing.contains(COCOA_MARKER) {
            // already ours — safe to overwrite
            if !dry_run {
                write_hook(&hook_path)?;
            }
            InstallOutcome::Updated {
                hook_path: hook_path.clone(),
            }
        } else {
            // belongs to someone else — back it up first
            if !dry_run {
                fs::copy(&hook_path, &backup_path)?;
                write_hook(&hook_path)?;
            }
            InstallOutcome::Replaced {
                hook_path: hook_path.clone(),
                backup_path: backup_path.clone(),
            }
        }
    } else {
        if !dry_run {
            // ensure the hooks directory exists (git init may not create it)
            fs::create_dir_all(hooks_dir)?;
            write_hook(&hook_path)?;
        }
        InstallOutcome::Installed {
            hook_path: hook_path.clone(),
        }
    };

    Ok(outcome)
}

/// Uninstalls the cocoa `commit-msg` hook from `hooks_dir`.
///
/// If a backup created by `install` is present it is restored. Otherwise the
/// hook file is removed. Returns [`UninstallOutcome::NotInstalled`] when no
/// cocoa-managed hook is found.
///
/// When `dry_run` is `true` no files are modified.
pub fn uninstall(hooks_dir: &Path, dry_run: bool) -> Result<UninstallOutcome, HookError> {
    if !hooks_dir.exists() {
        return Err(HookError::NotAGitRepo);
    }

    let hook_path = hooks_dir.join("commit-msg");
    let backup_path = hooks_dir.join(format!("commit-msg{}", BACKUP_SUFFIX));

    if !hook_path.exists() {
        return Ok(UninstallOutcome::NotInstalled);
    }

    let existing = fs::read_to_string(&hook_path)?;

    if !existing.contains(COCOA_MARKER) {
        return Err(HookError::NotManagedByCocoa);
    }

    let outcome = if backup_path.exists() {
        if !dry_run {
            fs::copy(&backup_path, &hook_path)?;
            fs::remove_file(&backup_path)?;
        }
        UninstallOutcome::Restored {
            hook_path: hook_path.clone(),
            backup_path: backup_path.clone(),
        }
    } else {
        if !dry_run {
            fs::remove_file(&hook_path)?;
        }
        UninstallOutcome::Removed {
            hook_path: hook_path.clone(),
        }
    };

    Ok(outcome)
}

/// Writes [`HOOK_SCRIPT`] to `path` and makes it executable.
fn write_hook(path: &Path) -> Result<(), HookError> {
    fs::write(path, HOOK_SCRIPT)?;

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

    // --- install ---

    #[test]
    fn test_install_fresh() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcome = install(&hooks_dir, false).unwrap();

        let hook_path = hooks_dir.join("commit-msg");
        assert!(
            matches!(outcome, InstallOutcome::Installed { .. }),
            "expected Installed"
        );
        assert!(hook_path.exists());
        let contents = fs::read_to_string(&hook_path).unwrap();
        assert!(contents.contains(COCOA_MARKER));
        assert!(contents.contains("cocoa lint --stdin"));
    }

    #[test]
    fn test_install_idempotent() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        install(&hooks_dir, false).unwrap();
        let outcome = install(&hooks_dir, false).unwrap();

        assert!(
            matches!(outcome, InstallOutcome::Updated { .. }),
            "second install should report Updated"
        );
    }

    #[test]
    fn test_install_backs_up_existing_hook() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");
        let backup_path = hooks_dir.join(format!("commit-msg{}", BACKUP_SUFFIX));

        fs::write(&hook_path, "#!/bin/sh\necho 'existing hook'\n").unwrap();

        let outcome = install(&hooks_dir, false).unwrap();

        assert!(
            matches!(outcome, InstallOutcome::Replaced { .. }),
            "expected Replaced"
        );
        assert!(backup_path.exists(), "backup should have been created");
        let hook_contents = fs::read_to_string(&hook_path).unwrap();
        assert!(hook_contents.contains(COCOA_MARKER));
    }

    #[test]
    fn test_install_dry_run_writes_nothing() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcome = install(&hooks_dir, true).unwrap();

        assert!(
            matches!(outcome, InstallOutcome::Installed { .. }),
            "expected Installed outcome even in dry-run"
        );
        assert!(
            !hooks_dir.join("commit-msg").exists(),
            "dry-run must not write files"
        );
    }

    #[test]
    fn test_install_no_hooks_dir() {
        let tmp = TempDir::new().unwrap();
        let hooks_dir = tmp.path().join("nonexistent");

        // directory does not exist — should fail
        let result = install(&hooks_dir, false);
        assert!(matches!(result, Err(HookError::NotAGitRepo)));
    }

    // --- uninstall ---

    #[test]
    fn test_uninstall_removes_hook() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");

        install(&hooks_dir, false).unwrap();
        assert!(hook_path.exists());

        let outcome = uninstall(&hooks_dir, false).unwrap();

        assert!(
            matches!(outcome, UninstallOutcome::Removed { .. }),
            "expected Removed"
        );
        assert!(!hook_path.exists());
    }

    #[test]
    fn test_uninstall_restores_backup() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");
        let backup_path = hooks_dir.join(format!("commit-msg{}", BACKUP_SUFFIX));

        let original = "#!/bin/sh\necho 'original hook'\n";
        fs::write(&hook_path, original).unwrap();
        install(&hooks_dir, false).unwrap();
        assert!(backup_path.exists(), "backup should exist after install");

        let outcome = uninstall(&hooks_dir, false).unwrap();

        assert!(
            matches!(outcome, UninstallOutcome::Restored { .. }),
            "expected Restored"
        );
        assert!(!backup_path.exists(), "backup should be gone after restore");
        let restored = fs::read_to_string(&hook_path).unwrap();
        assert_eq!(restored, original, "original hook should be restored");
    }

    #[test]
    fn test_uninstall_not_installed() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");

        let outcome = uninstall(&hooks_dir, false).unwrap();
        assert!(matches!(outcome, UninstallOutcome::NotInstalled));
    }

    #[test]
    fn test_uninstall_refuses_non_cocoa_hook() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");

        fs::write(&hook_path, "#!/bin/sh\necho 'not cocoa'\n").unwrap();

        let result = uninstall(&hooks_dir, false);
        assert!(matches!(result, Err(HookError::NotManagedByCocoa)));
    }

    #[test]
    fn test_uninstall_dry_run_changes_nothing() {
        let tmp = make_hooks_dir();
        let hooks_dir = tmp.path().join("hooks");
        let hook_path = hooks_dir.join("commit-msg");

        install(&hooks_dir, false).unwrap();
        assert!(hook_path.exists());

        let outcome = uninstall(&hooks_dir, true).unwrap();

        assert!(
            matches!(outcome, UninstallOutcome::Removed { .. }),
            "should report what would happen"
        );
        assert!(hook_path.exists(), "dry-run must not remove files");
    }

    // --- hook script content ---

    #[test]
    fn test_hook_script_is_executable_after_install() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let tmp = make_hooks_dir();
            let hooks_dir = tmp.path().join("hooks");

            install(&hooks_dir, false).unwrap();

            let hook_path = hooks_dir.join("commit-msg");
            let mode = fs::metadata(&hook_path).unwrap().permissions().mode();
            // check owner execute bit (0o100)
            assert_ne!(mode & 0o100, 0, "hook must be executable");
        }
    }
}
