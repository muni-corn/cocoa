use std::{io, path::PathBuf, process};

use anyhow::Result;
use clap::Args;
use rust_i18n::t;

use crate::{
    Config,
    generate::{self, GenerateResult},
    style::{
        goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error,
        print_error_bold, print_info, print_success_bold, print_warning, print_warning_bold,
    },
};

/// Arguments for the `cocoa generate` subcommand.
#[derive(Args, Debug)]
pub struct GenerateArgs {
    /// Write the generated message directly to FILE (git hook mode).
    ///
    /// When provided, cocoa writes the AI-generated commit message to FILE
    /// without any interactive prompts. This is intended for use from a
    /// `prepare-commit-msg` git hook installed by `cocoa hook generate`.
    ///
    /// On any failure (no AI configured, no staged changes, API error), cocoa
    /// writes a comment to FILE explaining the issue and exits 0 so the
    /// commit is never blocked.
    #[arg(long, value_name = "FILE")]
    pub hook: Option<PathBuf>,
}

pub async fn handle_generate(
    config: &Config,
    args: GenerateArgs,
    json_output: bool,
    quiet: bool,
    verbose: bool,
    _dry_run: bool,
) -> Result<()> {
    // non-interactive git hook mode: write message to file, never block commit
    if let Some(ref hook_file) = args.hook {
        return handle_generate_hook(config, hook_file).await;
    }

    // check if AI is configured
    if config.ai.is_none() {
        print_error_bold(t!("main.generate.no_ai"));
        print_info(t!("main.generate.no_ai_hint"));
        print_info(t!("main.generate.no_ai_docs"));
        goodbye_with_death(2);
    }

    if verbose {
        print_info(t!("main.generate.calling_provider"));
    }

    match generate::generate_commit_message(config).await {
        Ok(result) => {
            // destructure upfront to avoid partial-move issues
            let generate::GenerateResult {
                message,
                sensitive_warnings,
            } = result;

            // surface sensitive-content warnings before showing the message so
            // the user can decide whether to abort before committing
            if !sensitive_warnings.is_empty() && !quiet {
                for warning in &sensitive_warnings {
                    print_warning_bold(warning);
                }
                print_warning(t!("main.generate.secrets_warning"));
            }

            if json_output {
                let json_result = serde_json::json!({
                    "success": true,
                    "message": message,
                    "sensitive_warnings": sensitive_warnings,
                });
                println!("{}", serde_json::to_string_pretty(&json_result)?);
            } else if !quiet {
                print_success_bold(t!("main.generate.success"));
                println!("\n{}\n", message);

                // ask user if they want to commit with this message
                print_info(t!("main.generate.commit_prompt"));

                use std::io::Write;
                print!("❯ ");
                io::stdout().flush()?;

                let mut response = String::new();
                io::stdin().read_line(&mut response)?;

                if response.trim().to_lowercase().starts_with('y') {
                    // commit with the generated message
                    let output = std::process::Command::new("git")
                        .args(["commit", "-m", &message])
                        .output()?;

                    if output.status.success() {
                        print_success_bold(t!("main.generate.commit_success"));
                        goodbye_with_success();
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        print_error_bold(t!(
                            "main.generate.commit_failed",
                            error = stderr.to_string()
                        ));
                        goodbye_with_death(5);
                    }
                } else {
                    print_info(t!("main.generate.cancelled"));
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
                        print_error_bold(t!("main.generate.no_staged"));
                        print_info(t!("main.generate.no_staged_hint"));
                        print_info(t!("main.generate.no_staged_hint2"));
                    }
                    generate::GenerateError::AiGeneration(msg) => {
                        print_error_bold(t!("main.generate.ai_failed"));
                        print_error(msg);
                        print_info(t!("main.generate.check_ai"));
                    }
                    generate::GenerateError::GitContext(msg)
                    | generate::GenerateError::StagedChanges(msg)
                    | generate::GenerateError::GitCommand(msg) => {
                        print_error_bold(t!("main.generate.git_failed"));
                        print_error(msg);
                    }
                    generate::GenerateError::Validation(msg) => {
                        print_error_bold(t!("main.generate.validation_failed"));
                        print_error(msg);
                        print_info(t!("main.generate.validation_hint"));
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

/// Handles `cocoa generate --hook <file>` (non-interactive `prepare-commit-msg`
/// mode).
///
/// Writes the AI-generated message to `hook_file`. On any failure, prepends a
/// friendly comment to the file so the user sees the error in their editor,
/// then exits 0 to avoid blocking the commit.
async fn handle_generate_hook(config: &Config, hook_file: &PathBuf) -> Result<()> {
    // read any content git already placed in the message file (e.g. a template)
    let existing = std::fs::read_to_string(hook_file).unwrap_or_default();

    // check AI is configured; surface the error as a comment in the file
    if config.ai.is_none() {
        let comment = format!(
            "

# hi, cocoa here! ^~^
#
# i couldn't generate a commit message for you because you don't have an ai
# provider configured. add an [ai] section to .cocoa.toml!
#
{existing}"
        );
        let _ = std::fs::write(hook_file, comment);
        return Ok(());
    }

    match generate::generate_commit_message(config).await {
        Ok(GenerateResult { message, .. }) => {
            // write the generated message; best-effort (don't block commit on write error)
            let new_message = format!("{message}\n{existing}");
            let _ = std::fs::write(hook_file, new_message);
        }
        Err(e) => {
            // prepend the error as a comment so the user sees it in their editor
            let comment = format!(
                "

# hi, cocoa here! ^~^
#
# i couldn't generate a commit message for you due to this error:
#
#   {error_msg}
#
{existing}",
                error_msg = e.to_string().replace("\n", "\n#   ")
            );
            let _ = std::fs::write(hook_file, comment);
        }
    }

    Ok(())
}
