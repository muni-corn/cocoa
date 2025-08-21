mod cli;
mod commit;
mod config;
mod lint;

use std::io::{self, Read};

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use console::style;
use lint::Linter;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.config.as_deref().unwrap_or(".cocoa.toml");
    let config = Config::load_or_default(config_path);

    match cli.command {
        Commands::Lint { input, stdin } => {
            handle_lint(&config, input, stdin, cli.json, cli.quiet)?;
        }
        Commands::Init => {
            unimplemented!("init is not implemented yet");
            // TODO: Implement init command
        }
        Commands::Commit => {
            println!("interactive commit creation not yet implemented");
        }
        Commands::Generate => {
            println!("commit generation not yet implemented");
        }
        Commands::Changelog { range: _ } => {
            println!("changelog generation not yet implemented");
        }
        Commands::Bump { bump_type: _ } => {
            println!("version bumping not yet implemented");
        }
        Commands::Tag => {
            println!("git tagging not yet implemented");
        }
        Commands::Release => {
            println!("release management not yet implemented");
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
            eprintln!("git range linting not yet implemented");
            std::process::exit(1);
        } else {
            input
        }
    } else {
        // read from git commit message if available
        eprintln!("please provide a commit message via --stdin or as an argument");
        std::process::exit(1);
    };

    let result = linter.lint(&message)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else if !quiet {
        if result.violations.is_empty() {
            println!("{} commit message is valid", style("^u^").green().bold());
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
                println!(
                    "{} commit message has validation errors:",
                    style("×").red().bold()
                );
            } else if warning_count > 0 {
                println!(
                    "{} commit message is valid but has warnings:",
                    style("◆").yellow().bold()
                );
            }

            for violation in &result.violations {
                let severity_icon = match violation.severity {
                    lint::Severity::Error => style("×").red().bold(),
                    lint::Severity::Warning => style("◆").yellow().bold(),
                    lint::Severity::Info => style("ℹ").blue(),
                };

                println!(
                    "  {} [{}] {}",
                    severity_icon, violation.rule, violation.message
                );
            }
        }
    }

    if !result.is_valid {
        std::process::exit(3); // Validation error exit code as per spec
    }

    Ok(())
}
