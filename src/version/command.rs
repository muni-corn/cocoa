//! Command-based version update strategy.
//!
//! When `strategy = "command"` is set for a `[[version.files]]` entry, cocoa
//! shells out to the configured command instead of editing the file in-process.
//! This is useful for lockfiles like `pnpm-lock.yaml` or `yarn.lock` whose
//! formats are complex enough that in-process editing would be fragile.
//!
//! # How it works
//!
//! 1. Snapshot the current file content for rollback purposes.
//! 2. Run the configured command in the repository root.
//! 3. Re-read the file to produce the `PendingUpdate` record (the "updated"
//!    bytes are what the command left on disk; we store them so that the
//!    `apply_updates` rollback mechanism can restore the original on failure).
//!
//! Because the file is already written by the command, the `apply_updates`
//! write for this entry is a no-op (original == updated only if the command
//! made no changes, which is unusual). On error in a later entry, the rollback
//! path correctly restores the original bytes.
//!
//! # Configuration example
//! ```toml
//! [[version.files]]
//! path = "Cargo.lock"
//! kind = "cargo-lock"
//! strategy = "command"
//! command = ["cargo", "update", "--workspace"]
//! ```

use std::{
    path::Path,
    process::{Command, Stdio},
};

use crate::version::{
    FileKind, UpdatedFile, VersionError,
    handlers::{PendingUpdate, read_bytes},
};

/// Run a configured command as a version-update strategy.
///
/// Returns a `PendingUpdate` whose `updated` bytes reflect the file state
/// after the command ran. This allows the two-phase atomicity guarantee in
/// `apply_updates` to roll back the file on subsequent failures.
pub fn run_command(
    path: &str,
    command: &[String],
    kind: FileKind,
    repo_root: Option<&str>,
) -> Result<PendingUpdate, VersionError> {
    if command.is_empty() {
        return Err(VersionError::ToolchainNotFound {
            tool: "(empty)".to_owned(),
            hint: "set a non-empty command for strategy = \"command\"".to_owned(),
        });
    }

    // snapshot original content for rollback
    let original = read_bytes(path)?;

    let exe = &command[0];
    let args = &command[1..];

    // resolve the working directory: use repo_root if given, else the file's
    // parent directory, else the current directory
    let work_dir = repo_root.map(|r| r.to_owned()).unwrap_or_else(|| {
        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".to_owned())
    });

    let output = Command::new(exe)
        .args(args)
        .current_dir(&work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                VersionError::ToolchainNotFound {
                    tool: exe.clone(),
                    hint: format!(
                        "install the toolchain or set `strategy = \"in-process\"` (or `\"skip\"`) \
                         for '{path}'"
                    ),
                }
            } else {
                VersionError::File {
                    path: path.to_owned(),
                    source: e,
                }
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(VersionError::ToolchainFailed {
            command: command.join(" "),
            status: output.status.code().unwrap_or(-1),
            stderr,
        });
    }

    // re-read the file to capture whatever the command wrote
    let updated = read_bytes(path)?;

    Ok(PendingUpdate {
        path: path.to_owned(),
        original,
        updated,
        updated_file: UpdatedFile {
            path: path.to_owned(),
            kind,
            // command-driven updates report 0 textual replacements; callers
            // should treat this as "modified by external tool"
            replacements: 0,
        },
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write_tmp(content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, content).unwrap();
        (dir, path.to_string_lossy().into_owned())
    }

    #[test]
    fn test_command_snapshots_and_returns_pending_update() {
        let (_dir, path) = write_tmp("original content\n");

        // use `echo` (always available) to write something to stdout — but we
        // need a command that modifies the file; use `touch` which is a no-op
        // on content but succeeds.
        let command = vec!["touch".to_owned(), path.clone()];
        let update = run_command(&path, &command, FileKind::Plain, None).unwrap();

        // the original bytes should be captured
        assert_eq!(update.original, b"original content\n");
        // replacements is 0 for command-driven updates
        assert_eq!(update.updated_file.replacements, 0);
    }

    #[test]
    fn test_command_missing_exe_returns_toolchain_not_found() {
        let (_dir, path) = write_tmp("x\n");
        let command = vec!["this_command_does_not_exist_xyz".to_owned()];
        let err = run_command(&path, &command, FileKind::Plain, None).unwrap_err();
        assert!(matches!(err, VersionError::ToolchainNotFound { .. }));
    }

    #[test]
    fn test_command_nonzero_exit_returns_toolchain_failed() {
        let (_dir, path) = write_tmp("x\n");
        // `false` exits with code 1
        let command = vec!["false".to_owned()];
        let err = run_command(&path, &command, FileKind::Plain, None).unwrap_err();
        assert!(matches!(err, VersionError::ToolchainFailed { .. }));
    }

    #[test]
    fn test_empty_command_returns_toolchain_not_found() {
        let (_dir, path) = write_tmp("x\n");
        let command: Vec<String> = vec![];
        let err = run_command(&path, &command, FileKind::Plain, None).unwrap_err();
        assert!(matches!(err, VersionError::ToolchainNotFound { .. }));
    }

    #[test]
    fn test_command_missing_file_returns_file_error() {
        let command = vec!["touch".to_owned(), "/nonexistent/path.txt".to_owned()];
        let err =
            run_command("/nonexistent/path.txt", &command, FileKind::Plain, None).unwrap_err();
        assert!(matches!(err, VersionError::File { .. }));
    }
}
