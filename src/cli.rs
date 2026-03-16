use clap::{CommandFactory, Parser, Subcommand};

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

const SUBCOMMAND_HELP_TEMPLATE_NO_ARGS: &str = "\
{about}

usage:
  {usage}

options:
{options}
";

#[derive(Parser)]
#[command(name = "cocoa")]
#[command(about = "the conventional commit assistant")]
#[command(version, author)]
#[command(help_template = HELP_TEMPLATE)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, help = "path to config file")]
    pub config: Option<String>,

    #[arg(long, help = "enable verbose output")]
    pub verbose: bool,

    #[arg(long, help = "suppress non-error output")]
    pub quiet: bool,

    #[arg(long, help = "disable colored output")]
    pub no_color: bool,

    #[arg(long, help = "output in JSON format")]
    pub json: bool,

    #[arg(long, help = "show what would be done without executing")]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "initialize configuration")]
    Init,

    #[command(about = "interactive commit creation")]
    Commit,

    #[command(about = "generate commit from staged changes")]
    Generate,

    #[command(about = "lint commit messages")]
    Lint {
        #[arg(help = "commit message to lint, or git range (e.g., HEAD~5..HEAD)")]
        input: Option<String>,

        #[arg(long, help = "read commit message from stdin")]
        stdin: bool,
    },

    #[command(about = "generate changelog")]
    Changelog {
        #[arg(help = "git range for changelog (e.g., v1.0.0..HEAD)")]
        range: Option<String>,
    },

    #[command(about = "bump version")]
    Bump {
        #[arg(help = "bump type: major, minor, patch, or auto")]
        bump_type: Option<String>,
    },

    #[command(about = "create version tag")]
    Tag,

    #[command(about = "full release (version + changelog + tag)")]
    Release,
}

impl Cli {
    /// Creates a Command with conditional help templates for subcommands.
    pub fn command_with_conditional_help() -> clap::Command {
        let mut cmd = Self::command();

        // iterate through subcommands and set appropriate help template
        cmd = cmd.mut_subcommands(|subcmd| {
            let has_positionals = subcmd.get_positionals().next().is_some();
            let template = if has_positionals {
                SUBCOMMAND_HELP_TEMPLATE_WITH_ARGS
            } else {
                SUBCOMMAND_HELP_TEMPLATE_NO_ARGS
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
            Commands::Lint { stdin, .. } => assert!(stdin),
            _ => panic!("expected lint command"),
        }
    }

    #[test]
    fn test_parse_lint_with_input() {
        let args = vec!["cocoa", "lint", "feat: test"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Lint { input, .. } => assert_eq!(input, Some("feat: test".to_string())),
            _ => panic!("expected lint command"),
        }
    }

    #[test]
    fn test_parse_generate_command() {
        let args = vec!["cocoa", "generate"];
        let cli = Cli::try_parse_from(args).unwrap();

        assert!(matches!(cli.command, Commands::Generate));
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
