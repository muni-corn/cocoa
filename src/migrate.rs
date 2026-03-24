//! Migration tools for converting third-party tool configurations to
//! `.cocoa.toml`.
//!
//! Supports migrating from:
//! - commitlint (`.commitlintrc.*`, `commitlint.config.*`)
//! - conventional-changelog (`.changelog.config.*`, `changelog.config.*`)
//! - semantic-release (`.releaserc.*`, `release.config.*`)
//!
//! Any existing `.cocoa.toml` is backed up to `.cocoa.toml.bak` before
//! writing. Use [`rollback`] to restore a backup.

use std::path::PathBuf;

use thiserror::Error;

use crate::config::Config;

pub mod commitlint;
pub mod conventional_changelog;
pub mod semantic_release;

// output and backup file names
const COCOA_CONFIG_FILE: &str = ".cocoa.toml";
const COCOA_BACKUP_FILE: &str = ".cocoa.toml.bak";

/// Errors that can occur during migration or rollback.
#[derive(Debug, Error)]
pub enum MigrateError {
    /// No supported configuration file was detected in the current directory.
    #[error("no supported configuration file found in the current directory")]
    NoSourceFound,

    /// A source configuration file was found but could not be read.
    #[error("failed to read '{path}': {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The source configuration file could not be parsed.
    #[error("failed to parse configuration: {0}")]
    Parse(String),

    /// The converted configuration could not be written.
    #[error("failed to write configuration: {0}")]
    Write(String),

    /// Creating the backup of an existing `.cocoa.toml` failed.
    #[error("failed to create backup: {0}")]
    Backup(String),

    /// No `.cocoa.toml.bak` file was found; nothing to roll back to.
    #[error("no backup found at '.cocoa.toml.bak'. run `cocoa migrate` first")]
    NoBackupFound,
}

/// The third-party tool whose configuration is being migrated.
#[derive(Debug, Clone, PartialEq)]
pub enum MigrateSource {
    Commitlint,
    ConventionalChangelog,
    SemanticRelease,
}

impl std::fmt::Display for MigrateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateSource::Commitlint => write!(f, "commitlint"),
            MigrateSource::ConventionalChangelog => write!(f, "conventional-changelog"),
            MigrateSource::SemanticRelease => write!(f, "semantic-release"),
        }
    }
}

/// The result of a successful migration.
#[derive(Debug)]
pub struct MigrateResult {
    /// Which tool the configuration was migrated from.
    pub source: MigrateSource,
    /// The source configuration file that was read.
    pub source_file: PathBuf,
    /// The output `.cocoa.toml` path.
    pub output_file: PathBuf,
    /// Backup of the previous `.cocoa.toml`, if one existed.
    pub backup_file: Option<PathBuf>,
    /// The converted configuration (used for dry-run display).
    pub config: Config,
}

/// Auto-detects the first supported configuration file present in the current
/// working directory.
///
/// Checks commitlint sources first, then conventional-changelog, then
/// semantic-release. Returns the detected source type and file path, or
/// `None` if no supported file is found.
pub fn detect_source() -> Option<(MigrateSource, PathBuf)> {
    for name in commitlint::SUPPORTED_FILES {
        let p = PathBuf::from(name);
        if p.exists() {
            return Some((MigrateSource::Commitlint, p));
        }
    }

    for name in conventional_changelog::SUPPORTED_FILES {
        let p = PathBuf::from(name);
        if p.exists() {
            return Some((MigrateSource::ConventionalChangelog, p));
        }
    }

    for name in semantic_release::SUPPORTED_FILES {
        let p = PathBuf::from(name);
        if p.exists() {
            return Some((MigrateSource::SemanticRelease, p));
        }
    }

    None
}

/// Finds the first supported configuration file for a specific migration
/// source.
///
/// Returns `None` if no matching file exists in the current working directory.
pub fn find_source_file(source: &MigrateSource) -> Option<PathBuf> {
    let files: &[&str] = match source {
        MigrateSource::Commitlint => commitlint::SUPPORTED_FILES,
        MigrateSource::ConventionalChangelog => conventional_changelog::SUPPORTED_FILES,
        MigrateSource::SemanticRelease => semantic_release::SUPPORTED_FILES,
    };

    for name in files {
        let p = PathBuf::from(name);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

/// Migrate from a third-party tool's configuration to `.cocoa.toml`.
///
/// 1. Locates the source configuration file (auto-detected if `source` is
///    `None`).
/// 2. Parses and converts it to a [`Config`].
/// 3. Backs up any existing `.cocoa.toml` to `.cocoa.toml.bak`.
/// 4. Writes the new `.cocoa.toml`.
///
/// In dry-run mode no files are written or moved; the converted config is
/// returned for display purposes only.
pub fn migrate(
    source: Option<MigrateSource>,
    dry_run: bool,
) -> Result<MigrateResult, MigrateError> {
    let (source, source_file) = match source {
        Some(s) => {
            let file = find_source_file(&s).ok_or(MigrateError::NoSourceFound)?;
            (s, file)
        }
        None => detect_source().ok_or(MigrateError::NoSourceFound)?,
    };

    let config = match &source {
        MigrateSource::Commitlint => commitlint::parse(&source_file)?,
        MigrateSource::ConventionalChangelog => conventional_changelog::parse(&source_file)?,
        MigrateSource::SemanticRelease => semantic_release::parse(&source_file)?,
    };

    let output_file = PathBuf::from(COCOA_CONFIG_FILE);
    let backup_path = PathBuf::from(COCOA_BACKUP_FILE);

    if dry_run {
        return Ok(MigrateResult {
            source,
            source_file,
            output_file,
            backup_file: None,
            config,
        });
    }

    // back up any existing .cocoa.toml before overwriting
    let backup_file = if output_file.exists() {
        std::fs::copy(&output_file, &backup_path).map_err(|e| {
            MigrateError::Backup(format!(
                "could not copy '{}' to '{}': {}",
                output_file.display(),
                backup_path.display(),
                e
            ))
        })?;
        Some(backup_path)
    } else {
        None
    };

    let toml_str = toml::to_string_pretty(&config)
        .map_err(|e| MigrateError::Write(format!("failed to serialize config: {}", e)))?;

    std::fs::write(&output_file, &toml_str).map_err(|e| {
        MigrateError::Write(format!(
            "failed to write '{}': {}",
            output_file.display(),
            e
        ))
    })?;

    Ok(MigrateResult {
        source,
        source_file,
        output_file,
        backup_file,
        config,
    })
}

/// Restore the previous `.cocoa.toml` from its backup (`.cocoa.toml.bak`).
///
/// Returns the path of the restored file. Errors with
/// [`MigrateError::NoBackupFound`] if no backup exists.
pub fn rollback() -> Result<PathBuf, MigrateError> {
    let backup = PathBuf::from(COCOA_BACKUP_FILE);
    let target = PathBuf::from(COCOA_CONFIG_FILE);

    if !backup.exists() {
        return Err(MigrateError::NoBackupFound);
    }

    std::fs::rename(&backup, &target).map_err(|e| {
        MigrateError::Write(format!(
            "failed to restore '{}' from '{}': {}",
            target.display(),
            backup.display(),
            e
        ))
    })?;

    Ok(target)
}

#[cfg(test)]
mod tests {
    use std::{env, sync::Mutex};

    use tempfile::TempDir;

    use super::*;

    // serialise tests that change the process working directory; concurrent
    // set_current_dir calls from multiple threads corrupt each other's paths
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn in_temp_dir<F: FnOnce()>(f: F) {
        // hold the lock for the entire duration so no other test moves the CWD
        let _guard = CWD_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        let orig = env::current_dir().unwrap();
        env::set_current_dir(tmp.path()).unwrap();
        f();
        env::set_current_dir(orig).unwrap();
    }

    // --- MigrateSource::Display ---

    #[test]
    fn test_migrate_source_display_commitlint() {
        assert_eq!(MigrateSource::Commitlint.to_string(), "commitlint");
    }

    #[test]
    fn test_migrate_source_display_conventional_changelog() {
        assert_eq!(
            MigrateSource::ConventionalChangelog.to_string(),
            "conventional-changelog"
        );
    }

    #[test]
    fn test_migrate_source_display_semantic_release() {
        assert_eq!(
            MigrateSource::SemanticRelease.to_string(),
            "semantic-release"
        );
    }

    // --- MigrateError::Display ---

    #[test]
    fn test_migrate_error_no_source_found() {
        let err = MigrateError::NoSourceFound;
        assert!(err.to_string().contains("no supported configuration file"));
    }

    #[test]
    fn test_migrate_error_no_backup_found() {
        let err = MigrateError::NoBackupFound;
        assert!(err.to_string().contains("no backup found"));
    }

    #[test]
    fn test_migrate_error_parse() {
        let err = MigrateError::Parse("bad syntax".to_string());
        assert!(err.to_string().contains("bad syntax"));
    }

    #[test]
    fn test_migrate_error_write() {
        let err = MigrateError::Write("disk full".to_string());
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn test_migrate_error_backup() {
        let err = MigrateError::Backup("no permission".to_string());
        assert!(err.to_string().contains("no permission"));
    }

    // --- detect_source ---

    #[test]
    fn test_detect_source_returns_none_when_no_files_present() {
        in_temp_dir(|| {
            assert!(detect_source().is_none());
        });
    }

    // --- find_source_file ---

    #[test]
    fn test_find_source_file_returns_none_when_missing() {
        in_temp_dir(|| {
            assert!(find_source_file(&MigrateSource::Commitlint).is_none());
            assert!(find_source_file(&MigrateSource::ConventionalChangelog).is_none());
            assert!(find_source_file(&MigrateSource::SemanticRelease).is_none());
        });
    }

    // --- migrate ---

    #[test]
    fn test_migrate_no_source_found_returns_error() {
        in_temp_dir(|| {
            let result = migrate(None, true);
            assert!(matches!(result, Err(MigrateError::NoSourceFound)));
        });
    }

    #[test]
    fn test_migrate_explicit_source_not_found_returns_error() {
        in_temp_dir(|| {
            let result = migrate(Some(MigrateSource::Commitlint), true);
            assert!(matches!(result, Err(MigrateError::NoSourceFound)));
        });
    }

    // --- detect_source / find_source_file (success paths) ---

    #[test]
    fn test_detect_source_finds_commitlint_file() {
        in_temp_dir(|| {
            // create a minimal commitlint JSON config
            std::fs::write(".commitlintrc.json", b"{\"rules\":{}}").unwrap();
            let result = detect_source();
            assert!(result.is_some());
            let (source, _) = result.unwrap();
            assert_eq!(source, MigrateSource::Commitlint);
        });
    }

    #[test]
    fn test_find_source_file_returns_path_when_present() {
        in_temp_dir(|| {
            std::fs::write(".commitlintrc.json", b"{\"rules\":{}}").unwrap();
            let result = find_source_file(&MigrateSource::Commitlint);
            assert!(result.is_some());
        });
    }

    // --- migrate dry-run ---

    #[test]
    fn test_migrate_dry_run_with_commitlint_source() {
        in_temp_dir(|| {
            std::fs::write(".commitlintrc.json", b"{\"rules\":{}}").unwrap();
            let result = migrate(None, true);
            assert!(result.is_ok(), "migrate dry-run failed: {:?}", result);
            let mr = result.unwrap();
            assert_eq!(mr.source, MigrateSource::Commitlint);
            // in dry-run mode no output file should be written
            assert!(!PathBuf::from(COCOA_CONFIG_FILE).exists());
        });
    }

    #[test]
    fn test_migrate_writes_output_file() {
        in_temp_dir(|| {
            std::fs::write(".commitlintrc.json", b"{\"rules\":{}}").unwrap();
            let result = migrate(None, false);
            assert!(result.is_ok(), "migrate failed: {:?}", result);
            // output file should now exist
            assert!(PathBuf::from(COCOA_CONFIG_FILE).exists());
        });
    }

    #[test]
    fn test_migrate_backs_up_existing_config() {
        in_temp_dir(|| {
            std::fs::write(".commitlintrc.json", b"{\"rules\":{}}").unwrap();
            // pre-existing .cocoa.toml
            std::fs::write(COCOA_CONFIG_FILE, b"[commit]\ntypes = [\"old\"]\n").unwrap();
            let result = migrate(None, false);
            assert!(result.is_ok());
            // backup should exist
            assert!(PathBuf::from(COCOA_BACKUP_FILE).exists());
            let mr = result.unwrap();
            assert!(mr.backup_file.is_some());
        });
    }

    // --- rollback ---

    #[test]
    fn test_rollback_no_backup_returns_error() {
        in_temp_dir(|| {
            let result = rollback();
            assert!(matches!(result, Err(MigrateError::NoBackupFound)));
        });
    }

    #[test]
    fn test_rollback_restores_backup() {
        in_temp_dir(|| {
            // create a fake backup file
            std::fs::write(COCOA_BACKUP_FILE, b"[commit]\ntypes = [\"feat\"]\n").unwrap();
            let result = rollback();
            assert!(result.is_ok());
            // the target file should now exist
            assert!(PathBuf::from(COCOA_CONFIG_FILE).exists());
            // the backup should be gone
            assert!(!PathBuf::from(COCOA_BACKUP_FILE).exists());
        });
    }
}
