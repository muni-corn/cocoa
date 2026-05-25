//! pyproject.toml version handler.
//!
//! Updates `[project].version` (PEP 621 standard) or falls back to
//! `[tool.poetry].version` (Poetry legacy) in `pyproject.toml` files.
//! Uses `toml_edit` to preserve all comments, formatting, and ordering.
//!
//! # Configuration example
//! ```toml
//! [[version.files]]
//! path = "pyproject.toml"
//! kind = "pyproject"
//! ```

use toml_edit::DocumentMut;

use crate::version::{
    FileKind, UpdatedFile, VersionError,
    handlers::{Handler, PendingUpdate, read_text},
};

/// Handler for pyproject.toml files.
///
/// Tries `[project].version` first (PEP 621). If that field is absent,
/// falls back to `[tool.poetry].version` (Poetry). Returns
/// `ManifestFieldMissing` when neither field is present.
#[derive(Debug, Default)]
pub struct PyprojectHandler;

impl Handler for PyprojectHandler {
    fn prepare(
        &self,
        path: &str,
        _old_version: &str,
        new_version: &str,
    ) -> Result<Option<PendingUpdate>, VersionError> {
        let original_text = read_text(path)?;
        let original_bytes = original_text.as_bytes().to_vec();

        let mut doc: DocumentMut = original_text.parse().map_err(|e: toml_edit::TomlError| {
            VersionError::ManifestParse {
                path: path.to_owned(),
                message: e.to_string(),
            }
        })?;

        let mut replacements = 0;

        // PEP 621: [project].version
        if let Some(project) = doc.get_mut("project").and_then(|v| v.as_table_mut())
            && let Some(ver) = project.get_mut("version")
        {
            *ver = toml_edit::value(new_version);
            replacements += 1;
        }

        // Poetry: [tool.poetry].version (only if PEP 621 field not found)
        if replacements == 0
            && let Some(tool) = doc.get_mut("tool").and_then(|v| v.as_table_mut())
            && let Some(poetry) = tool.get_mut("poetry").and_then(|v| v.as_table_mut())
            && let Some(ver) = poetry.get_mut("version")
        {
            *ver = toml_edit::value(new_version);
            replacements += 1;
        }

        if replacements == 0 {
            return Err(VersionError::ManifestFieldMissing {
                field: "[project].version or [tool.poetry].version".to_owned(),
                path: path.to_owned(),
            });
        }

        Ok(Some(PendingUpdate {
            path: path.to_owned(),
            original: original_bytes,
            updated: doc.to_string().into_bytes(),
            updated_file: UpdatedFile {
                path: path.to_owned(),
                kind: FileKind::Pyproject,
                replacements,
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write_tmp(content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pyproject.toml");
        fs::write(&path, content).unwrap();
        (dir, path.to_string_lossy().into_owned())
    }

    #[test]
    fn test_pyproject_updates_pep621_version() {
        let toml = "[project]\nname = \"my-app\"\nversion = \"1.0.0\"\n";
        let (_dir, path) = write_tmp(toml);
        let h = PyprojectHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"2.0.0\""));
        assert!(result.contains("name = \"my-app\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_pyproject_updates_poetry_version_as_fallback() {
        let toml = "[tool.poetry]\nname = \"my-app\"\nversion = \"0.5.0\"\n";
        let (_dir, path) = write_tmp(toml);
        let h = PyprojectHandler;
        let update = h.prepare(&path, "0.5.0", "0.6.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"0.6.0\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_pyproject_prefers_pep621_over_poetry() {
        let toml = "[project]\nversion = \"1.0.0\"\n\n[tool.poetry]\nversion = \"1.0.0\"\n";
        let (_dir, path) = write_tmp(toml);
        let h = PyprojectHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        // only the [project] section should be updated (replacements = 1)
        assert_eq!(update.updated_file.replacements, 1);
        // [project].version is bumped
        assert!(result.contains("[project]\nversion = \"2.0.0\""));
        // [tool.poetry].version is left at 1.0.0
        assert!(result.contains("[tool.poetry]\nversion = \"1.0.0\""));
    }

    #[test]
    fn test_pyproject_preserves_comments_and_formatting() {
        let toml = "# top comment\n[project]\nname = \"pkg\" # inline\nversion = \"0.1.0\"\n";
        let (_dir, path) = write_tmp(toml);
        let h = PyprojectHandler;
        let update = h.prepare(&path, "0.1.0", "0.2.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("# top comment"));
        assert!(result.contains("name = \"pkg\" # inline"));
        assert!(result.contains("version = \"0.2.0\""));
    }

    #[test]
    fn test_pyproject_no_version_field_returns_error() {
        let toml = "[project]\nname = \"no-version\"\n";
        let (_dir, path) = write_tmp(toml);
        let h = PyprojectHandler;
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::ManifestFieldMissing { .. }));
    }

    #[test]
    fn test_pyproject_invalid_toml_returns_error() {
        let (_dir, path) = write_tmp("this = [not valid");
        let h = PyprojectHandler;
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::ManifestParse { .. }));
    }

    #[test]
    fn test_pyproject_kind_is_pyproject() {
        let toml = "[project]\nversion = \"1.0.0\"\n";
        let (_dir, path) = write_tmp(toml);
        let h = PyprojectHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.kind, FileKind::Pyproject);
    }
}
