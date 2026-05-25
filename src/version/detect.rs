//! Basename-based handler auto-detection.
//!
//! Maps known file basenames to their corresponding `FileEntryKind` values.
//! Used when `kind = "auto"` (the default) to avoid requiring users to
//! explicitly declare the handler for well-known files.
//!
//! Detection is intentionally conservative: unknown basenames fall back to
//! `FileEntryKind::Plain`, preserving historical behavior.

use crate::{config::FileEntryKind, version::handlers::basename};

/// Infer the appropriate `FileEntryKind` from a file path's basename.
///
/// Returns `FileEntryKind::Plain` for unrecognized files, which preserves
/// the historical behavior of global string replacement.
///
/// # Examples
/// ```ignore
/// assert_eq!(infer_kind("Cargo.toml"), FileEntryKind::Cargo);
/// assert_eq!(infer_kind("Cargo.lock"), FileEntryKind::CargoLock);
/// assert_eq!(infer_kind("package.json"), FileEntryKind::Npm);
/// assert_eq!(infer_kind("package-lock.json"), FileEntryKind::NpmLock);
/// assert_eq!(infer_kind("pnpm-lock.yaml"), FileEntryKind::PnpmLock);
/// assert_eq!(infer_kind("yarn.lock"), FileEntryKind::YarnLock);
/// assert_eq!(infer_kind("pyproject.toml"), FileEntryKind::Pyproject);
/// assert_eq!(infer_kind("README.md"), FileEntryKind::Plain);
/// ```
pub fn infer_kind(path: &str) -> FileEntryKind {
    match basename(path) {
        "Cargo.toml" => FileEntryKind::Cargo,
        "Cargo.lock" => FileEntryKind::CargoLock,
        "package.json" => FileEntryKind::Npm,
        "package-lock.json" => FileEntryKind::NpmLock,
        "pnpm-lock.yaml" | "pnpm-lock.yml" => FileEntryKind::PnpmLock,
        "yarn.lock" => FileEntryKind::YarnLock,
        "pyproject.toml" => FileEntryKind::Pyproject,
        _ => FileEntryKind::Plain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_cargo_toml() {
        assert_eq!(infer_kind("Cargo.toml"), FileEntryKind::Cargo);
        assert_eq!(infer_kind("path/to/Cargo.toml"), FileEntryKind::Cargo);
    }

    #[test]
    fn test_detects_cargo_lock() {
        assert_eq!(infer_kind("Cargo.lock"), FileEntryKind::CargoLock);
    }

    #[test]
    fn test_detects_package_json() {
        assert_eq!(infer_kind("package.json"), FileEntryKind::Npm);
        assert_eq!(infer_kind("apps/web/package.json"), FileEntryKind::Npm);
    }

    #[test]
    fn test_detects_package_lock_json() {
        assert_eq!(infer_kind("package-lock.json"), FileEntryKind::NpmLock);
    }

    #[test]
    fn test_detects_pnpm_lock() {
        assert_eq!(infer_kind("pnpm-lock.yaml"), FileEntryKind::PnpmLock);
        assert_eq!(infer_kind("pnpm-lock.yml"), FileEntryKind::PnpmLock);
    }

    #[test]
    fn test_detects_yarn_lock() {
        assert_eq!(infer_kind("yarn.lock"), FileEntryKind::YarnLock);
    }

    #[test]
    fn test_detects_pyproject_toml() {
        assert_eq!(infer_kind("pyproject.toml"), FileEntryKind::Pyproject);
    }

    #[test]
    fn test_unknown_falls_back_to_plain() {
        assert_eq!(infer_kind("README.md"), FileEntryKind::Plain);
        assert_eq!(infer_kind("version.txt"), FileEntryKind::Plain);
        assert_eq!(infer_kind("some_manifest.xml"), FileEntryKind::Plain);
    }

    #[test]
    fn test_bare_filename_works() {
        assert_eq!(infer_kind("Cargo.toml"), FileEntryKind::Cargo);
    }
}
