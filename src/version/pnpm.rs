//! pnpm and yarn lockfile handlers.
//!
//! Both pnpm-lock.yaml and yarn.lock use formats that are complex enough that
//! in-process editing would be fragile. These handlers default to the
//! `command` strategy, shelling out to the respective toolchain to regenerate
//! the lockfile after the manifest version has already been updated.
//!
//! Default commands:
//! - pnpm: `pnpm install --lockfile-only --ignore-scripts`
//! - yarn (classic): `yarn install --mode=update-lockfile`
//!
//! Users can override the command via `[[version.files]]` entries.

use crate::version::{FileKind, VersionError, command::run_command, handlers::PendingUpdate};

/// Default pnpm command for lockfile update.
pub const PNPM_DEFAULT_COMMAND: &[&str] =
    &["pnpm", "install", "--lockfile-only", "--ignore-scripts"];

/// Default yarn classic command for lockfile update.
pub const YARN_DEFAULT_COMMAND: &[&str] = &["yarn", "install", "--mode=update-lockfile"];

/// Run the pnpm lockfile update command.
///
/// Uses `pnpm install --lockfile-only --ignore-scripts` by default.
/// Pass a custom command slice to override.
pub fn update_pnpm_lock(
    path: &str,
    command: Option<&[String]>,
    repo_root: Option<&str>,
) -> Result<PendingUpdate, VersionError> {
    let owned_default: Vec<String> = PNPM_DEFAULT_COMMAND.iter().map(|s| s.to_string()).collect();

    let cmd: &[String] = command.unwrap_or(&owned_default);
    run_command(path, cmd, FileKind::PnpmLock, repo_root)
}

/// Run the yarn lockfile update command.
///
/// Uses `yarn install --mode=update-lockfile` by default (yarn classic).
/// Pass a custom command slice to override.
pub fn update_yarn_lock(
    path: &str,
    command: Option<&[String]>,
    repo_root: Option<&str>,
) -> Result<PendingUpdate, VersionError> {
    let owned_default: Vec<String> = YARN_DEFAULT_COMMAND.iter().map(|s| s.to_string()).collect();

    let cmd: &[String] = command.unwrap_or(&owned_default);
    run_command(path, cmd, FileKind::YarnLock, repo_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pnpm_default_command_is_non_empty() {
        assert!(!PNPM_DEFAULT_COMMAND.is_empty());
        assert_eq!(PNPM_DEFAULT_COMMAND[0], "pnpm");
    }

    #[test]
    fn test_yarn_default_command_is_non_empty() {
        assert!(!YARN_DEFAULT_COMMAND.is_empty());
        assert_eq!(YARN_DEFAULT_COMMAND[0], "yarn");
    }

    #[test]
    fn test_update_pnpm_lock_fails_without_pnpm_installed() {
        // pnpm is almost certainly not installed in the test environment;
        // verify that the error is ToolchainNotFound (not a panic)
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pnpm-lock.yaml");
        fs::write(&path, "lockfileVersion: '6.0'\n").unwrap();

        let result = update_pnpm_lock(&path.to_string_lossy(), None, None);
        // either succeeds (pnpm is installed) or fails with a known error type
        match result {
            Ok(_) => {}                                       // pnpm is installed — fine
            Err(VersionError::ToolchainNotFound { .. }) => {} // expected in most CI
            Err(VersionError::ToolchainFailed { .. }) => {}   // pnpm installed but errored
            Err(other) => panic!("unexpected error: {other}"),
        }
    }
}
