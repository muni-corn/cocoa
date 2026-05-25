//! npm package.json and package-lock.json version handlers.
//!
//! Updates the `"version"` field at the top level of `package.json`, and
//! updates only the root entry in `package-lock.json` (the `"version"` key at
//! the document root and the matching entry under `packages[""]`).
//!
//! # Configuration examples
//! ```toml
//! [[version.files]]
//! path = "package.json"
//! kind = "npm"
//!
//! [[version.files]]
//! path = "package-lock.json"
//! kind = "npm-lock"
//! ```

use serde::Serialize as _;
use serde_json::Value;

use crate::version::{
    FileKind, UpdatedFile, VersionError,
    handlers::{Handler, PendingUpdate, read_text},
};

// ── package.json handler
// ──────────────────────────────────────────────────────

/// Handler for `package.json` files.
///
/// Updates only the top-level `"version"` field. All other fields,
/// including dependency versions, are left completely unchanged.
#[derive(Debug, Default)]
pub struct NpmManifestHandler;

impl Handler for NpmManifestHandler {
    fn prepare(
        &self,
        path: &str,
        _old_version: &str,
        new_version: &str,
    ) -> Result<Option<PendingUpdate>, VersionError> {
        let original_text = read_text(path)?;
        let original_bytes = original_text.as_bytes().to_vec();

        let mut json: Value =
            serde_json::from_str(&original_text).map_err(|e| VersionError::ManifestParse {
                path: path.to_owned(),
                message: e.to_string(),
            })?;

        let obj = json
            .as_object_mut()
            .ok_or_else(|| VersionError::ManifestParse {
                path: path.to_owned(),
                message: "expected a JSON object at the root".to_owned(),
            })?;

        if !obj.contains_key("version") {
            return Err(VersionError::ManifestFieldMissing {
                field: "version".to_owned(),
                path: path.to_owned(),
            });
        }

        obj.insert("version".to_owned(), Value::String(new_version.to_owned()));

        // re-serialize preserving the original indentation style
        let indent = detect_indent(&original_text);
        let updated_text = serialize_json_indented(&json, indent)?;

        Ok(Some(PendingUpdate {
            path: path.to_owned(),
            original: original_bytes,
            updated: updated_text.into_bytes(),
            updated_file: UpdatedFile {
                path: path.to_owned(),
                kind: FileKind::NpmManifest,
                replacements: 1,
            },
        }))
    }
}

// ── package-lock.json handler
// ─────────────────────────────────────────────────

/// Handler for `package-lock.json` files.
///
/// Updates only the root-level `"version"` field and the `"version"` inside
/// `packages[""]` (the self-referential entry for the project itself).
/// All `node_modules/*` package entries are left unchanged.
#[derive(Debug, Default)]
pub struct NpmLockHandler;

impl Handler for NpmLockHandler {
    fn prepare(
        &self,
        path: &str,
        _old_version: &str,
        new_version: &str,
    ) -> Result<Option<PendingUpdate>, VersionError> {
        let original_text = read_text(path)?;
        let original_bytes = original_text.as_bytes().to_vec();

        let mut json: Value =
            serde_json::from_str(&original_text).map_err(|e| VersionError::ManifestParse {
                path: path.to_owned(),
                message: e.to_string(),
            })?;

        let obj = json
            .as_object_mut()
            .ok_or_else(|| VersionError::ManifestParse {
                path: path.to_owned(),
                message: "expected a JSON object at the root".to_owned(),
            })?;

        let mut replacements = 0;

        // update the root "version" field (lockfile v1 format)
        if obj.contains_key("version") {
            obj.insert("version".to_owned(), Value::String(new_version.to_owned()));
            replacements += 1;
        }

        // update packages[""].version (lockfile v2/v3 format)
        if let Some(packages) = obj.get_mut("packages").and_then(|p| p.as_object_mut())
            && let Some(root_entry) = packages.get_mut("").and_then(|e| e.as_object_mut())
            && root_entry.contains_key("version")
        {
            root_entry.insert("version".to_owned(), Value::String(new_version.to_owned()));
            replacements += 1;
        }

        if replacements == 0 {
            return Err(VersionError::ManifestFieldMissing {
                field: "version or packages[\"\"].version".to_owned(),
                path: path.to_owned(),
            });
        }

        let indent = detect_indent(&original_text);
        let updated_text = serialize_json_indented(&json, indent)?;

        Ok(Some(PendingUpdate {
            path: path.to_owned(),
            original: original_bytes,
            updated: updated_text.into_bytes(),
            updated_file: UpdatedFile {
                path: path.to_owned(),
                kind: FileKind::NpmLock,
                replacements,
            },
        }))
    }
}

// ── helpers
// ───────────────────────────────────────────────────────────────────

/// Detect the indentation used in a JSON file.
///
/// Returns the number of spaces, defaulting to 2.
fn detect_indent(text: &str) -> usize {
    for line in text.lines() {
        let trimmed = line.trim_start_matches(' ');
        let spaces = line.len() - trimmed.len();
        if spaces > 0 && !trimmed.is_empty() {
            return spaces;
        }
    }
    2
}

/// Serialize a JSON value with the given indentation level, adding a trailing
/// newline to match npm's output convention.
fn serialize_json_indented(value: &Value, indent: usize) -> Result<String, VersionError> {
    let indent_bytes = b" ".repeat(indent);
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent_bytes.as_slice());
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value
        .serialize(&mut ser)
        .map_err(|e| VersionError::ManifestParse {
            path: String::new(),
            message: e.to_string(),
        })?;
    buf.push(b'\n');
    Ok(String::from_utf8_lossy(&buf).into_owned())
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

    // ── NpmManifestHandler tests ──────────────────────────────────────────────

    #[test]
    fn test_npm_manifest_updates_version() {
        let json = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "description": "test"
}
"#;
        let (_dir, path) = write_tmp("package.json", json);
        let h = NpmManifestHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("\"version\": \"2.0.0\""));
        // other fields should be unchanged
        assert!(result.contains("\"name\": \"my-app\""));
        assert!(result.contains("\"description\": \"test\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_npm_manifest_does_not_touch_dependency_versions() {
        let json = r#"{
  "version": "1.0.0",
  "dependencies": {
    "lodash": "1.0.0"
  }
}
"#;
        let (_dir, path) = write_tmp("package.json", json);
        let h = NpmManifestHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        // root version bumped
        assert!(result.contains("\"version\": \"2.0.0\""));
        // dep version untouched
        assert!(result.contains("\"lodash\": \"1.0.0\""));
    }

    #[test]
    fn test_npm_manifest_missing_version_returns_error() {
        let json = r#"{"name": "no-version"}"#;
        let (_dir, path) = write_tmp("package.json", json);
        let h = NpmManifestHandler;
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::ManifestFieldMissing { .. }));
    }

    #[test]
    fn test_npm_manifest_invalid_json_returns_error() {
        let (_dir, path) = write_tmp("package.json", "not json {");
        let h = NpmManifestHandler;
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::ManifestParse { .. }));
    }

    #[test]
    fn test_npm_manifest_kind_is_npm_manifest() {
        let json = r#"{"version": "1.0.0"}"#;
        let (_dir, path) = write_tmp("package.json", json);
        let h = NpmManifestHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.kind, FileKind::NpmManifest);
    }

    // ── NpmLockHandler tests ──────────────────────────────────────────────────

    #[test]
    fn test_npm_lock_v1_updates_root_version() {
        let json = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "lockfileVersion": 1,
  "dependencies": {
    "lodash": {
      "version": "4.17.21"
    }
  }
}
"#;
        let (_dir, path) = write_tmp("package-lock.json", json);
        let h = NpmLockHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        // root version bumped
        assert!(result.contains("\"version\": \"2.0.0\""));
        // dependency version untouched
        assert!(result.contains("\"4.17.21\""));
    }

    #[test]
    fn test_npm_lock_v2_updates_root_and_packages_entry() {
        let json = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "lockfileVersion": 2,
  "packages": {
    "": {
      "version": "1.0.0",
      "name": "my-app"
    },
    "node_modules/lodash": {
      "version": "4.17.21"
    }
  }
}
"#;
        let (_dir, path) = write_tmp("package-lock.json", json);
        let h = NpmLockHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        let result = String::from_utf8(update.updated).unwrap();
        // root version and packages[""] entry both bumped
        assert_eq!(result.matches("\"version\": \"2.0.0\"").count(), 2);
        // lodash version untouched
        assert!(result.contains("\"4.17.21\""));
        assert_eq!(update.updated_file.replacements, 2);
    }

    #[test]
    fn test_npm_lock_no_version_fields_returns_error() {
        let json = r#"{"name": "app", "lockfileVersion": 3}"#;
        let (_dir, path) = write_tmp("package-lock.json", json);
        let h = NpmLockHandler;
        let err = h.prepare(&path, "1.0.0", "2.0.0").unwrap_err();
        assert!(matches!(err, VersionError::ManifestFieldMissing { .. }));
    }

    #[test]
    fn test_npm_lock_kind_is_npm_lock() {
        let json = r#"{"version": "1.0.0"}"#;
        let (_dir, path) = write_tmp("package-lock.json", json);
        let h = NpmLockHandler;
        let update = h.prepare(&path, "1.0.0", "2.0.0").unwrap().unwrap();
        assert_eq!(update.updated_file.kind, FileKind::NpmLock);
    }

    #[test]
    fn test_detect_indent_2_spaces() {
        let json = "{\n  \"a\": 1\n}";
        assert_eq!(detect_indent(json), 2);
    }

    #[test]
    fn test_detect_indent_4_spaces() {
        let json = "{\n    \"a\": 1\n}";
        assert_eq!(detect_indent(json), 4);
    }
}
