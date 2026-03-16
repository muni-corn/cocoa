#![allow(dead_code)]

mod cli;
mod style;

use std::{
    io::{self, Read},
    process,
};

use anyhow::Result;
use clap::FromArgMatches;
use cli::{Cli, Commands};
use cocoa::{
    Config, generate,
    git_ops::{Git2Ops, GitOperations},
    init, lint,
};
use lint::Linter;
use style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error, print_error_bold,
    print_info, print_success_bold, print_warning, print_warning_bold, welcome,
};

#[tokio::main]
async fn main() -> Result<()> {
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
            welcome("hi! checking this commit message...");
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
        Commands::Init => {
            welcome("cocoa init");
            match init::init(cli.dry_run) {
                Ok(()) => {
                    if cli.dry_run {
                        print_info("dry run complete — no file was written");
                    } else {
                        print_success_bold("wrote .cocoa.toml");
                    }
                    goodbye_with_success();
                }
                Err(init::InitError::Aborted) => {
                    print_warning("init cancelled");
                    goodbye_with_warning();
                }
                Err(init::InitError::FileExists) => {
                    print_error_bold(".cocoa.toml already exists");
                    print_info("delete it or run interactively to overwrite");
                    goodbye_with_death(1);
                }
                Err(e) => {
                    print_error_bold(format!("init failed: {}", e));
                    goodbye_with_death(1);
                }
            }
        }
        Commands::Commit => {
            welcome("cocoa");
            print_error_bold("interactive commit creation not yet implemented");
        }
        Commands::Generate => {
            welcome("hi! generating your commit message...");
            handle_generate(&config, cli.json, cli.quiet, cli.verbose, cli.dry_run).await?;
        }
        Commands::Changelog { range: _ } => {
            welcome("cocoa");
            print_error_bold("changelog generation not yet implemented");
        }
        Commands::Bump { bump_type: _ } => {
            welcome("cocoa");
            print_error_bold("version bumping not yet implemented");
        }
        Commands::Hook => {
            welcome("cocoa hook");
        }
        Commands::Unhook => {
            welcome("cocoa unhook");
        }
        Commands::Tag => {
            welcome("cocoa");
            print_error_bold("git tagging not yet implemented");
        }
        Commands::Release => {
            welcome("cocoa");
            print_error_bold("release management not yet implemented");
        }
    }

    Ok(())
}

fn handle_lint(
    config: &Config,
    input: Option<String>,
    stdin: bool,
    json_output: bool,
    quiet: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    let linter = Linter::new(config);

    let message: String = if stdin {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    } else if let Some(input_str) = input {
        let path = std::path::Path::new(&input_str);
        if path.exists() && path.is_file() {
            // treat input as a file path containing a commit message
            let contents = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("failed to read '{}': {}", input_str, e))?;
            if verbose {
                print_info(format!("reading commit message from file: {}", input_str));
            }
            contents.trim().to_string()
        } else if input_str.contains("..") {
            // treat input as a git range (e.g., HEAD~5..HEAD)
            return handle_range_lint(&linter, &input_str, json_output, quiet, verbose, dry_run);
        } else {
            // treat input as a raw commit message string
            input_str
        }
    } else {
        print_error_bold("um... i need a commit message to work with!");
        print_info("pass a commit message, a file path, or a git range (e.g., HEAD~5..HEAD)");
        print_info("or read stdin with `--stdin`");
        goodbye_with_death(1);
    };

    if verbose {
        print_info(format!("linting message ({} chars):", message.len()));
        for line in message.lines() {
            print_info(format!("  {}", line));
        }
    }

    let result = linter.lint(&message);
    output_single_lint_result(&result, json_output, quiet, dry_run)
}

/// Output the result of linting a single commit message, respecting output
/// flags.
///
/// Exits the process with code 3 if the message is invalid and dry-run is not
/// set.
fn output_single_lint_result(
    result: &lint::LintResult,
    json_output: bool,
    quiet: bool,
    dry_run: bool,
) -> Result<()> {
    if json_output {
        println!("{}", serde_json::to_string(result)?);
    } else if !quiet {
        if result.violations.is_empty() {
            print_success_bold("commit message is valid!");
            goodbye_with_success();
        } else {
            let error_count = result
                .violations
                .iter()
                .filter(|v| matches!(v.severity, lint::Severity::Error))
                .count();

            let warning_count = result
                .violations
                .iter()
                .filter(|v| matches!(v.severity, lint::Severity::Warning))
                .count();

            if error_count > 0 {
                print_error_bold("commit message has errors:");
            } else if warning_count > 0 {
                print_warning_bold("commit message is valid, but there are some warnings:");
            }

            for violation in &result.violations {
                let print_fn = match violation.severity {
                    lint::Severity::Error => print_error,
                    lint::Severity::Warning => print_warning,
                    lint::Severity::Info => print_info,
                };
                print_fn(format!("[{}] {}", violation.rule, violation.message));
            }

            if error_count > 0 {
                if dry_run {
                    print_info("dry-run mode: errors detected but not failing");
                    goodbye_with_warning();
                } else {
                    goodbye_with_death(3);
                }
            } else {
                goodbye_with_warning();
            }
        }
    }

    if !result.is_valid && !dry_run {
        process::exit(3);
    }

    Ok(())
}

/// Lint all commits in a git range (e.g., `HEAD~5..HEAD`).
///
/// Parses the `from..to` syntax, walks the range with libgit2, lints each
/// commit subject, and reports per-commit results. Exits with code 3 if any
/// commit fails, unless `dry_run` is set.
fn handle_range_lint(
    linter: &Linter,
    range: &str,
    json_output: bool,
    quiet: bool,
    verbose: bool,
    dry_run: bool,
) -> Result<()> {
    // parse "from..to" — everything before the first ".." is `from`
    let (from, to) = if let Some((f, t)) = range.split_once("..") {
        (f, t)
    } else {
        print_error_bold(format!("invalid git range: '{}'", range));
        goodbye_with_death(1);
    };

    if verbose {
        print_info(format!("linting commits in range '{}..{}'", from, to));
    }

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(format!("failed to open git repository: {}", e));
            goodbye_with_death(5);
        }
    };

    let commits = match git_ops.get_commits_in_range(from, to) {
        Ok(c) => c,
        Err(e) => {
            print_error_bold(format!("failed to get commits in range '{}': {}", range, e));
            goodbye_with_death(5);
        }
    };

    if commits.is_empty() {
        if !quiet {
            print_warning_bold("no commits found in range");
        }
        goodbye_with_warning();
        return Ok(());
    }

    if verbose {
        print_info(format!("found {} commits to lint", commits.len()));
    }

    // lint each commit's subject line
    let lint_results: Vec<_> = commits
        .into_iter()
        .map(|commit| {
            let result = linter.lint(&commit.message);
            (commit, result)
        })
        .collect();

    let invalid_count = lint_results.iter().filter(|(_, r)| !r.is_valid).count();

    if json_output {
        let json_results: Vec<serde_json::Value> = lint_results
            .iter()
            .map(|(commit, result)| {
                let short_id = commit.id.get(..8).unwrap_or(&commit.id);
                serde_json::json!({
                    "commit_id": short_id,
                    "message": commit.message,
                    "is_valid": result.is_valid,
                    "violations": result.violations,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&json_results)?);
    } else if !quiet {
        for (commit, result) in &lint_results {
            let short_id = commit.id.get(..8).unwrap_or(&commit.id);
            let commit_label = format!("[{}] {}", short_id, commit.message);

            if result.violations.is_empty() {
                print_success_bold(&commit_label);
            } else {
                let err_count = result
                    .violations
                    .iter()
                    .filter(|v| matches!(v.severity, lint::Severity::Error))
                    .count();

                if err_count > 0 {
                    print_error_bold(&commit_label);
                } else {
                    print_warning_bold(&commit_label);
                }

                for violation in &result.violations {
                    let print_fn = match violation.severity {
                        lint::Severity::Error => print_error,
                        lint::Severity::Warning => print_warning,
                        lint::Severity::Info => print_info,
                    };
                    print_fn(format!("  [{}] {}", violation.rule, violation.message));
                }
            }
        }

        if invalid_count > 0 {
            print_error_bold(format!(
                "{}/{} commit(s) failed linting",
                invalid_count,
                lint_results.len()
            ));
            if dry_run {
                print_info("dry-run mode: errors detected but not failing");
                goodbye_with_warning();
            } else {
                goodbye_with_death(3);
            }
        } else {
            print_success_bold(format!("all {} commits passed!", lint_results.len()));
            goodbye_with_success();
        }
    }

    if invalid_count > 0 && !dry_run {
        process::exit(3);
    }

    Ok(())
}

async fn handle_generate(
    config: &Config,
    json_output: bool,
    quiet: bool,
    verbose: bool,
    _dry_run: bool,
) -> Result<()> {
    // Check if AI is configured
    if config.ai.is_none() {
        print_error_bold("you don't have ai configured for me, so i can't use ai");
        print_info("add an [ai] section to your .cocoa.toml configuration");
        print_info("see the documentation for configuration examples");
        goodbye_with_death(2);
    }

    if verbose {
        print_info("calling ai to generate commit message...");
    }

    match generate::generate_commit_message(config).await {
        Ok(message) => {
            if json_output {
                let result = serde_json::json!({
                    "success": true,
                    "message": message
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if !quiet {
                print_success_bold("generated commit message:");
                println!("\n{}\n", message);

                // Ask user if they want to commit with this message
                print_info("would you like to commit with this message? (y/n)");

                use std::io::Write;
                print!("❯ ");
                io::stdout().flush()?;

                let mut response = String::new();
                io::stdin().read_line(&mut response)?;

                if response.trim().to_lowercase().starts_with('y') {
                    // Commit with the generated message
                    let output = std::process::Command::new("git")
                        .args(["commit", "-m", &message])
                        .output()?;

                    if output.status.success() {
                        print_success_bold("commit successful!");
                        goodbye_with_success();
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        print_error_bold(format!("git commit failed: {}", stderr));
                        goodbye_with_death(5);
                    }
                } else {
                    print_info("commit cancelled. you can use the generated message manually.");
                    goodbye_with_warning();
                }
            }
        }
        Err(e) => {
            if json_output {
                let result = serde_json::json!({
                    "success": false,
                    "error": e.to_string()
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
                process::exit(match e {
                    generate::GenerateError::NoStagedChanges => 1,
                    generate::GenerateError::GitContext(_) => 5,
                    generate::GenerateError::StagedChanges(_) => 5,
                    generate::GenerateError::AiGeneration(_) => 4,
                    generate::GenerateError::Validation(_) => 3,
                    generate::GenerateError::GitCommand(_) => 5,
                });
            } else {
                match &e {
                    generate::GenerateError::NoStagedChanges => {
                        print_error_bold("no staged changes found");
                        print_info("use `git add <files>` to stage changes first");
                        print_info("then run `cocoa generate` again");
                    }
                    generate::GenerateError::AiGeneration(msg) => {
                        print_error_bold("ai generation failed");
                        print_error(msg);
                        print_info("check your ai configuration and api key");
                    }
                    generate::GenerateError::GitContext(msg)
                    | generate::GenerateError::StagedChanges(msg)
                    | generate::GenerateError::GitCommand(msg) => {
                        print_error_bold("git operation failed");
                        print_error(msg);
                    }
                    generate::GenerateError::Validation(msg) => {
                        print_error_bold("generated message failed validation");
                        print_error(msg);
                        print_info("this may indicate an issue with ai configuration");
                    }
                }

                let exit_code = match e {
                    generate::GenerateError::NoStagedChanges => 1,
                    generate::GenerateError::GitContext(_) => 5,
                    generate::GenerateError::StagedChanges(_) => 5,
                    generate::GenerateError::AiGeneration(_) => 4,
                    generate::GenerateError::Validation(_) => 3,
                    generate::GenerateError::GitCommand(_) => 5,
                };
                goodbye_with_death(exit_code);
            }
        }
    }

    Ok(())
}
