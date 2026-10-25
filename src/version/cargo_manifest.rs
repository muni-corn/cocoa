//! Cargo manifest version handler.
//!
//! Updates `[package].version` (and optionally `[workspace.package].version`)
//! in a Cargo.toml file using `toml_edit`, which preserves all comments,
//! formatting, and ordering in the original file.
//!
//! # Configuration example
//! ```toml
//! [[version.files]]
//! path = "Cargo.toml"
//! kind = "cargo"
//! ```

use toml_edit::DocumentMut;

use crate::version::{
    FileKind, UpdatedFile, VersionError,
    handlers::{Handler, PendingUpdate, read_text},
};

/// Handler for Cargo.toml package manifests.
///
/// Updates `[package].version` when present, and `[workspace.package].version`
/// when the file is a workspace root that declares `[workspace.package]`.
/// Both can coexist in the same file (mixed workspace root + member). A
/// version inherited with `version.workspace = true` is left untouched.
#[derive(Debug)]
pub struct CargoManifestHandler {
    /// Whether the version fields must currently hold `old_version`.
    ///
    /// When `false`, version fields are overwritten whatever they hold.
    pub verify_old_version: bool,
}

impl Default for CargoManifestHandler {
    fn default() -> Self {
        Self {
            verify_old_version: true,
        }
    }
}

impl Handler for CargoManifestHandler {
    fn prepare(
        &self,
        path: &str,
        old_version: &str,
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
        let mut inherited = false;

        // update [package].version if present; `version.workspace = true` is
        // not a string, and replacing it would break the inheritance
        if let Some(pkg) = doc.get_mut("package").and_then(|v| v.as_table_mut())
            && let Some(ver) = pkg.get_mut("version")
        {
            if ver.is_str() {
                self.check_old_version(path, "[package].version", ver, old_version)?;
                *ver = toml_edit::value(new_version);
                replacements += 1;
            } else {
                inherited = true;
            }
        }

        // update [workspace.package].version if present (workspace root)
        if let Some(ws) = doc.get_mut("workspace").and_then(|v| v.as_table_mut())
            && let Some(ws_pkg) = ws.get_mut("package").and_then(|v| v.as_table_mut())
            && let Some(ver) = ws_pkg.get_mut("version")
        {
            if ver.is_str() {
                self.check_old_version(path, "[workspace.package].version", ver, old_version)?;
                *ver = toml_edit::value(new_version);
                replacements += 1;
            } else {
                inherited = true;
            }
        }

        // the version lives in the workspace root, so this file needs no change
        if replacements == 0 && inherited {
            return Ok(None);
        }

        if replacements == 0 {
            return Err(VersionError::ManifestFieldMissing {
                field: "[package].version or [workspace.package].version".to_owned(),
                path: path.to_owned(),
            });
        }

        Ok(Some(PendingUpdate {
            path: path.to_owned(),
            original: original_bytes,
            updated: doc.to_string().into_bytes(),
            updated_file: UpdatedFile {
                path: path.to_owned(),
                kind: FileKind::CargoManifest,
                replacements,
            },
        }))
    }
}

impl CargoManifestHandler {
    /// Fail when a version field does not hold the version being replaced.
    fn check_old_version(
        &self,
        path: &str,
        what: &str,
        field: &toml_edit::Item,
        old_version: &str,
    ) -> Result<(), VersionError> {
        let found = field.as_str().unwrap_or_default();
        if self.verify_old_version && found != old_version {
            return Err(VersionError::VersionMismatch {
                path: path.to_owned(),
                what: what.to_owned(),
                found: found.to_owned(),
                expected: old_version.to_owned(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn write_tmp(name: &str, content: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        (dir, path.to_string_lossy().into_owned())
    }

    #[test]
    fn test_cargo_manifest_updates_package_version() {
        let toml = r#"[package]
name = "my-crate"
version = "1.0.0"
edition = "2021"
"#;
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"2.0.0\""));
        // name and edition must be unchanged
        assert!(result.contains("name = \"my-crate\""));
        assert!(result.contains("edition = \"2021\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_manifest_updates_workspace_package_version() {
        let toml = r#"[workspace]
members = ["crate-a", "crate-b"]

[workspace.package]
version = "0.5.0"
edition = "2021"
"#;
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let update = h.prepare(&path, "0.5.0", "0.6.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"0.6.0\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_manifest_updates_both_package_and_workspace_version() {
        let toml = r#"[workspace]
members = ["crate-a"]

[workspace.package]
version = "1.2.3"

[package]
name = "root-crate"
version = "1.2.3"
"#;
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let update = h.prepare(&path, "1.2.3", "1.3.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        // both version fields should be updated
        assert_eq!(result.matches("\"1.3.0\"").count(), 2);
        assert_eq!(update.updated_file.replacements, 2);
    }

    #[test]
    fn test_cargo_manifest_does_not_touch_dependency_versions() {
        let toml = r#"[package]
name = "my-crate"
version = "1.0.0"

[dependencies]
serde = "1.0.0"
"#;
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        // serde version must stay at 1.0.0
        assert!(result.contains("serde = \"1.0.0\""));
        // package version must be updated
        assert!(result.contains("version = \"2.0.0\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_manifest_preserves_comments_and_formatting() {
        let toml = "# top comment\n[package]\nname = \"crate\" # inline\nversion = \"0.1.0\"\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let update = h.prepare(&path, "0.1.0", "0.2.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("# top comment"));
        assert!(result.contains("name = \"crate\" # inline"));
        assert!(result.contains("version = \"0.2.0\""));
    }

    #[test]
    fn test_cargo_manifest_no_version_field_returns_error() {
        let toml = "[package]\nname = \"minimal\"\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let err = h.prepare(&path, "0.1.0", "0.2.0").unwrap_err();
        assert!(matches!(err, VersionError::ManifestFieldMissing { .. }));
    }

    #[test]
    fn test_cargo_manifest_invalid_toml_returns_error() {
        let (_dir, path) = write_tmp("Cargo.toml", "this = [not valid");
        let h = CargoManifestHandler::default();
        let err = h.prepare(&path, "0.1.0", "0.2.0").unwrap_err();
        assert!(matches!(err, VersionError::ManifestParse { .. }));
    }

    #[test]
    fn test_cargo_manifest_kind_is_cargo_manifest() {
        let toml = "[package]\nname = \"c\"\nversion = \"1.0.0\"\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.kind, FileKind::CargoManifest);
    }

    #[test]
    fn test_cargo_manifest_leaves_inherited_dotted_version_alone() {
        let toml = "[package]\nname = \"member\"\nversion.workspace = true\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        // nothing to change: the workspace root owns the version
        assert!(h.prepare(&path, "1.0.0", "2.0.0").unwrap().is_none());
    }

    #[test]
    fn test_cargo_manifest_leaves_inherited_inline_version_alone() {
        let toml = "[package]\nname = \"member\"\nversion = { workspace = true }\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        assert!(h.prepare(&path, "1.0.0", "2.0.0").unwrap().is_none());
    }

    #[test]
    fn test_cargo_manifest_updates_workspace_version_but_keeps_inherited_member() {
        let toml = r#"[workspace]
members = ["a"]

[workspace.package]
version = "1.0.0"

[package]
name = "root"
version.workspace = true
"#;
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler::default();
        let update = h.prepare(&path, "1.0.0", "1.1.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"1.1.0\""));
        assert!(result.contains("version.workspace = true"));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_manifest_version_mismatch_returns_error() {
        let toml = "[package]\nname = \"cocoa\"\nversion = \"0.5.1\"\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let err = CargoManifestHandler::default()
            .prepare(&path, "0.5.3", "0.5.4")
            .unwrap_err();
        match err {
            VersionError::VersionMismatch {
                what,
                found,
                expected,
                ..
            } => {
                assert_eq!(what, "[package].version");
                assert_eq!(found, "0.5.1");
                assert_eq!(expected, "0.5.3");
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn test_cargo_manifest_workspace_version_mismatch_returns_error() {
        let toml = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.5.1\"\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let err = CargoManifestHandler::default()
            .prepare(&path, "0.5.3", "0.5.4")
            .unwrap_err();
        assert!(matches!(
            err,
            VersionError::VersionMismatch { what, .. } if what == "[workspace.package].version"
        ));
    }

    #[test]
    fn test_cargo_manifest_without_verification_overwrites_any_version() {
        let toml = "[package]\nname = \"cocoa\"\nversion = \"0.1.0\"\n";
        let (_dir, path) = write_tmp("Cargo.toml", toml);
        let h = CargoManifestHandler {
            verify_old_version: false,
        };
        let update = h.prepare(&path, "0.0.0", "0.1.1").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"0.1.1\""));
    }
}
