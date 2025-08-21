mod cli;
mod commit;
mod config;
mod lint;
mod style;

use std::io::{self, Read};

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use lint::Linter;

use crate::style::{print_error, print_info, print_success, print_warning, welcome};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.config.as_deref().unwrap_or(".cocoa.toml");
    let config = Config::load_or_default(config_path);

    match cli.command {
        Commands::Lint { input, stdin } => {
            welcome("hi! checking this commit message...");
            handle_lint(&config, input, stdin, cli.json, cli.quiet)?;
        }
        Commands::Init => {
            welcome("cocoa");
            print_error("init is not implemented yet");
            // TODO: Implement init command
        }
        Commands::Commit => {
            welcome("cocoa");
            print_error("interactive commit creation not yet implemented");
        }
        Commands::Generate => {
            welcome("cocoa");
            print_error("commit generation not yet implemented");
        }
        Commands::Changelog { range: _ } => {
            welcome("cocoa");
            print_error("changelog generation not yet implemented");
        }
        Commands::Bump { bump_type: _ } => {
            welcome("cocoa");
            print_error("version bumping not yet implemented");
        }
        Commands::Tag => {
            welcome("cocoa");
            print_error("git tagging not yet implemented");
        }
        Commands::Release => {
            welcome("cocoa");
            print_error("release management not yet implemented");
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
) -> Result<()> {
    let linter = Linter::new(config);

    let message = if stdin {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    } else if let Some(input) = input {
        if input.contains("..") {
            // TODO: handle git range
            print_error("git range linting not yet implemented");
            std::process::exit(1);
        } else {
            input
        }
    } else {
        // read from git commit message if available
        print_error("please provide a commit message via --stdin or as an argument");
        std::process::exit(1);
    };

    let result = linter.lint(&message)?;

    if json_output {
        println!("{}", serde_json::to_string(&result)?);
    } else if !quiet {
        if result.violations.is_empty() {
            print_success("commit message is valid");
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
                print_error("commit message has validation errors:");
            } else if warning_count > 0 {
                print_warning("commit message is valid but has warnings:");
            }

            for violation in &result.violations {
                print_info(format!("[{}] {}", violation.rule, violation.message));
            }
        }
    }

    if !result.is_valid {
        std::process::exit(3); // Validation error exit code as per spec
    }

    Ok(())
}
