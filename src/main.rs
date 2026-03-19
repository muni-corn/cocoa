// initialize the binary-side translation backend (shares the same locales/ dir
// as the library so all keys are available in both lib and bin code)
rust_i18n::i18n!("locales");

mod cmd;
mod style;

use std::process;

use anyhow::Result;
use clap::FromArgMatches;
use cocoa::{
    Config,
    changelog::{self, OutputFormat},
    cli::{Cli, Commands, MigrateSourceArg},
    git_ops::{Git2Ops, GitOperations},
    hook,
    i18n::{detect_locale, set_locale},
    migrate, release, tag, version,
};
use rust_i18n::t;
use style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error, print_error_bold,
    print_info, print_success_bold, print_warning, welcome,
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
            handle_changelog(
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
            handle_bump(&config, bump_type.as_deref(), cli.json, cli.dry_run)?;
        }
        Commands::Hook => {
            welcome(t!("main.hook.welcome"));
            handle_hook(&config, cli.dry_run)?;
        }
        Commands::Unhook => {
            welcome(t!("main.unhook.welcome"));
            handle_unhook(&config, cli.dry_run)?;
        }
        Commands::Tag { version } => {
            if !cli.json {
                welcome(t!("main.tag.welcome"));
            }
            handle_tag(&config, version.as_deref(), cli.json, cli.dry_run)?;
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
            handle_release(
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

/// Installs the cocoa `commit-msg` git hook.
///
/// Resolves the hooks directory from the current git repository and delegates
/// to [`hook::install`]. Reports the outcome to the user and exits with an
/// appropriate code.
fn handle_hook(_config: &Config, dry_run: bool) -> Result<()> {
    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let hooks_dir = match git_ops.get_hook_path() {
        Ok(p) => p,
        Err(e) => {
            print_error_bold(t!("main.git.hook_path_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    match hook::install(&hooks_dir, dry_run) {
        Ok(hook::InstallOutcome::Installed { hook_path }) => {
            if dry_run {
                print_info(t!(
                    "main.hook.dry_run_install",
                    path = hook_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.hook.installed",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::InstallOutcome::Updated { hook_path }) => {
            if dry_run {
                print_info(t!(
                    "main.hook.dry_run_update",
                    path = hook_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.hook.updated",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::InstallOutcome::Replaced {
            hook_path,
            backup_path,
        }) => {
            if dry_run {
                print_info(t!(
                    "main.hook.dry_run_replace",
                    backup = backup_path.display().to_string()
                ));
            } else {
                print_warning(t!(
                    "main.hook.replaced_backup",
                    path = backup_path.display().to_string()
                ));
                print_success_bold(t!(
                    "main.hook.installed",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Err(hook::HookError::NotAGitRepo) => {
            print_error_bold(t!("main.hook.not_git_repo"));
            goodbye_with_death(5);
        }
        Err(e) => {
            print_error_bold(t!("main.hook.install_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}

/// Removes the cocoa `commit-msg` git hook, restoring a backup if present.
///
/// Resolves the hooks directory from the current git repository and delegates
/// to [`hook::uninstall`]. Reports the outcome to the user and exits with an
/// appropriate code.
fn handle_unhook(_config: &Config, dry_run: bool) -> Result<()> {
    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let hooks_dir = match git_ops.get_hook_path() {
        Ok(p) => p,
        Err(e) => {
            print_error_bold(t!("main.git.hook_path_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    match hook::uninstall(&hooks_dir, dry_run) {
        Ok(hook::UninstallOutcome::Removed { hook_path }) => {
            if dry_run {
                print_info(t!(
                    "main.unhook.dry_run_remove",
                    path = hook_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.unhook.removed",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::UninstallOutcome::Restored {
            hook_path,
            backup_path,
        }) => {
            if dry_run {
                print_info(t!(
                    "main.unhook.dry_run_restore",
                    hook = hook_path.display().to_string(),
                    backup = backup_path.display().to_string()
                ));
            } else {
                print_success_bold(t!(
                    "main.unhook.restored",
                    path = hook_path.display().to_string()
                ));
            }
            goodbye_with_success();
        }
        Ok(hook::UninstallOutcome::NotInstalled) => {
            print_warning(t!("main.unhook.not_installed"));
            goodbye_with_warning();
        }
        Err(hook::HookError::NotAGitRepo) => {
            print_error_bold(t!("main.hook.not_git_repo"));
            goodbye_with_death(5);
        }
        Err(hook::HookError::NotManagedByCocoa) => {
            print_error_bold(t!("main.unhook.not_managed"));
            goodbye_with_death(1);
        }
        Err(e) => {
            print_error_bold(t!("main.unhook.remove_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}

/// Bump the project version and update configured version files.
///
/// Accepts an explicit bump type (major, minor, patch) or "auto" to detect
/// the appropriate bump from commits since the last version tag. In dry-run
/// mode the new version is displayed but no files are written.
fn handle_bump(
    config: &Config,
    bump_type_str: Option<&str>,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let v_config = config.version.clone().unwrap_or_default();

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    // detect the current version from git tags; default to 0.0.0 if none exist
    let current_version = match version::detect_current_semver(&git_ops, &v_config.tag_prefix) {
        Ok(Some(v)) => v,
        Ok(None) => {
            print_info(t!("main.bump.no_tags"));
            version::SemVer::parse("0.0.0").expect("0.0.0 is always valid semver")
        }
        Err(e) => {
            print_error_bold(t!("main.bump.detect_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    // resolve the bump type from the argument or auto-detect from commits
    let bump_type = match bump_type_str {
        Some("major") => version::BumpType::Major,
        Some("minor") => version::BumpType::Minor,
        Some("patch") => version::BumpType::Patch,
        Some("auto") | None => {
            // collect commits since the last version tag for analysis
            let commits = match version::detect_latest_tag(&git_ops, &v_config.tag_prefix) {
                Ok(Some(tag)) => git_ops
                    .get_commits_in_range(&tag.target, "HEAD")
                    .unwrap_or_default(),
                // no tags yet — scan all commits reachable from HEAD
                Ok(None) | Err(_) => git_ops.get_commits_in_range("", "HEAD").unwrap_or_default(),
            };
            let detected = version::detect_bump_type(&commits);
            let label = match detected {
                version::BumpType::Major => "major",
                version::BumpType::Minor => "minor",
                version::BumpType::Patch => "patch",
            };
            print_info(t!("main.bump.auto_detected", bump_type = label));
            detected
        }
        Some(other) => {
            print_error_bold(t!("main.bump.unknown_type", bump_type = other));
            goodbye_with_death(1);
        }
    };

    let new_version = match bump_type {
        version::BumpType::Major => current_version.bump_major(),
        version::BumpType::Minor => current_version.bump_minor(),
        version::BumpType::Patch => current_version.bump_patch(),
    };

    let old_str = current_version.to_string();
    let new_str = new_version.to_string();

    let bump_label = match bump_type {
        version::BumpType::Major => "major",
        version::BumpType::Minor => "minor",
        version::BumpType::Patch => "patch",
    };

    if !json_output {
        print_info(t!("main.bump.version_arrow", old = old_str, new = new_str));
    }

    let files: &[String] = v_config.commit_version_files.as_deref().unwrap_or(&[]);

    if files.is_empty() {
        if json_output {
            let out = serde_json::json!({
                "old_version": old_str,
                "new_version": new_str,
                "bump_type": bump_label,
                "dry_run": dry_run,
                "files": serde_json::Value::Array(vec![]),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else if dry_run {
            print_info(t!("main.bump.dry_run_no_files"));
            goodbye_with_warning();
        } else {
            print_warning(t!("main.bump.no_files"));
            goodbye_with_warning();
        }
        return Ok(());
    }

    if dry_run {
        if json_output {
            let out = serde_json::json!({
                "old_version": old_str,
                "new_version": new_str,
                "bump_type": bump_label,
                "dry_run": true,
                "files": files,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            print_info(t!("main.bump.dry_run_would_update", count = files.len()));
            for f in files {
                print_info(t!("main.bump.dry_run_file", file = f));
            }
            goodbye_with_success();
        }
    } else {
        match version::update_version_files(files, &old_str, &new_str) {
            Ok(()) => {
                if json_output {
                    let out = serde_json::json!({
                        "old_version": old_str,
                        "new_version": new_str,
                        "bump_type": bump_label,
                        "dry_run": false,
                        "files": files,
                    });
                    println!("{}", serde_json::to_string_pretty(&out)?);
                } else {
                    print_success_bold(t!("main.bump.bumped", old = old_str, new = new_str));
                    for f in files {
                        print_info(t!("main.bump.updated_file", file = f));
                    }
                    goodbye_with_success();
                }
            }
            Err(e) => {
                if json_output {
                    let out = serde_json::json!({
                        "success": false,
                        "error": e.to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&out)?);
                    process::exit(1);
                }
                print_error_bold(t!("main.bump.update_failed", error = e.to_string()));
                goodbye_with_death(1);
            }
        }
    }

    Ok(())
}

/// Create an annotated version tag with the changelog as its message.
///
/// Resolves the target version (from the argument or by auto-detecting the
/// appropriate bump from commits since the last tag), verifies uniqueness, and
/// creates the tag. In dry-run mode the tag name and message are printed
/// without writing to git.
fn handle_tag(
    config: &Config,
    version_str: Option<&str>,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let v_config = config.version.clone().unwrap_or_default();
    let cl_config = config.changelog.clone().unwrap_or_default();

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let version = match tag::resolve_version(&git_ops, version_str, &v_config) {
        Ok(v) => v,
        Err(tag::TagError::Version(e)) => {
            print_error_bold(t!("main.tag.invalid_version", error = e.to_string()));
            goodbye_with_death(1);
        }
        Err(e) => {
            print_error_bold(t!("main.tag.resolve_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    };

    if !json_output {
        let tag_name = format!("{}{}", v_config.tag_prefix, version);
        print_info(t!("main.tag.preparing", name = tag_name));
    }

    match tag::create_version_tag(&git_ops, &version, &v_config, &cl_config, dry_run) {
        Ok((name, message)) => {
            if json_output {
                let out = serde_json::json!({
                    "tag_name": name,
                    "version": version.to_string(),
                    "dry_run": dry_run,
                    "message": message,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else if dry_run {
                print_info(t!("main.tag.dry_run", name = name));
                println!("\n{}\n", message);
                goodbye_with_success();
            } else {
                print_success_bold(t!("main.tag.created", name = name));
                goodbye_with_success();
            }
        }
        Err(tag::TagError::AlreadyExists(name)) => {
            if json_output {
                let out = serde_json::json!({
                    "success": false,
                    "error": format!("tag '{}' already exists", name)
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(1);
            }
            print_error_bold(t!("main.tag.already_exists", name = name));
            print_info(t!("main.tag.already_exists_hint"));
            goodbye_with_death(1);
        }
        Err(tag::TagError::Git(msg)) => {
            if json_output {
                let out = serde_json::json!({ "success": false, "error": msg });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(5);
            }
            print_error_bold(t!("main.tag.git_failed", error = msg));
            goodbye_with_death(5);
        }
        Err(e) => {
            if json_output {
                let out = serde_json::json!({ "success": false, "error": e.to_string() });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(1);
            }
            print_error_bold(t!("main.tag.failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    }

    Ok(())
}

/// Execute the full release workflow: bump version, update files, write
/// changelog, commit, and tag.
///
/// In dry-run mode the plan is shown without making any changes.
fn handle_release(
    config: &Config,
    bump_type: Option<&str>,
    skip_changelog: bool,
    skip_commit: bool,
    skip_tag: bool,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let v_config = config.version.clone().unwrap_or_default();
    let cl_config = config.changelog.clone().unwrap_or_default();

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let opts = release::ReleaseOptions {
        bump_type: bump_type.map(|s| s.to_string()),
        dry_run,
        skip_changelog,
        skip_commit,
        skip_tag,
    };

    match release::execute(&git_ops, &v_config, &cl_config, &opts) {
        Ok(outcome) => {
            let bump_label = match outcome.bump_type {
                version::BumpType::Major => "major",
                version::BumpType::Minor => "minor",
                version::BumpType::Patch => "patch",
            };
            if json_output {
                let out = serde_json::json!({
                    "success": true,
                    "previous_version": outcome.previous_version,
                    "new_version": outcome.new_version,
                    "tag_name": outcome.tag_name,
                    "bump_type": bump_label,
                    "dry_run": dry_run,
                    "updated_files": outcome.updated_files,
                    "changelog_path": outcome.changelog_path,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else if dry_run {
                print_info(t!(
                    "main.release.dry_run_summary",
                    old = outcome.previous_version,
                    new = outcome.new_version,
                    bump_type = bump_label
                ));
                if !outcome.updated_files.is_empty() {
                    print_info(t!("main.release.dry_run_update_files"));
                    for f in &outcome.updated_files {
                        print_info(t!("main.release.dry_run_file", file = f));
                    }
                }
                if !skip_changelog {
                    print_info(t!(
                        "main.release.dry_run_changelog",
                        path = outcome.changelog_path
                    ));
                }
                if !skip_commit {
                    print_info(t!(
                        "main.release.dry_run_commit",
                        version = outcome.new_version
                    ));
                }
                if !skip_tag {
                    print_info(t!("main.release.dry_run_tag", name = outcome.tag_name));
                }
            } else {
                print_success_bold(t!(
                    "main.release.success",
                    old = outcome.previous_version,
                    new = outcome.new_version
                ));
                if !outcome.updated_files.is_empty() {
                    for f in &outcome.updated_files {
                        print_info(t!("main.release.updated_file", file = f));
                    }
                }
                if !skip_changelog {
                    print_info(t!(
                        "main.release.wrote_changelog",
                        path = outcome.changelog_path
                    ));
                }
                if !skip_tag {
                    print_info(t!("main.release.created_tag", name = outcome.tag_name));
                }
            }
            goodbye_with_success();
        }
        Err(e) => {
            if json_output {
                let (exit_code, error_msg) = match &e {
                    release::ReleaseError::Git(msg) => (5, msg.clone()),
                    _ => (1, e.to_string()),
                };
                let out = serde_json::json!({ "success": false, "error": error_msg });
                println!("{}", serde_json::to_string_pretty(&out)?);
                process::exit(exit_code);
            }
            match e {
                release::ReleaseError::InvalidBumpType(s) => {
                    print_error_bold(t!("main.release.invalid_bump", bump_type = s));
                    goodbye_with_death(1);
                }
                release::ReleaseError::Tag(tag::TagError::AlreadyExists(name)) => {
                    print_error_bold(t!("main.release.tag_exists", name = name));
                    print_info(t!("main.release.tag_exists_hint"));
                    goodbye_with_death(1);
                }
                release::ReleaseError::Git(msg) => {
                    print_error_bold(t!("main.release.git_failed", error = msg));
                    goodbye_with_death(5);
                }
                _ => {
                    print_error_bold(t!("main.release.failed", error = e.to_string()));
                    goodbye_with_death(1);
                }
            }
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

/// Generate a changelog from git history and write or print it.
///
/// In dry-run mode the output is printed to stdout instead of being written to
/// the configured file. When no format is specified, Markdown is used.
fn handle_changelog(
    config: &Config,
    range: Option<&str>,
    format_str: Option<&str>,
    output_path: Option<&str>,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let cl_config = config.changelog.clone().unwrap_or_default();

    let format = match format_str {
        Some(s) => match OutputFormat::parse(s) {
            Some(f) => f,
            None => {
                print_error_bold(t!("main.changelog.unknown_format", format = s));
                goodbye_with_death(1);
            }
        },
        None => OutputFormat::Markdown,
    };

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let cl = match changelog::parser::parse_history(&git_ops, range, &cl_config) {
        Ok(c) => c,
        Err(changelog::ChangelogError::Git(msg)) => {
            print_error_bold(t!("main.changelog.git_failed", error = msg));
            goodbye_with_death(5);
        }
        Err(e) => {
            print_error_bold(t!("main.changelog.failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    };

    if cl.versions.is_empty() {
        print_warning(t!("main.changelog.empty"));
        goodbye_with_warning();
        return Ok(());
    }

    let rendered = match changelog::renderer::render(&cl, &format, &cl_config) {
        Ok(s) => s,
        Err(e) => {
            print_error_bold(t!("main.changelog.render_failed", error = e.to_string()));
            goodbye_with_death(1);
        }
    };

    let dest = output_path.unwrap_or(&cl_config.output_file);

    if json_output {
        // wrap the rendered content in a JSON envelope for machine consumption
        let out = serde_json::json!({
            "content": rendered,
            "format": format_str.unwrap_or("markdown"),
            "path": dest,
            "dry_run": dry_run,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else if dry_run {
        print_info(t!("main.changelog.dry_run", path = dest));
        println!("\n{}", rendered);
        goodbye_with_success();
    } else {
        std::fs::write(dest, &rendered)
            .map_err(|e| anyhow::anyhow!("failed to write '{}': {}", dest, e))?;
        print_success_bold(t!("main.changelog.wrote", path = dest));
        goodbye_with_success();
    }

    Ok(())
}
