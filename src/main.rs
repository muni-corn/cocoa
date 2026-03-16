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
use cocoa::{Config, generate, lint};
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

    let config_path = cli.config.as_deref().unwrap_or(".cocoa.toml");
    let config = Config::load_or_default(config_path);

    match cli.command {
        Commands::Lint { input, stdin } => {
            welcome("hi! checking this commit message...");
            handle_lint(&config, input, stdin, cli.json, cli.quiet, cli.verbose)?;
        }
        Commands::Init => {
            welcome("cocoa");
            print_error_bold("init is not implemented yet");
            // TODO: Implement init command
        }
        Commands::Commit => {
            welcome("cocoa");
            print_error_bold("interactive commit creation not yet implemented");
        }
        Commands::Generate => {
            welcome("hi! generating your commit message...");
            handle_generate(&config, cli.json, cli.quiet, cli.verbose).await?;
        }
        Commands::Changelog { range: _ } => {
            welcome("cocoa");
            print_error_bold("changelog generation not yet implemented");
        }
        Commands::Bump { bump_type: _ } => {
            welcome("cocoa");
            print_error_bold("version bumping not yet implemented");
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
) -> Result<()> {
    let linter = Linter::new(config);

    let message = if stdin {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    } else if let Some(input) = input {
        if input.contains("..") {
            // TODO: handle git range
            print_error_bold("git range linting not yet implemented");
            std::process::exit(1);
        } else {
            input
        }
    } else {
        // read from git commit message if available
        print_error_bold("um... i need a commit message to work with!");
        print_info("you can pass a file containing the commit message");
        print_info("or pass a commit message directly with `--text`");
        print_info("or read stdin in with `--stdin`");
        goodbye_with_death(1);
    };

    if verbose {
        print_info(format!("linting message ({} chars):", message.len()));
        for line in message.lines() {
            print_info(format!("  {}", line));
        }
    }

    let result = linter.lint(&message);

    if json_output {
        println!("{}", serde_json::to_string(&result)?);
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
                goodbye_with_death(3);
            } else {
                goodbye_with_warning();
            }
        }
    }

    if !result.is_valid {
        process::exit(3);
    }

    Ok(())
}

async fn handle_generate(
    config: &Config,
    json_output: bool,
    quiet: bool,
    verbose: bool,
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
