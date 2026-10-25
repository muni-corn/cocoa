//! Dry-run planning for version file updates.

use super::{UpdateOptions, dispatch};
use crate::{
    config::{FileEntryStrategy, VersionFileEntry},
    version::{UpdatedFile, VersionError},
};

/// Work out what
/// [`update_version_files_rich`](super::update_version_files_rich)
/// would do, without writing anything.
///
/// In-process handlers run their full checks, so problems such as a missing
/// file or a version mismatch surface here exactly as they would in a real
/// run. Entries that shell out to a package manager are listed with zero
/// replacements but never executed, because running them would change files.
/// Entries with the `skip` strategy, and files a handler leaves unchanged, are
/// left out.
pub fn plan_version_files(
    entries: &[VersionFileEntry],
    old_version: &str,
    new_version: &str,
    options: &UpdateOptions,
) -> Result<Vec<UpdatedFile>, VersionError> {
    let mut planned = Vec::new();

    for entry in entries {
        if entry.strategy == FileEntryStrategy::Skip {
            continue;
        }

        let toolchain_entry = || UpdatedFile {
            path: entry.path.clone(),
            kind: super::entry_kind_to_file_kind(&dispatch::effective_kind(entry)),
            replacements: 0,
        };

        if entry.strategy == FileEntryStrategy::Command {
            planned.push(toolchain_entry());
            continue;
        }

        match dispatch::prepare_in_process(entry, old_version, new_version, options)? {
            dispatch::Dispatch::Prepared(Some(update)) => planned.push(update.updated_file),
            dispatch::Dispatch::Prepared(None) => {}
            dispatch::Dispatch::Toolchain => planned.push(toolchain_entry()),
        }
    }

    Ok(planned)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::{config::FileEntryKind, version::FileKind};

    fn entry(path: &str, kind: FileEntryKind, strategy: FileEntryStrategy) -> VersionFileEntry {
        VersionFileEntry {
            path: path.to_string(),
            kind,
            strategy,
            command: Some(vec!["definitely-not-a-real-command".to_string()]),
            pattern: None,
            occurrences: None,
        }
    }

    fn manifest(dir: &std::path::Path, version: &str) -> String {
        let path = dir.join("Cargo.toml");
        fs::write(
            &path,
            format!("[package]\nname = \"demo\"\nversion = \"{version}\"\n"),
        )
        .unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn test_plan_reports_handler_kind_and_replacements_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = manifest(dir.path(), "1.0.0");
        let entries = [entry(
            &path,
            FileEntryKind::Auto,
            FileEntryStrategy::InProcess,
        )];

        let planned =
            plan_version_files(&entries, "1.0.0", "1.1.0", &UpdateOptions::default()).unwrap();

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].kind, FileKind::CargoManifest);
        assert_eq!(planned[0].replacements, 1);
        assert!(fs::read_to_string(&path).unwrap().contains("\"1.0.0\""));
    }

    #[test]
    fn test_plan_surfaces_version_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = manifest(dir.path(), "0.5.1");
        let entries = [entry(
            &path,
            FileEntryKind::Auto,
            FileEntryStrategy::InProcess,
        )];

        let err =
            plan_version_files(&entries, "0.5.3", "0.5.4", &UpdateOptions::default()).unwrap_err();
        assert!(matches!(err, VersionError::VersionMismatch { .. }));
    }

    #[test]
    fn test_plan_surfaces_missing_file() {
        let entries = [entry(
            "/nonexistent/version.txt",
            FileEntryKind::Plain,
            FileEntryStrategy::InProcess,
        )];
        let err =
            plan_version_files(&entries, "1.0.0", "2.0.0", &UpdateOptions::default()).unwrap_err();
        assert!(matches!(err, VersionError::File { .. }));
    }

    #[test]
    fn test_plan_lists_command_entries_without_running_them() {
        // the command does not exist, so running it would fail
        let entries = [entry(
            "Cargo.lock",
            FileEntryKind::CargoLock,
            FileEntryStrategy::Command,
        )];
        let planned =
            plan_version_files(&entries, "1.0.0", "2.0.0", &UpdateOptions::default()).unwrap();

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].kind, FileKind::CargoLock);
        assert_eq!(planned[0].replacements, 0);
    }

    #[test]
    fn test_plan_lists_toolchain_kinds_without_running_them() {
        let entries = [entry(
            "pnpm-lock.yaml",
            FileEntryKind::PnpmLock,
            FileEntryStrategy::InProcess,
        )];
        let planned =
            plan_version_files(&entries, "1.0.0", "2.0.0", &UpdateOptions::default()).unwrap();

        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].kind, FileKind::PnpmLock);
    }

    #[test]
    fn test_plan_leaves_out_skipped_entries() {
        let entries = [entry(
            "whatever.txt",
            FileEntryKind::Plain,
            FileEntryStrategy::Skip,
        )];
        let planned =
            plan_version_files(&entries, "1.0.0", "2.0.0", &UpdateOptions::default()).unwrap();
        assert!(planned.is_empty());
    }
}
