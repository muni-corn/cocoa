use std::{io, process};

use anyhow::Result;
use cocoa::{Config, generate};
use rust_i18n::t;

use crate::style::{
    goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error, print_error_bold,
    print_info, print_success_bold, print_warning, print_warning_bold,
};

pub async fn handle_generate(
    config: &Config,
    json_output: bool,
    quiet: bool,
    verbose: bool,
    _dry_run: bool,
) -> Result<()> {
    // check if AI is configured
    if config.ai.is_none() {
        print_error_bold(t!("main.generate.no_ai"));
        print_info(t!("main.generate.no_ai_hint"));
        print_info(t!("main.generate.no_ai_docs"));
        goodbye_with_death(2);
    }

    if verbose {
        print_info(t!("main.generate.calling_ai"));
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
