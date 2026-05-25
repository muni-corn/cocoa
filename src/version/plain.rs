//! Plain text version handler.
//!
//! Replaces all occurrences of the old version string with the new version
//! string in the file. This matches the historical behavior of
//! `update_version_files`.

use crate::{
    config::Occurrences,
    version::{
        FileKind, UpdatedFile, VersionError,
        handlers::{Handler, PendingUpdate, read_bytes, read_text},
    },
};

/// Handler that replaces the version string by plain text substitution.
///
/// Uses `str::replace` for `occurrences = "all"` (the default) and
/// `replacen` when a count or `"first"` is configured.
#[derive(Default)]
pub struct PlainHandler {
    /// How many occurrences to replace.
    pub occurrences: Occurrences,
}

impl Handler for PlainHandler {
    fn prepare(
        &self,
        path: &str,
        old_version: &str,
        new_version: &str,
    ) -> Result<Option<PendingUpdate>, VersionError> {
        let original_text = read_text(path)?;
        let original_bytes = read_bytes(path)?;

        if !original_text.contains(old_version) {
            return Err(VersionError::NotFound {
                version: old_version.to_owned(),
                path: path.to_owned(),
            });
        }

        let (updated_text, replacements) =
            apply_replacements(&original_text, old_version, new_version, &self.occurrences);

        Ok(Some(PendingUpdate {
            path: path.to_owned(),
            original: original_bytes,
            updated: updated_text.into_bytes(),
            updated_file: UpdatedFile {
                path: path.to_owned(),
                kind: FileKind::Plain,
                replacements,
            },
        }))
    }
}

/// Apply replacements according to the occurrences setting.
///
/// Returns the updated string and the number of replacements made.
pub fn apply_replacements(
    text: &str,
    old: &str,
    new: &str,
    occurrences: &Occurrences,
) -> (String, usize) {
    use crate::config::OccurrencesNamed;

    match occurrences {
        Occurrences::Named(OccurrencesNamed::All) => {
            let count = text.matches(old).count();
            (text.replace(old, new), count)
        }
        Occurrences::Named(OccurrencesNamed::First) | Occurrences::Count(1) => {
            let count = usize::from(text.contains(old));
            (text.replacen(old, new, 1), count)
        }
        Occurrences::Count(n) => {
            let count = text.matches(old).count().min(*n);
            (text.replacen(old, new, *n), count)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::config::OccurrencesNamed;

    fn write_tmp(content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, content).unwrap();
        let path_str = path.to_string_lossy().into_owned();
        (dir, path_str)
    }

    #[test]
    fn test_plain_handler_replaces_all_by_default() {
        let (_dir, path) = write_tmp("v1.0.0 and v1.0.0");
        let h = PlainHandler::default();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.replacements, 2);
        let text = String::from_utf8(update.updated).unwrap();
        assert_eq!(text, "v2.0.0 and v2.0.0");
    }

    #[test]
    fn test_plain_handler_replaces_first_only() {
        let (_dir, path) = write_tmp("1.0.0 and 1.0.0");
        let h = PlainHandler {
            occurrences: Occurrences::Named(OccurrencesNamed::First),
        };
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.replacements, 1);
        let text = String::from_utf8(update.updated).unwrap();
        assert_eq!(text, "2.0.0 and 1.0.0");
    }

    #[test]
    fn test_plain_handler_replaces_n_occurrences() {
        let (_dir, path) = write_tmp("1.0.0 1.0.0 1.0.0");
        let h = PlainHandler {
            occurrences: Occurrences::Count(2),
        };
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.replacements, 2);
        let text = String::from_utf8(update.updated).unwrap();
        assert_eq!(text, "2.0.0 2.0.0 1.0.0");
    }

    #[test]
    fn test_plain_handler_not_found_returns_error() {
        let (_dir, path) = write_tmp("nothing here");
        let h = PlainHandler::default();
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::NotFound { .. }));
    }

    #[test]
    fn test_plain_handler_missing_file_returns_error() {
        let h = PlainHandler::default();
        let err = h
            .prepare("/nonexistent/path.txt", "1.0.0", "2.0.0")
            .unwrap_err();
        assert!(matches!(err, VersionError::File { .. }));
    }
}
