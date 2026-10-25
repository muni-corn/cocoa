//! Version file updates.
//!
//! Applies a version change to the files listed in the release configuration,
//! choosing a handler for each file and writing all of them atomically.

pub mod options;

pub use options::UpdateOptions;

use super::{FileKind, UpdatedFile, VersionError};

/// Update the version string in each of the given files atomically.
///
/// Reads every file first, replaces the version string using the appropriate
/// handler for each file, then writes them all. If any write fails the
/// already-written files are restored to their original content, so the set of
/// files is either fully updated or left completely unchanged.
///
/// Handler selection is automatic by file basename:
/// - `Cargo.toml` → [`cargo_manifest::CargoManifestHandler`]
/// - `Cargo.lock` → [`cargo_lock::CargoLockHandler`]
/// - `package.json` → [`npm::NpmManifestHandler`]
/// - `package-lock.json` → [`npm::NpmLockHandler`]
/// - All other files → [`plain::PlainHandler`] (historical behavior)
///
/// Returns `VersionError::NotFound` for plain-text files that do not contain
/// `old_version`.
pub fn update_version_files(
    files: &[String],
    old_version: &str,
    new_version: &str,
) -> Result<(), VersionError> {
    use crate::config::{FileEntryKind, VersionFileEntry};

    // convert the flat list into rich entries so we get auto-detection
    let entries: Vec<VersionFileEntry> = files
        .iter()
        .map(|p| VersionFileEntry {
            path: p.clone(),
            kind: FileEntryKind::Auto,
            strategy: crate::config::FileEntryStrategy::InProcess,
            command: None,
            pattern: None,
            occurrences: None,
        })
        .collect();

    update_version_files_rich(&entries, old_version, new_version)?;
    Ok(())
}

/// Update version files using rich per-file entries from `[[version.files]]`.
///
/// Each entry specifies a path and an optional handler kind, strategy, and
/// extra options. Falls back to [`update_version_files`] for entries whose
/// kind resolves to `Plain`.
///
/// Returns a list of [`UpdatedFile`] records describing each update applied.
pub fn update_version_files_rich(
    entries: &[crate::config::VersionFileEntry],
    old_version: &str,
    new_version: &str,
) -> Result<Vec<UpdatedFile>, VersionError> {
    use super::{
        cargo_lock::CargoLockHandler,
        cargo_manifest::CargoManifestHandler,
        command::run_command,
        detect::infer_kind,
        handlers::{Handler, apply_updates},
        npm::{NpmLockHandler, NpmManifestHandler},
        plain::PlainHandler,
        pnpm::{update_pnpm_lock, update_yarn_lock},
        pyproject::PyprojectHandler,
        regex_handler::RegexHandler,
    };
    use crate::config::{FileEntryKind, FileEntryStrategy, Occurrences, OccurrencesNamed};

    let mut pending = Vec::new();

    for entry in entries {
        if entry.strategy == FileEntryStrategy::Skip {
            continue;
        }

        if entry.strategy == FileEntryStrategy::Command {
            // resolve the FileKind from the entry kind so the UpdatedFile
            // record carries the correct variant
            let effective_kind = if entry.kind == FileEntryKind::Auto {
                infer_kind(&entry.path)
            } else {
                entry.kind.clone()
            };
            let kind = entry_kind_to_file_kind(&effective_kind);
            let cmd = entry.command.as_deref().unwrap_or(&[]);
            let update = run_command(&entry.path, cmd, kind, None)?;
            pending.push(update);
            continue;
        }

        // for in-process strategy, resolve auto kind from file basename
        let effective_kind = if entry.kind == FileEntryKind::Auto {
            infer_kind(&entry.path)
        } else {
            entry.kind.clone()
        };

        let handler_result = match &effective_kind {
            FileEntryKind::Cargo => {
                CargoManifestHandler.prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::CargoLock => {
                CargoLockHandler::default().prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::Npm => {
                NpmManifestHandler.prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::NpmLock => {
                NpmLockHandler.prepare(&entry.path, old_version, new_version)?
            }
            FileEntryKind::Regex => {
                let pattern = entry.pattern.as_deref().unwrap_or("");
                let occurrences = entry
                    .occurrences
                    .clone()
                    .unwrap_or(Occurrences::Named(OccurrencesNamed::First));
                RegexHandler::new(&entry.path, pattern, occurrences)?.prepare(
                    &entry.path,
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
                PlainHandler { occurrences }.prepare(&entry.path, old_version, new_version)?
            }
            // pnpm-lock and yarn-lock default to the command strategy since
            // their file formats are complex; route them here when the user
            // has chosen in-process (which is an explicit override)
            FileEntryKind::PnpmLock => {
                let cmd = entry.command.as_deref();
                let update = update_pnpm_lock(&entry.path, cmd, None)?;
                pending.push(update);
                continue;
            }
            FileEntryKind::YarnLock => {
                let cmd = entry.command.as_deref();
                let update = update_yarn_lock(&entry.path, cmd, None)?;
                pending.push(update);
                continue;
            }
            FileEntryKind::Pyproject => {
                PyprojectHandler.prepare(&entry.path, old_version, new_version)?
            }
            // auto should never reach here (already resolved above), but
            // handle it defensively with plain
            FileEntryKind::Auto => {
                PlainHandler::default().prepare(&entry.path, old_version, new_version)?
            }
        };

        if let Some(update) = handler_result {
            pending.push(update);
        }
    }

    apply_updates(pending)
}

/// Map a `FileEntryKind` config value to the corresponding `FileKind` enum.
fn entry_kind_to_file_kind(kind: &crate::config::FileEntryKind) -> FileKind {
    use crate::config::FileEntryKind;
    match kind {
        FileEntryKind::Cargo => FileKind::CargoManifest,
        FileEntryKind::CargoLock => FileKind::CargoLock,
        FileEntryKind::Npm => FileKind::NpmManifest,
        FileEntryKind::NpmLock => FileKind::NpmLock,
        FileEntryKind::PnpmLock => FileKind::PnpmLock,
        FileEntryKind::YarnLock => FileKind::YarnLock,
        FileEntryKind::Pyproject => FileKind::Pyproject,
        FileEntryKind::Regex => FileKind::Regex,
        FileEntryKind::Plain | FileEntryKind::Auto => FileKind::Plain,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    // update_version_files

    #[test]
    fn test_update_version_files_single_file_plain() {
        // uses a generic filename so the plain handler is selected
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("version.txt");
        fs::write(&path, "version = \"1.0.0\"\n").unwrap();

        let files = vec![path.to_string_lossy().into_owned()];
        update_version_files(&files, "1.0.0", "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "version = \"2.0.0\"\n");
    }

    #[test]
    fn test_update_version_files_cargo_toml_uses_structured_handler() {
        // with auto-detection, Cargo.toml gets the CargoManifestHandler
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        let toml = "[package]\nname = \"my-crate\"\nversion = \"1.0.0\"\n";
        fs::write(&path, toml).unwrap();

        let files = vec![path.to_string_lossy().into_owned()];
        update_version_files(&files, "1.0.0", "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("version = \"2.0.0\""));
        // name must be unchanged
        assert!(contents.contains("name = \"my-crate\""));
    }

    #[test]
    fn test_update_version_files_multiple_occurrences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, "version 1.0.0 and also 1.0.0\n").unwrap();

        let files = vec![path.to_string_lossy().into_owned()];
        update_version_files(&files, "1.0.0", "1.1.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "version 1.1.0 and also 1.1.0\n");
    }

    #[test]
    fn test_update_version_files_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.toml");
        let b = dir.path().join("b.json");
        fs::write(&a, "version = \"0.1.0\"\n").unwrap();
        fs::write(&b, "{\"version\":\"0.1.0\"}\n").unwrap();

        let files = vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ];
        update_version_files(&files, "0.1.0", "0.2.0").unwrap();

        assert!(fs::read_to_string(&a).unwrap().contains("0.2.0"));
        assert!(fs::read_to_string(&b).unwrap().contains("0.2.0"));
    }

    #[test]
    fn test_update_version_files_not_found_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.txt");
        fs::write(&path, "some content\n").unwrap();

        let files = vec![path.to_string_lossy().into_owned()];
        let err = update_version_files(&files, "9.9.9", "10.0.0").unwrap_err();

        assert!(matches!(err, VersionError::NotFound { .. }));
    }

    #[test]
    fn test_update_version_files_missing_file_returns_error() {
        let files = vec!["/nonexistent/path/file.txt".to_string()];
        let err = update_version_files(&files, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::File { .. }));
    }
}
