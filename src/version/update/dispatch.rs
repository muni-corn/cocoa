//! Handler selection for version file entries.
//!
//! Maps a `[[version.files]]` entry to the in-process handler that prepares
//! its update. Shared by real updates and dry-run planning so both pick the
//! same handler with the same options.

use super::UpdateOptions;
use crate::{
    config::{FileEntryKind, Occurrences, OccurrencesNamed, VersionFileEntry},
    version::{
        VersionError,
        cargo_lock::CargoLockHandler,
        cargo_manifest::CargoManifestHandler,
        detect::infer_kind,
        handlers::{Handler, PendingUpdate},
        npm::{NpmLockHandler, NpmManifestHandler},
        plain::PlainHandler,
        pyproject::PyprojectHandler,
        regex_handler::RegexHandler,
    },
};

/// The result of asking for an entry's in-process handler.
#[derive(Debug)]
pub enum Dispatch {
    /// An in-process handler ran; `None` means the file needs no change.
    Prepared(Option<PendingUpdate>),
    /// The kind has no in-process handler because it shells out to a package
    /// manager (pnpm and yarn lockfiles).
    Toolchain,
}

/// Resolve an entry's kind, inferring it from the file name when set to auto.
pub fn effective_kind(entry: &VersionFileEntry) -> FileEntryKind {
    if entry.kind == FileEntryKind::Auto {
        infer_kind(&entry.path)
    } else {
        entry.kind.clone()
    }
}

/// Prepare an entry's update with its in-process handler, without writing.
///
/// Returns [`Dispatch::Toolchain`] for kinds that can only be updated by
/// running a package manager.
pub fn prepare_in_process(
    entry: &VersionFileEntry,
    old_version: &str,
    new_version: &str,
    options: &UpdateOptions,
) -> Result<Dispatch, VersionError> {
    let path = entry.path.as_str();

    let prepared = match effective_kind(entry) {
        FileEntryKind::Cargo => CargoManifestHandler {
            verify_old_version: options.verify_old_version,
        }
        .prepare(path, old_version, new_version)?,
        FileEntryKind::CargoLock => CargoLockHandler {
            verify_old_version: options.verify_old_version,
            ..Default::default()
        }
        .prepare(path, old_version, new_version)?,
        FileEntryKind::Npm => NpmManifestHandler.prepare(path, old_version, new_version)?,
        FileEntryKind::NpmLock => NpmLockHandler.prepare(path, old_version, new_version)?,
        FileEntryKind::Regex => {
            let pattern = entry.pattern.as_deref().unwrap_or("");
            let occurrences = entry
                .occurrences
                .clone()
                .unwrap_or(Occurrences::Named(OccurrencesNamed::First));
            RegexHandler::new(path, pattern, occurrences)?.prepare(
                path,
                old_version,
                new_version,
            )?
        }
        // plain: use configured occurrences (default: all)
        FileEntryKind::Plain => {
            let occurrences = entry
                .occurrences
                .clone()
                .unwrap_or(Occurrences::Named(OccurrencesNamed::All));
            PlainHandler { occurrences }.prepare(path, old_version, new_version)?
        }
        FileEntryKind::Pyproject => PyprojectHandler.prepare(path, old_version, new_version)?,
        // pnpm and yarn lockfiles are too complex to edit in-process
        FileEntryKind::PnpmLock | FileEntryKind::YarnLock => return Ok(Dispatch::Toolchain),
        // auto is resolved by `effective_kind`; fall back to plain defensively
        FileEntryKind::Auto => PlainHandler::default().prepare(path, old_version, new_version)?,
    };

    Ok(Dispatch::Prepared(prepared))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::config::FileEntryStrategy;

    fn entry(path: &str, kind: FileEntryKind) -> VersionFileEntry {
        VersionFileEntry {
            path: path.to_string(),
            kind,
            strategy: FileEntryStrategy::InProcess,
            command: None,
            pattern: None,
            occurrences: None,
        }
    }

    #[test]
    fn test_effective_kind_infers_from_file_name() {
        assert_eq!(
            effective_kind(&entry("pkg/Cargo.toml", FileEntryKind::Auto)),
            FileEntryKind::Cargo
        );
    }

    #[test]
    fn test_effective_kind_keeps_explicit_kind() {
        assert_eq!(
            effective_kind(&entry("Cargo.toml", FileEntryKind::Plain)),
            FileEntryKind::Plain
        );
    }

    #[test]
    fn test_prepare_in_process_returns_pending_update() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("version.txt");
        fs::write(&path, "1.0.0\n").unwrap();

        let e = entry(path.to_str().unwrap(), FileEntryKind::Plain);
        let result = prepare_in_process(&e, "1.0.0", "2.0.0", &UpdateOptions::default()).unwrap();

        let Dispatch::Prepared(Some(update)) = result else {
            panic!("expected a prepared update, got {result:?}");
        };
        assert_eq!(update.updated, b"2.0.0\n");
        // preparing must never write
        assert_eq!(fs::read_to_string(&path).unwrap(), "1.0.0\n");
    }

    #[test]
    fn test_prepare_in_process_reports_toolchain_kinds() {
        let e = entry("pnpm-lock.yaml", FileEntryKind::PnpmLock);
        let result = prepare_in_process(&e, "1.0.0", "2.0.0", &UpdateOptions::default()).unwrap();
        assert!(matches!(result, Dispatch::Toolchain));
    }
}
