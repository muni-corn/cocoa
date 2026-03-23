use std::{
    io,
    path::PathBuf,
    process::{self, Command},
};

use anyhow::Result;
use clap::{Args, ValueEnum};
use rust_i18n::t;

use crate::{
    Config,
    generate::{self, GenerateResult},
    style::{
        goodbye_with_death, goodbye_with_success, print_error, print_error_bold, print_info,
        print_success_bold, print_warning, print_warning_bold,
    },
};

/// The different types of commit sources git can pass as an argument to a
/// `prepare-commit-msg` hook.
#[derive(Clone, Copy, ValueEnum, Debug)]
pub enum CommitMessageSource {
    Message,
    Template,
    Merge,
    Squash,
    Commit,
}

/// Arguments for the `cocoa generate` subcommand.
#[derive(Args, Debug)]
pub struct GenerateArgs {
    /// The arguments that `git` will pass to cocoa if invoked via the
    /// `prepare-commit-msg` hook.
    ///
    /// The first is a path to a file containing a commit message.
    ///
    /// When provided, cocoa prepends the AI-generated commit message to the
    /// file without any interactive prompts. This is intended for use from
    /// a `prepare-commit-msg` git hook installed by `cocoa hook generate`.
    ///
    /// On any failure (no AI configured, no staged changes, API error), cocoa
    /// prepends a comment to the file explaining the issue and exits with 0 so
    /// the commit is never blocked.
    #[arg()]
    pub hook_args: Vec<String>,
}

pub async fn handle_generate(
    config: &Config,
    args: GenerateArgs,
    json_output: bool,
    quiet: bool,
    verbose: bool,
    _dry_run: bool,
) -> Result<()> {
    let mut iter = args.hook_args.iter();
    let (message_file, message_source) = (iter.next(), iter.next());

    // check if the `source` argument can be parsed, and, if so, if it's a source we
    // don't support and will exit immediately for
    let source_warrants_abort = message_source.is_some_and(|s| !s.is_empty() && s != "template");

    eprintln!("debug: message_file {message_file:?}");
    eprintln!("debug: message_source {message_source:?}");
    eprintln!("debug: aborting? {source_warrants_abort}");

    // exit now, silently
    if source_warrants_abort {
        return Ok(());
    }

    // non-interactive git hook mode: write message to file, never block commit
    if let Some(ref file_path) = message_file.map(PathBuf::from) {
        return handle_generate_hook(config, file_path).await;
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
        Ok(GenerateResult {
            message,
            sensitive_warnings,
        }) => {
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
                    if Command::new("git")
                        .args(["commit", "-m", &message])
                        .spawn()?
                        .wait()?
                        .success()
                    {
                        print_success_bold(t!("main.generate.commit_success"));
                        goodbye_with_success();
                    } else {
                        print_error_bold(t!("main.generate.commit_failed"));
                        goodbye_with_death(5);
                    }
                } else {
                    print_info(t!("main.generate.cancelled"));
                    goodbye_with_success();
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
