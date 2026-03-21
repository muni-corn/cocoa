// initialize the binary-side translation backend (shares the same locales/ dir
// as the library so all keys are available in both lib and bin code)
rust_i18n::i18n!("locales");

use anyhow::Result;
use clap::FromArgMatches;
use cocoa::{
    cli::Cli,
    cmd::{
        self, Command,
        bump::BumpArgs,
        changelog::ChangelogArgs,
        lint::{LintArgs, handle_lint},
        man::handle_man,
        migrate::MigrateArgs,
        release::ReleaseArgs,
        tag::TagArgs,
    },
    config::Config,
    i18n::{detect_locale, set_locale},
    style::welcome,
};
use rust_i18n::t;

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
        Command::Lint(LintArgs { input, stdin }) => {
            welcome(t!("main.lint.welcome"));
            handle_lint(
                &config,
                input,
                stdin,
                cli.json,
                cli.quiet,
                cli.verbose,
                cli.dry_run,
            )?;
        }
        Command::Init => {
            welcome(t!("main.init.welcome"));
            cmd::init::handle_init(cli.dry_run)?;
        }
        Command::Commit => {
            welcome(t!("main.commit.welcome"));
            cmd::commit::handle_commit(&config, cli.dry_run)?;
        }
        Command::Generate => {
            welcome(t!("main.generate.welcome"));
            cmd::generate::handle_generate(&config, cli.json, cli.quiet, cli.verbose, cli.dry_run)
                .await?;
        }
        Command::Changelog(ChangelogArgs {
            range,
            format,
            output,
        }) => {
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
        Command::Bump(BumpArgs { bump_type }) => {
            if !cli.json {
                welcome(t!("main.bump.welcome"));
            }
            cmd::bump::handle_bump(&config, bump_type.as_deref(), cli.json, cli.dry_run)?;
        }
        Command::Hook => {
            welcome(t!("main.hook.welcome"));
            cmd::hook::handle_hook(&config, cli.dry_run)?;
        }
        Command::Unhook => {
            welcome(t!("main.unhook.welcome"));
            cmd::unhook::handle_unhook(&config, cli.dry_run)?;
        }
        Command::Tag(TagArgs { version }) => {
            if !cli.json {
                welcome(t!("main.tag.welcome"));
            }
            cmd::tag::handle_tag(&config, version.as_deref(), cli.json, cli.dry_run)?;
        }
        Command::Release(ReleaseArgs {
            bump_type,
            skip_changelog,
            skip_commit,
            skip_tag,
        }) => {
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
        Command::Migrate(MigrateArgs { from, undo }) => {
            welcome(t!("main.migrate.welcome"));
            cmd::migrate::handle_migrate(from, undo, cli.dry_run)?;
        }

        Command::Man => handle_man()?,
    }

    Ok(())
}
