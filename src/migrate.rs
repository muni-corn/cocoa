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

    /// No `.cocoa.toml.bak` file was found — nothing to roll back to.
    #[error("no backup found at '.cocoa.toml.bak' — run `cocoa migrate` first")]
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
