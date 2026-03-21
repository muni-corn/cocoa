use std::{
    io::{self, Read},
    process,
};

use anyhow::Result;
use clap::Args;
use rust_i18n::t;

use crate::{
    Config,
    git_ops::{Git2Ops, GitOperations},
    lint,
    lint::Linter,
    style::{
        goodbye_with_death, goodbye_with_success, goodbye_with_warning, print_error,
        print_error_bold, print_info, print_success_bold, print_warning, print_warning_bold,
    },
};

#[derive(Args)]
pub struct LintArgs {
    /// Commit message, file path, or git range to lint.
    ///
    /// Omit to read from stdin (requires --stdin).
    #[arg(
        value_name = "INPUT",
        help = "Commit message, file path, or git range (e.g. HEAD~5..HEAD)"
    )]
    pub input: Option<String>,

    /// Read the commit message from standard input.
    ///
    /// Intended for use as a commit-msg git hook. Install the hook
    /// automatically with `cocoa hook`.
    #[arg(long, help = "Read commit message from stdin")]
    pub stdin: bool,
}

pub fn handle_lint(
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
                print_info(t!("main.lint.reading_from_file", path = input_str));
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
        print_error_bold(t!("main.lint.no_input"));
        print_info(t!("main.lint.no_input_hint"));
        print_info(t!("main.lint.stdin_hint"));
        goodbye_with_death(1);
    };

    if verbose {
        print_info(t!("main.lint.linting_message", len = message.len()));
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
            print_success_bold(t!("main.lint.message_valid"));
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
                print_error_bold(t!("main.lint.message_has_errors"));
            } else if warning_count > 0 {
                print_warning_bold(t!("main.lint.message_has_warnings"));
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
                    print_info(t!("main.lint.dry_run_errors"));
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
    let (from, to) = if let Some((f, tgt)) = range.split_once("..") {
        (f, tgt)
    } else {
        print_error_bold(t!("main.lint.invalid_range", range = range));
        goodbye_with_death(1);
    };

    if verbose {
        print_info(t!("main.lint.linting_range", from = from, to = to));
    }

    let git_ops = match Git2Ops::open() {
        Ok(ops) => ops,
        Err(e) => {
            print_error_bold(t!("main.git.open_failed", error = e.to_string()));
            goodbye_with_death(5);
        }
    };

    let commits = match git_ops.get_commits_in_range(from, to) {
        Ok(c) => c,
        Err(e) => {
            print_error_bold(t!(
                "main.lint.range_commits_failed",
                range = range,
                error = e.to_string()
            ));
            goodbye_with_death(5);
        }
    };

    if commits.is_empty() {
        if !quiet {
            print_warning_bold(t!("main.lint.no_commits"));
        }
        goodbye_with_warning();
        return Ok(());
    }

    if verbose {
        print_info(t!("main.lint.found_commits", count = commits.len()));
    }

    // lint each commit's subject line
    let lint_results: Vec<_> = commits
        .into_iter()
        .map(|commit| {
            let result = linter.lint(&commit.summary);
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
                    "message": commit.summary,
                    "is_valid": result.is_valid,
                    "violations": result.violations,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&json_results)?);
    } else if !quiet {
        for (commit, result) in &lint_results {
            let short_id = commit.id.get(..8).unwrap_or(&commit.id);
            let commit_label = format!("[{}] {}", short_id, commit.summary);

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
            print_error_bold(t!(
                "main.lint.some_failed",
                invalid = invalid_count,
                total = lint_results.len()
            ));
            if dry_run {
                print_info(t!("main.lint.dry_run_errors"));
                goodbye_with_warning();
            } else {
                goodbye_with_death(3);
            }
        } else {
            print_success_bold(t!("main.lint.all_passed", count = lint_results.len()));
            goodbye_with_success();
        }
    }

    if invalid_count > 0 && !dry_run {
        process::exit(3);
    }

    Ok(())
}
