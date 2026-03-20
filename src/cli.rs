use clap::{CommandFactory, Parser};

use crate::cmd::Command;

const HELP_TEMPLATE: &str = "\
{name} {version}

{about}

usage:
  {usage}

commands:
{subcommands}

options:
{options}
";

const SUBCOMMAND_HELP_TEMPLATE_WITH_ARGS: &str = "\
{about}

usage:
  {usage}

arguments:
{positionals}

options:
{options}
";

const SUBCOMMAND_HELP_TEMPLATE_WITH_ARGS_AND_EXAMPLES: &str = "\
{about}

usage:
  {usage}

arguments:
{positionals}

options:
{options}

{after-help}
";

const SUBCOMMAND_HELP_TEMPLATE_NO_ARGS: &str = "\
{about}

usage:
  {usage}

options:
{options}
";

const SUBCOMMAND_HELP_TEMPLATE_NO_ARGS_AND_EXAMPLES: &str = "\
{about}

usage:
  {usage}

options:
{options}

{after-help}
";

#[derive(Parser)]
#[command(name = "cocoa")]
#[command(about = "the conventional commit assistant")]
#[command(
    long_about = "hi! i'm cocoa, the conventional commit assistant! i can help you write
                  well-formed commit messages, lint existing ones, generate changelogs,
                  and manage semantic versioning, all from a single tool.\n
                  All commands accept --dry-run to preview changes without writing anything,
                  --json for machine-readable output, and --quiet to suppress non-error output."
)]
#[command(version, author)]
#[command(help_template = HELP_TEMPLATE)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Path to the configuration file.
    ///
    /// Overrides automatic discovery. By default cocoa searches for
    /// .cocoa.toml in the current directory, then $XDG_CONFIG_HOME/cocoa/,
    /// ~/.config/cocoa/, and /etc/cocoa/.
    #[arg(long, value_name = "PATH", help = "Path to config file")]
    pub config: Option<String>,

    /// Enable verbose output.
    ///
    /// Prints additional diagnostic information such as the commit message
    /// being linted, the AI prompt being sent, and intermediate git
    /// operations.
    #[arg(long, help = "Enable verbose output")]
    pub verbose: bool,

    /// Suppress non-error output.
    ///
    /// Silences informational and success messages. Errors and warnings are
    /// still printed to stderr. Useful when piping output or running in CI.
    #[arg(long, help = "Suppress non-error output")]
    pub quiet: bool,

    /// Disable colored terminal output.
    ///
    /// Forces plain text output with no ANSI escape codes. Useful when piping
    /// output to a file or running in environments that do not support color.
    #[arg(long, help = "Disable colored output")]
    pub no_color: bool,

    /// Output results as JSON.
    ///
    /// Emits structured JSON instead of human-readable text. The schema
    /// varies per command — see each subcommand's documentation for details.
    #[arg(long, help = "Output in JSON format")]
    pub json: bool,

    /// Show what would be done without making any changes.
    ///
    /// All write operations (file writes, git commits, tag creation, hook
    /// installation) are skipped. Output describes the actions that would
    /// have been taken.
    #[arg(long, help = "Show what would be done without executing")]
    pub dry_run: bool,
}

impl Cli {
    /// Creates a Command with conditional help templates for subcommands.
    ///
    /// Subcommands with positional arguments use a template that includes
    /// an "arguments:" section; those without use a shorter template.
    /// Subcommands that define after_help examples get an extended template
    /// that renders the examples section.
    pub fn command_with_conditional_help() -> clap::Command {
        let mut cmd = Self::command();

        // assign appropriate help template based on positionals and after_help
        cmd = cmd.mut_subcommands(|subcmd| {
            let has_positionals = subcmd.get_positionals().next().is_some();
            let has_after_help = subcmd.get_after_help().is_some();

            let template = match (has_positionals, has_after_help) {
                (true, true) => SUBCOMMAND_HELP_TEMPLATE_WITH_ARGS_AND_EXAMPLES,
                (true, false) => SUBCOMMAND_HELP_TEMPLATE_WITH_ARGS,
                (false, true) => SUBCOMMAND_HELP_TEMPLATE_NO_ARGS_AND_EXAMPLES,
                (false, false) => SUBCOMMAND_HELP_TEMPLATE_NO_ARGS,
            };

            subcmd.help_template(template)
        });

        cmd
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn test_cli_can_parse() {
        // verify cli structure is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_parse_lint_with_stdin() {
        let args = vec!["cocoa", "lint", "--stdin"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Command::Lint { stdin, .. } => assert!(stdin),
            _ => panic!("expected lint command"),
        }
    }

    #[test]
    fn test_parse_lint_with_input() {
        let args = vec!["cocoa", "lint", "feat: test"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Command::Lint { input, .. } => assert_eq!(input, Some("feat: test".to_string())),
            _ => panic!("expected lint command"),
        }
    }

    #[test]
    fn test_parse_generate_command() {
        let args = vec!["cocoa", "generate"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(matches!(cli.command, Command::Generate));
    }

    #[test]
    fn test_parse_with_config_flag() {
        let args = vec!["cocoa", "--config", "custom.toml", "lint", "--stdin"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert_eq!(cli.config, Some("custom.toml".to_string()));
    }

    #[test]
    fn test_parse_with_json_flag() {
        let args = vec!["cocoa", "--json", "lint", "--stdin"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(cli.json);
    }

    #[test]
    fn test_parse_with_quiet_flag() {
        let args = vec!["cocoa", "--quiet", "lint", "--stdin"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(cli.quiet);
    }
}
