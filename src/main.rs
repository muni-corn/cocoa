mod cli;
mod commit;
mod config;
mod lint;
mod style;

use std::{
    io::{self, Read},
    process,
};

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use lint::Linter;

use crate::style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error, print_error_bold,
    print_info, print_success_bold, print_warning, print_warning_bold, welcome,
};

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
            print_error_bold("init is not implemented yet");
            // TODO: Implement init command
        }
        Commands::Commit => {
            welcome("cocoa");
            print_error_bold("interactive commit creation not yet implemented");
        }
        Commands::Generate => {
            welcome("cocoa");
            print_error_bold("commit generation not yet implemented");
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
