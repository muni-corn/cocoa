//! File handler infrastructure for version bumping.
//!
//! Each handler knows how to update a specific kind of file (manifest,
//! lockfile, plain text, etc.) when the project version changes.

use std::{fs, path::Path};

use crate::version::{UpdatedFile, VersionError};

/// A pending file update, holding both the original and new content.
///
/// Used in the two-phase write cycle: first all handlers compute their
/// updates, then they are written atomically with rollback on failure.
#[derive(Debug)]
pub struct PendingUpdate {
    /// Path of the file to write.
    pub path: String,
    /// Original file content (used for rollback).
    pub original: Vec<u8>,
    /// New file content to write.
    pub updated: Vec<u8>,
    /// Metadata about how the update was computed.
    pub updated_file: UpdatedFile,
}

/// Common interface for all file version handlers.
pub trait Handler {
    /// Compute the update for the given file without writing it.
    ///
    /// Returns `None` if the handler decides the file needs no changes (for
    /// example, a `skip` strategy), or an error if something went wrong.
    fn prepare(
        &self,
        path: &str,
        old_version: &str,
        new_version: &str,
    ) -> Result<Option<PendingUpdate>, VersionError>;
}

/// Apply a list of pending updates atomically.
///
/// Writes every update in order. On the first write failure, restores all
/// previously written files to their original content and returns the error.
pub fn apply_updates(updates: Vec<PendingUpdate>) -> Result<Vec<UpdatedFile>, VersionError> {
    let mut written: Vec<(String, Vec<u8>)> = Vec::new();
    let mut results: Vec<UpdatedFile> = Vec::new();

    for update in &updates {
        if let Err(e) = fs::write(&update.path, &update.updated) {
            // roll back files that were already written
            for (p, orig) in &written {
                let _ = fs::write(p, orig);
            }
            return Err(VersionError::File {
                path: update.path.clone(),
                source: e,
            });
        }
        written.push((update.path.clone(), update.original.clone()));
        results.push(update.updated_file.clone());
    }

    Ok(results)
}

/// Read a file's bytes, returning a `VersionError::File` on failure.
pub fn read_bytes(path: &str) -> Result<Vec<u8>, VersionError> {
    fs::read(path).map_err(|e| VersionError::File {
        path: path.to_owned(),
        source: e,
    })
}

/// Read a file as UTF-8, returning a `VersionError::File` on failure.
pub fn read_text(path: &str) -> Result<String, VersionError> {
    fs::read_to_string(path).map_err(|e| VersionError::File {
        path: path.to_owned(),
        source: e,
    })
}

/// Extract the file basename from a path string.
pub fn basename(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}
