use anyhow::Result;
use clap::{Args, ValueEnum};
use rust_i18n::t;

use crate::{
    migrate,
    style::{
        goodbye_with_death, goodbye_with_success, print_error, print_error_bold, print_info,
        print_success_bold,
    },
};

/// The third-party tool to migrate from, as supplied on the command line.
#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum MigrateSourceArg {
    /// Migrate from a commitlint configuration file.
    Commitlint,
    /// Migrate from a conventional-changelog configuration file.
    #[value(name = "conventional-changelog")]
    ConventionalChangelog,
    /// Migrate from a semantic-release configuration file.
    #[value(name = "semantic-release")]
    SemanticRelease,
}

#[derive(Args)]
pub struct MigrateArgs {
    /// Source tool to migrate from.
    ///
    /// One of: commitlint, conventional-changelog, semantic-release.
    /// When omitted, the source is auto-detected by looking for known
    /// configuration files in the current directory.
    #[arg(
        long,
        value_enum,
        value_name = "TOOL",
        help = "Source tool to migrate from (auto-detected if omitted)"
    )]
    pub from: Option<MigrateSourceArg>,

    /// Restore the previous `.cocoa.toml` from the backup.
    ///
    /// Renames `.cocoa.toml.bak` back to `.cocoa.toml`. Use this to
    /// undo a migration.
    #[arg(long, help = "Undo migration by restoring .cocoa.toml.bak")]
    pub undo: bool,
}

/// Migrate a third-party tool's configuration to `.cocoa.toml`.
///
/// Detects or uses the specified source, parses the config, writes
/// `.cocoa.toml`, and backs up any existing config. In dry-run mode the
/// converted TOML is printed but not written. The `--undo` flag restores
/// `.cocoa.toml` from its backup.
pub fn handle_migrate(from: Option<MigrateSourceArg>, undo: bool, dry_run: bool) -> Result<()> {
    if undo {
        match migrate::rollback() {
            Ok(path) => {
                print_success_bold(t!(
                    "main.migrate.rolled_back",
                    path = path.display().to_string()
                ));
                goodbye_with_success();
            }
            Err(migrate::MigrateError::NoBackupFound) => {
                print_error_bold(t!("main.migrate.no_backup"));
                goodbye_with_death(1);
            }
            Err(e) => {
                print_error_bold(t!("main.migrate.rollback_failed", error = e.to_string()));
                goodbye_with_death(1);
            }
        }
        return Ok(());
    }

    // convert the CLI enum to the library enum
    let source = from.map(|s| match s {
        MigrateSourceArg::Commitlint => migrate::MigrateSource::Commitlint,
        MigrateSourceArg::ConventionalChangelog => migrate::MigrateSource::ConventionalChangelog,
        MigrateSourceArg::SemanticRelease => migrate::MigrateSource::SemanticRelease,
    });

    match migrate::migrate(source, dry_run) {
        Ok(result) => {
            if dry_run {
                print_info(t!(
                    "main.migrate.dry_run",
                    source = result.source.to_string(),
                    file = result.source_file.display().to_string()
                ));
                // print the converted config as TOML
                match toml::to_string_pretty(&result.config) {
                    Ok(toml_str) => println!("\n{}", toml_str),
                    Err(e) => {
                        print_error_bold(t!(
                            "main.migrate.serialize_failed",
                            error = e.to_string()
                        ));
                        goodbye_with_death(1);
                    }
                }
            } else {
                print_success_bold(t!(
                    "main.migrate.success",
                    source = result.source.to_string(),
                    file = result.source_file.display().to_string(),
                    output = result.output_file.display().to_string()
                ));
                if let Some(backup) = result.backup_file {
                    print_info(t!(
                        "main.migrate.backed_up",
                        path = backup.display().to_string()
                    ));
                }
            }
            goodbye_with_success();
        }
        Err(migrate::MigrateError::NoSourceFound) => {
            print_error_bold(t!("main.migrate.no_source"));
            print_info(t!("main.migrate.no_source_hint"));
            goodbye_with_death(1);
        }
        Err(migrate::MigrateError::Parse(msg)) => {
            print_error_bold(t!("main.migrate.parse_failed"));
            print_error(&msg);
            goodbye_with_death(1);
        }
        Err(e) => {
            print_error_bold(t!("main.migrate.failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}
