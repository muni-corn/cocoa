mod cli;
mod commit;
mod config;
mod lint;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use lint::Linter;
use std::io::{self, Read};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.config.as_deref().unwrap_or(".cocoa.toml");
    let config = Config::load_or_default(config_path);

    match cli.command {
        Commands::Lint { input, stdin } => {
            handle_lint(&config, input, stdin, cli.json, cli.quiet)?;
        }
        Commands::Init => {
            println!("Initializing cocoa configuration...");
            // TODO: Implement init command
        }
        Commands::Commit => {
            println!("Interactive commit creation not yet implemented");
        }
        Commands::Generate => {
            println!("Commit generation not yet implemented");
        }
        Commands::Changelog { range: _ } => {
            println!("Changelog generation not yet implemented");
        }
        Commands::Bump { bump_type: _ } => {
            println!("Version bumping not yet implemented");
        }
        Commands::Tag => {
            println!("Git tagging not yet implemented");
        }
        Commands::Release => {
            println!("Release management not yet implemented");
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
            // TODO: Handle git range
            eprintln!("Git range linting not yet implemented");
            std::process::exit(1);
        } else {
            input
        }
    } else {
        // Read from git commit message if available
        eprintln!("Please provide a commit message via --stdin or as an argument");
        std::process::exit(1);
    };

    let result = linter.lint(&message)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if !quiet {
        if result.is_valid {
            println!("✓ Commit message is valid");
        } else {
            println!("✗ Commit message has validation errors:");
            for violation in &result.violations {
                let severity_icon = match violation.severity {
                    lint::Severity::Error => "❌",
                    lint::Severity::Warning => "⚠️",
                    lint::Severity::Info => "ℹ️",
                };

                if let Some(line) = violation.line {
                    println!(
                        "  {} [{}:{}] {}",
                        severity_icon, violation.rule, line, violation.message
                    );
                } else {
                    println!(
                        "  {} [{}] {}",
                        severity_icon, violation.rule, violation.message
                    );
                }
            }
        }
    }

    if !result.is_valid {
        std::process::exit(3); // Validation error exit code as per spec
    }

    Ok(())
}
