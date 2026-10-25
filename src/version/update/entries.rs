//! Conversion of plain file paths into version file entries.

use crate::config::{FileEntryKind, FileEntryStrategy, VersionFileEntry};

/// Turn a flat list of paths into entries that use automatic handler
/// detection and the in-process strategy.
///
/// This is how `[version].commit_version_files` entries are treated.
pub fn entries_from_paths(files: &[String]) -> Vec<VersionFileEntry> {
    files
        .iter()
        .map(|p| VersionFileEntry {
            path: p.clone(),
            kind: FileEntryKind::Auto,
            strategy: FileEntryStrategy::InProcess,
            command: None,
            pattern: None,
            occurrences: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entries_from_paths_uses_auto_detection_in_process() {
        let entries = entries_from_paths(&["Cargo.toml".to_string(), "a/b.txt".to_string()]);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "Cargo.toml");
        assert_eq!(entries[1].path, "a/b.txt");
        for e in &entries {
            assert_eq!(e.kind, FileEntryKind::Auto);
            assert_eq!(e.strategy, FileEntryStrategy::InProcess);
        }
    }

    #[test]
    fn test_entries_from_paths_empty() {
        assert!(entries_from_paths(&[]).is_empty());
    }
}
