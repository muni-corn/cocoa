// initialize the binary-side translation backend (shares the same locales/ dir
// as the library so all keys are available in both lib and bin code)
rust_i18n::i18n!("locales");

mod cmd;
mod style;

use anyhow::Result;
use clap::FromArgMatches;
use cocoa::{
    Config,
    cli::{Cli, Commands, MigrateSourceArg},
    i18n::{detect_locale, set_locale},
    migrate,
};
use rust_i18n::t;
use style::{
    goodbye_with_death, goodbye_with_success, print_error, print_error_bold, print_info,
    print_success_bold, welcome,
};

#[tokio::main]
async fn main() -> Result<()> {
    // detect and apply system locale before any output is produced
    let locale = detect_locale();
    set_locale(&locale);

    let matches = Cli::command_with_conditional_help().get_matches();
    let cli = Cli::from_arg_matches(&matches)
        .map_err(|e| e.exit())
        .unwrap();

    if cli.no_color {
        console::set_colors_enabled(false);
    }

    let config = match cli.config.as_deref() {
        Some(path) => Config::load_or_default(path),
        None => Config::load_discovered_or_default(),
    };

    match cli.command {
        Commands::Lint { input, stdin } => {
            welcome(t!("main.lint.welcome"));
            cmd::lint::handle_lint(
                &config,
                input,
                stdin,
                cli.json,
                cli.quiet,
                cli.verbose,
                cli.dry_run,
            )?;
        }
        Commands::Init => {
            welcome(t!("main.init.welcome"));
            cmd::init::handle_init(cli.dry_run)?;
        }
        Commands::Commit => {
            welcome(t!("main.commit.welcome"));
            cmd::commit::handle_commit(&config, cli.dry_run)?;
        }
        Commands::Generate => {
            welcome(t!("main.generate.welcome"));
            cmd::generate::handle_generate(&config, cli.json, cli.quiet, cli.verbose, cli.dry_run)
                .await?;
        }
        Commands::Changelog {
            range,
            format,
            output,
        } => {
            if !cli.json {
                welcome(t!("main.changelog.welcome"));
            }
            cmd::changelog::handle_changelog(
                &config,
                range.as_deref(),
                format.as_deref(),
                output.as_deref(),
                cli.json,
                cli.dry_run,
            )?;
        }
        Commands::Bump { bump_type } => {
            if !cli.json {
                welcome(t!("main.bump.welcome"));
            }
            cmd::bump::handle_bump(&config, bump_type.as_deref(), cli.json, cli.dry_run)?;
        }
        Commands::Hook => {
            welcome(t!("main.hook.welcome"));
            cmd::hook::handle_hook(&config, cli.dry_run)?;
        }
        Commands::Unhook => {
            welcome(t!("main.unhook.welcome"));
            cmd::unhook::handle_unhook(&config, cli.dry_run)?;
        }
        Commands::Tag { version } => {
            if !cli.json {
                welcome(t!("main.tag.welcome"));
            }
            cmd::tag::handle_tag(&config, version.as_deref(), cli.json, cli.dry_run)?;
        }
        Commands::Release {
            bump_type,
            skip_changelog,
            skip_commit,
            skip_tag,
        } => {
            if !cli.json {
                welcome(t!("main.release.welcome"));
            }
            cmd::release::handle_release(
                &config,
                bump_type.as_deref(),
                skip_changelog,
                skip_commit,
                skip_tag,
                cli.json,
                cli.dry_run,
            )?;
        }
        Commands::Migrate { from, undo } => {
            welcome(t!("main.migrate.welcome"));
            handle_migrate(from, undo, cli.dry_run)?;
        }
    }

    Ok(())
}

/// Migrate a third-party tool's configuration to `.cocoa.toml`.
///
/// Detects or uses the specified source, parses the config, writes
/// `.cocoa.toml`, and backs up any existing config. In dry-run mode the
/// converted TOML is printed but not written. The `--undo` flag restores
/// `.cocoa.toml` from its backup.
fn handle_migrate(from: Option<MigrateSourceArg>, undo: bool, dry_run: bool) -> Result<()> {
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
