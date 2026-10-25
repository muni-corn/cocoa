//! Cargo.lock workspace-aware version handler.
//!
//! Updates only the `[[package]]` entries in `Cargo.lock` whose `name` field
//! matches a workspace member (or the root package), leaving all transient
//! dependencies untouched. Workspace members are discovered by parsing the
//! workspace's `Cargo.toml`.
//!
//! # How it works
//!
//! 1. Locate the workspace manifest (walks up from the lockfile's directory
//!    until it finds a `Cargo.toml` containing `[workspace]`, or falls back to
//!    any `Cargo.toml` in the same directory).
//! 2. Collect member package names from that manifest.
//! 3. Parse `Cargo.lock` with `toml_edit`.
//! 4. Iterate `[[package]]` entries; update `version` only where `name` is in
//!    the member set.
//!
//! # Configuration example
//! ```toml
//! [[version.files]]
//! path = "Cargo.lock"
//! kind = "cargo-lock"
//! ```

use std::path::Path;

use toml_edit::{ArrayOfTables, DocumentMut, Item};

use crate::version::{
    FileKind, UpdatedFile, VersionError,
    handlers::{Handler, PendingUpdate, read_text},
};

/// Handler for Cargo.lock files.
///
/// Discovers workspace members from `Cargo.toml` (relative to the lockfile
/// path) and updates only those `[[package]]` entries.
#[derive(Debug)]
pub struct CargoLockHandler {
    /// Path to the workspace root `Cargo.toml`.
    ///
    /// When `None`, the handler infers the manifest location from the lockfile
    /// path.
    pub manifest_path: Option<String>,

    /// Whether member entries must currently hold `old_version` to be updated.
    ///
    /// When `false`, every member entry is set to the new version regardless of
    /// what it holds now.
    pub verify_old_version: bool,
}

impl Default for CargoLockHandler {
    fn default() -> Self {
        Self {
            manifest_path: None,
            verify_old_version: true,
        }
    }
}

impl Handler for CargoLockHandler {
    fn prepare(
        &self,
        path: &str,
        old_version: &str,
        new_version: &str,
    ) -> Result<Option<PendingUpdate>, VersionError> {
        // resolve the manifest path
        let manifest_path = match &self.manifest_path {
            Some(p) => p.clone(),
            None => infer_manifest_path(path)?,
        };

        // collect workspace member names from the manifest
        let member_names = collect_workspace_member_names(&manifest_path)?;

        // parse the lockfile
        let original_text = read_text(path)?;
        let original_bytes = original_text.as_bytes().to_vec();

        let mut doc: DocumentMut = original_text.parse().map_err(|e: toml_edit::TomlError| {
            VersionError::ManifestParse {
                path: path.to_owned(),
                message: e.to_string(),
            }
        })?;

        // update only [[package]] entries whose name is a workspace member
        let LockUpdate {
            replacements,
            mismatched,
        } = update_lock_packages(&mut doc, &member_names, VersionChange {
            old: old_version,
            new: new_version,
            verify_old: self.verify_old_version,
        });

        if replacements == 0 {
            // a member was present but at another version: the files and tags
            // have drifted apart, which the user can fix
            if let Some((name, found)) = mismatched.into_iter().next() {
                return Err(VersionError::VersionMismatch {
                    path: path.to_owned(),
                    what: format!("'{name}'"),
                    found,
                    expected: old_version.to_owned(),
                });
            }

            // no member entries at all; this is likely a config mistake
            return Err(VersionError::ManifestFieldMissing {
                field: format!(
                    "[[package]] entries matching workspace members {:?}",
                    member_names
                ),
                path: path.to_owned(),
            });
        }

        Ok(Some(PendingUpdate {
            path: path.to_owned(),
            original: original_bytes,
            updated: doc.to_string().into_bytes(),
            updated_file: UpdatedFile {
                path: path.to_owned(),
                kind: FileKind::CargoLock,
                replacements,
            },
        }))
    }
}

/// Find a Cargo.toml adjacent to or above the lockfile path.
///
/// Tries the same directory first (common single-crate layout), then walks
/// upward one level to handle workspaces where the lockfile sits in the root
/// alongside the workspace Cargo.toml.
fn infer_manifest_path(lock_path: &str) -> Result<String, VersionError> {
    let lock = Path::new(lock_path);
    let dir = lock.parent().unwrap_or(Path::new("."));

    // first try: Cargo.toml next to the lockfile
    let candidate = dir.join("Cargo.toml");
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().into_owned());
    }

    // second try: one level up (monorepo style)
    if let Some(parent) = dir.parent() {
        let up = parent.join("Cargo.toml");
        if up.exists() {
            return Ok(up.to_string_lossy().into_owned());
        }
    }

    Err(VersionError::ManifestFieldMissing {
        field: "Cargo.toml".to_owned(),
        path: lock_path.to_owned(),
    })
}

/// Collect the names of all packages that are workspace members.
///
/// This includes:
/// - The root package (from `[package].name`), if present.
/// - All packages listed in `[workspace.members]` (parsed by glob patterns
///   pointing to sub-directories containing their own Cargo.toml files).
///
/// For the glob-based sub-members we read each sub-`Cargo.toml` to find
/// `[package].name`. If glob expansion fails we fall back to the last path
/// component as the name (which matches crate naming conventions for simple
/// cases).
fn collect_workspace_member_names(manifest_path: &str) -> Result<Vec<String>, VersionError> {
    let text = read_text(manifest_path)?;
    let doc: DocumentMut =
        text.parse()
            .map_err(|e: toml_edit::TomlError| VersionError::ManifestParse {
                path: manifest_path.to_owned(),
                message: e.to_string(),
            })?;

    let mut names: Vec<String> = Vec::new();

    // the root package itself
    if let Some(name) = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        names.push(name.to_owned());
    }

    // workspace members listed under [workspace.members]
    if let Some(members) = doc
        .get("workspace")
        .and_then(|ws| ws.get("members"))
        .and_then(|m| m.as_array())
    {
        let manifest_dir = Path::new(manifest_path).parent().unwrap_or(Path::new("."));

        for member in members.iter() {
            let Some(pattern) = member.as_str() else {
                continue;
            };

            // try to expand the glob to find sub-Cargo.toml files
            let full_pattern = manifest_dir.join(pattern).join("Cargo.toml");
            let full_str = full_pattern.to_string_lossy();

            if let Ok(paths) = glob::glob(&full_str) {
                for entry in paths.flatten() {
                    if let Ok(sub_text) = std::fs::read_to_string(&entry)
                        && let Ok(sub_doc) = sub_text.parse::<DocumentMut>()
                        && let Some(n) = sub_doc
                            .get("package")
                            .and_then(|p| p.get("name"))
                            .and_then(|n| n.as_str())
                    {
                        names.push(n.to_owned());
                    }
                }
            } else {
                // glob expansion failed; use the last path component as the
                // best-effort name (e.g. "crates/foo" → "foo")
                if let Some(fallback) = Path::new(pattern).file_name().and_then(|n| n.to_str()) {
                    names.push(fallback.to_owned());
                }
            }
        }
    }

    Ok(names)
}

/// The outcome of updating workspace members in a lockfile.
struct LockUpdate {
    /// Number of entries whose version was replaced.
    replacements: usize,
    /// Member entries left alone because their version was not the old one,
    /// as `(name, version found)` pairs.
    mismatched: Vec<(String, String)>,
}

/// The version change to apply to lockfile entries.
#[derive(Clone, Copy)]
struct VersionChange<'a> {
    old: &'a str,
    new: &'a str,
    /// Whether an entry must hold `old` to be updated.
    verify_old: bool,
}

/// Update `version` in each `[[package]]` entry whose `name` is in `members`.
fn update_lock_packages(
    doc: &mut DocumentMut,
    members: &[String],
    change: VersionChange<'_>,
) -> LockUpdate {
    let Some(Item::ArrayOfTables(aot)) = doc.get_mut("package") else {
        return LockUpdate {
            replacements: 0,
            mismatched: Vec::new(),
        };
    };

    update_packages_in_aot(aot, members, change)
}

/// Walk an `ArrayOfTables` updating matching entries.
fn update_packages_in_aot(
    aot: &mut ArrayOfTables,
    members: &[String],
    change: VersionChange<'_>,
) -> LockUpdate {
    let mut update = LockUpdate {
        replacements: 0,
        mismatched: Vec::new(),
    };

    for table in aot.iter_mut() {
        let name = table
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_owned();

        if !members.contains(&name) {
            continue;
        }

        if let Some(ver) = table.get_mut("version") {
            // only update if the version matches what we expect to replace
            match ver.as_str() {
                Some(v) if !change.verify_old || v == change.old => {
                    *ver = toml_edit::value(change.new);
                    update.replacements += 1;
                }
                Some(v) => update.mismatched.push((name, v.to_owned())),
                None => {}
            }
        }
    }

    update
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    struct TmpWorkspace {
        _dir: tempfile::TempDir,
        root: std::path::PathBuf,
    }

    impl TmpWorkspace {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().to_path_buf();
            Self { _dir: dir, root }
        }

        fn write(&self, rel: &str, content: &str) {
            let p = self.root.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(p, content).unwrap();
        }

        fn path(&self, rel: &str) -> String {
            self.root.join(rel).to_string_lossy().into_owned()
        }
    }

    fn single_crate_manifest(version: &str, name: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2021\"\n")
    }

    fn simple_lock(name: &str, version: &str) -> String {
        format!(
            "# This file is automatically @generated by Cargo.\nversion = 4\n\n[[package]]\nname \
             = \"{name}\"\nversion = \"{version}\"\nsource = \"registry+something\"\n"
        )
    }

    #[test]
    fn test_cargo_lock_updates_single_workspace_member() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("1.0.0", "my-crate"));
        ws.write("Cargo.lock", &simple_lock("my-crate", "1.0.0"));

        let handler = CargoLockHandler::default();
        let update = handler
            .prepare(&ws.path("Cargo.lock"), "1.0.0", "2.0.0")
            .unwrap()
            .unwrap();

        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"2.0.0\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_lock_does_not_update_transient_deps() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("1.0.0", "my-crate"));

        // lockfile contains both the workspace member and a transient dep
        // that happens to share the same version
        let lock = "version = 4\n\n[[package]]\nname = \"my-crate\"\nversion = \
                    \"1.0.0\"\n\n[[package]]\nname = \"transient-dep\"\nversion = \"1.0.0\"\n";
        ws.write("Cargo.lock", lock);

        let handler = CargoLockHandler::default();
        let update = handler
            .prepare(&ws.path("Cargo.lock"), "1.0.0", "2.0.0")
            .unwrap()
            .unwrap();

        let result = String::from_utf8(update.updated).unwrap();
        // only the workspace member entry should have the new version
        assert!(result.contains("name = \"my-crate\"\nversion = \"2.0.0\""));
        // the transient dep must remain at 1.0.0
        assert!(result.contains("name = \"transient-dep\"\nversion = \"1.0.0\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_lock_updates_multiple_workspace_members() {
        let ws = TmpWorkspace::new();

        // workspace root
        let root_manifest = "[workspace]\nmembers = [\"crate-a\", \
                             \"crate-b\"]\n\n[workspace.package]\nversion = \"0.5.0\"\n";
        ws.write("Cargo.toml", root_manifest);

        // sub-crate manifests
        ws.write(
            "crate-a/Cargo.toml",
            "[package]\nname = \"crate-a\"\nversion = \"0.5.0\"\n",
        );
        ws.write(
            "crate-b/Cargo.toml",
            "[package]\nname = \"crate-b\"\nversion = \"0.5.0\"\n",
        );

        let lock = "version = 4\n\n[[package]]\nname = \"crate-a\"\nversion = \
                    \"0.5.0\"\n\n[[package]]\nname = \"crate-b\"\nversion = \
                    \"0.5.0\"\n\n[[package]]\nname = \"external\"\nversion = \"0.5.0\"\n";
        ws.write("Cargo.lock", lock);

        let handler = CargoLockHandler::default();
        let update = handler
            .prepare(&ws.path("Cargo.lock"), "0.5.0", "0.6.0")
            .unwrap()
            .unwrap();

        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("name = \"crate-a\"\nversion = \"0.6.0\""));
        assert!(result.contains("name = \"crate-b\"\nversion = \"0.6.0\""));
        // external dep untouched
        assert!(result.contains("name = \"external\"\nversion = \"0.5.0\""));
        assert_eq!(update.updated_file.replacements, 2);
    }

    #[test]
    fn test_cargo_lock_no_matching_entries_returns_error() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("1.0.0", "my-crate"));

        // lockfile contains only an unrelated package
        let lock = "version = 4\n\n[[package]]\nname = \"other\"\nversion = \"1.0.0\"\n";
        ws.write("Cargo.lock", lock);

        let handler = CargoLockHandler::default();
        let err = handler
            .prepare(&ws.path("Cargo.lock"), "1.0.0", "2.0.0")
            .unwrap_err();
        assert!(matches!(err, VersionError::ManifestFieldMissing { .. }));
    }

    #[test]
    fn test_cargo_lock_invalid_toml_returns_error() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("1.0.0", "my-crate"));
        ws.write("Cargo.lock", "this = [not valid");

        let handler = CargoLockHandler::default();
        let err = handler
            .prepare(&ws.path("Cargo.lock"), "1.0.0", "2.0.0")
            .unwrap_err();
        assert!(matches!(err, VersionError::ManifestParse { .. }));
    }

    #[test]
    fn test_cargo_lock_kind_is_cargo_lock() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("1.0.0", "my-crate"));
        ws.write("Cargo.lock", &simple_lock("my-crate", "1.0.0"));

        let handler = CargoLockHandler::default();
        let update = handler
            .prepare(&ws.path("Cargo.lock"), "1.0.0", "2.0.0")
            .unwrap()
            .unwrap();
        assert_eq!(update.updated_file.kind, FileKind::CargoLock);
    }

    #[test]
    fn test_cargo_lock_explicit_manifest_path() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("1.0.0", "my-crate"));
        ws.write("Cargo.lock", &simple_lock("my-crate", "1.0.0"));

        let handler = CargoLockHandler {
            manifest_path: Some(ws.path("Cargo.toml")),
            ..Default::default()
        };
        let update = handler
            .prepare(&ws.path("Cargo.lock"), "1.0.0", "2.0.0")
            .unwrap()
            .unwrap();
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_lock_skips_entry_when_version_does_not_match_old() {
        // if a package is a workspace member but its lockfile version differs
        // from old_version, we should not touch it (avoids incorrect updates
        // in out-of-sync states)
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("2.0.0", "my-crate"));

        // lockfile already has 2.0.0; old_version says 1.0.0
        ws.write("Cargo.lock", &simple_lock("my-crate", "2.0.0"));

        let handler = CargoLockHandler::default();
        let err = handler
            .prepare(&ws.path("Cargo.lock"), "1.0.0", "3.0.0")
            .unwrap_err();
        // the member is present at another version, so the error says so
        match err {
            VersionError::VersionMismatch {
                found, expected, ..
            } => {
                assert_eq!(found, "2.0.0");
                assert_eq!(expected, "1.0.0");
            }
            other => panic!("expected VersionMismatch, got {other:?}"),
        }
    }

    #[test]
    fn test_cargo_lock_mismatch_error_names_the_member() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("0.5.1", "cocoa"));
        ws.write("Cargo.lock", &simple_lock("cocoa", "0.5.1"));

        let err = CargoLockHandler::default()
            .prepare(&ws.path("Cargo.lock"), "0.5.3", "0.5.4")
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("'cocoa' is 0.5.1"),
            "unexpected message: {msg}"
        );
        assert!(msg.contains("0.5.3 was expected"));
    }

    #[test]
    fn test_cargo_lock_without_verification_updates_any_member_version() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("0.1.0", "my-crate"));
        ws.write("Cargo.lock", &simple_lock("my-crate", "0.1.0"));

        let handler = CargoLockHandler {
            verify_old_version: false,
            ..Default::default()
        };
        // 0.0.0 is a placeholder that the lockfile never held
        let update = handler
            .prepare(&ws.path("Cargo.lock"), "0.0.0", "0.1.1")
            .unwrap()
            .unwrap();

        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("version = \"0.1.1\""));
        assert_eq!(update.updated_file.replacements, 1);
    }

    #[test]
    fn test_cargo_lock_without_verification_still_skips_transient_deps() {
        let ws = TmpWorkspace::new();
        ws.write("Cargo.toml", &single_crate_manifest("0.1.0", "my-crate"));
        let lock = "version = 4\n\n[[package]]\nname = \"my-crate\"\nversion = \
                    \"0.1.0\"\n\n[[package]]\nname = \"transient-dep\"\nversion = \"0.1.0\"\n";
        ws.write("Cargo.lock", lock);

        let handler = CargoLockHandler {
            verify_old_version: false,
            ..Default::default()
        };
        let update = handler
            .prepare(&ws.path("Cargo.lock"), "0.0.0", "0.2.0")
            .unwrap()
            .unwrap();

        let result = String::from_utf8(update.updated).unwrap();
        assert!(result.contains("name = \"transient-dep\"\nversion = \"0.1.0\""));
    }
}
