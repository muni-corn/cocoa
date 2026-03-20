use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

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

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a cocoa configuration file interactively.
    ///
    /// Prompts you for commit type preferences, scope rules, line-length
    /// thresholds, AI provider settings, and changelog configuration, then
    /// writes a .cocoa.toml file to the current directory.
    ///
    /// Use --dry-run to preview the generated file without writing it.
    #[command(
        about = "Initialize cocoa config interactively",
        after_help = "examples:
    cocoa init               # interactive setup, writes .cocoa.toml
    cocoa --dry-run init     # preview generated config without writing"
    )]
    Init,

    /// Create a conventional commit interactively.
    ///
    /// Guides you through selecting a commit type, entering an optional
    /// scope, writing a subject line with live character counting,
    /// optionally adding a body, annotating breaking changes, and linking
    /// issue references. The assembled message is validated with the linter
    /// before the commit is created.
    ///
    /// Use --dry-run to print the assembled message without committing.
    #[command(
        about = "Create commits interactively",
        after_help = "examples:
    cocoa commit             # interactive commit wizard
    cocoa --dry-run commit   # preview message without committing"
    )]
    Commit,

    /// Generate a commit message from staged changes using AI.
    ///
    /// Reads the staged diff, sends it to the configured AI provider, and
    /// proposes a conventional commit message. You are prompted to accept or
    /// discard the suggestion before any commit is made.
    ///
    /// Requires an [ai] section in .cocoa.toml with a provider and API key.
    /// Use `cocoa init` to configure AI interactively.
    #[command(
        about = "Generate commit messages with AI",
        after_help = "examples:
    git add -p && cocoa generate   # stage hunks, then generate message
    cocoa --json generate          # emit proposed message as JSON"
    )]
    Generate,

    /// Lint one or more commit messages against conventional commit rules.
    ///
    /// INPUT can be:
    ///   - a raw commit message string (e.g. "feat: add login")
    ///   - a file path containing a commit message (e.g. .git/COMMIT_EDITMSG)
    ///   - a git range (e.g. HEAD~5..HEAD) to lint multiple commits at once
    ///
    /// Read from stdin with --stdin (used by the commit-msg git hook).
    ///
    /// Exit codes: 0 = all valid, 3 = one or more violations found.
    #[command(
        about = "Lint commit messages",
        after_help = r#"examples:
    cocoa lint "feat: add login"          # lint a message string
    cocoa lint HEAD~5..HEAD               # lint last 5 commits
    cocoa lint .git/COMMIT_EDITMSG        # lint a message file
    echo "fix: typo" | cocoa lint --stdin # lint from stdin (git hook)
    cocoa --json lint HEAD~3..HEAD        # machine-readable range output"#
    )]
    Lint {
        /// Commit message, file path, or git range to lint.
        ///
        /// Omit to read from stdin (requires --stdin).
        #[arg(
            value_name = "INPUT",
            help = "Commit message, file path, or git range (e.g. HEAD~5..HEAD)"
        )]
        input: Option<String>,

        /// Read the commit message from standard input.
        ///
        /// Intended for use as a commit-msg git hook. Install the hook
        /// automatically with `cocoa hook`.
        #[arg(long, help = "Read commit message from stdin")]
        stdin: bool,
    },

    /// Generate a changelog from conventional commit history.
    ///
    /// Parses commits in the given range (or all commits if omitted),
    /// groups them by type and version, and renders the result in the
    /// requested format. Breaking changes are always listed prominently.
    ///
    /// Output is written to the file configured in [changelog] (default:
    /// CHANGELOG.md). Use --dry-run to print to stdout without writing.
    ///
    /// Supported formats: markdown (default), json, html, rst, asciidoc,
    /// and template:<path> for a custom Jinja2-style template file.
    #[command(
        about = "Generate a changelog from commit history",
        after_help = r#"examples:
    cocoa changelog                                       # full history -> CHANGELOG.md
    cocoa changelog v1.0.0..HEAD                          # since a specific tag
    cocoa changelog --format json                         # emit as JSON
    cocoa changelog --output CHANGES.md                   # write to a custom file
    cocoa --dry-run changelog v1.2.0..HEAD                # preview without writing
    cocoa changelog --format template:tmpl/changelog.md   # custom template"#
    )]
    Changelog {
        /// Git range to include in the changelog.
        ///
        /// Uses the format FROM..TO (e.g. v1.0.0..HEAD). Omit to include
        /// the full commit history reachable from HEAD.
        #[arg(
            value_name = "RANGE",
            help = "Git range (e.g. v1.0.0..HEAD); defaults to full history"
        )]
        range: Option<String>,

        /// Output format.
        ///
        /// One of: markdown, json, html, rst, asciidoc, or
        /// template:<path> to use a custom Jinja2-style template.
        #[arg(
            long,
            value_name = "FORMAT",
            help = "Output format: markdown (default), json, html, rst, asciidoc, template:<path>"
        )]
        format: Option<String>,

        /// Output file path.
        ///
        /// Overrides the path set in [changelog] config (default:
        /// CHANGELOG.md).
        #[arg(
            long,
            value_name = "PATH",
            help = "Output file path (overrides config)"
        )]
        output: Option<String>,
    },

    /// Bump the project version based on conventional commits.
    ///
    /// Reads the current version from git tags, determines the appropriate
    /// bump type from commit history (or uses the explicit BUMP_TYPE
    /// argument), and updates all version strings in files configured under
    /// [version] commit_version_files.
    ///
    /// Use --dry-run to see the proposed new version and affected files
    /// without making any changes.
    #[command(
        about = "Bump the project version",
        after_help = "examples:
    cocoa bump              # auto-detect bump type from commits
    cocoa bump minor        # force a minor bump
    cocoa bump major        # force a major bump
    cocoa --dry-run bump    # preview new version without writing"
    )]
    Bump {
        /// Bump type to apply.
        ///
        /// One of: major, minor, patch, or auto (default). When auto is
        /// used (or the argument is omitted), the bump type is inferred
        /// from conventional commits since the last version tag: a breaking
        /// change triggers major, feat triggers minor, and fix triggers
        /// patch.
        #[arg(
            value_name = "BUMP_TYPE",
            help = "Bump type: major, minor, patch, or auto (default: auto)"
        )]
        bump_type: Option<String>,
    },

    /// Install the cocoa commit-msg git hook.
    ///
    /// Writes a shell script to .git/hooks/commit-msg that pipes the
    /// commit message through `cocoa lint --stdin`. If a non-cocoa hook
    /// already exists it is backed up before being replaced.
    ///
    /// The hook prevents commits with invalid messages from being created.
    /// Use `cocoa unhook` to remove it.
    #[command(
        about = "Install the commit-msg git hook",
        after_help = "examples:
    cocoa hook               # install the hook
    cocoa --dry-run hook     # show what would be written without installing"
    )]
    Hook,

    /// Remove the cocoa commit-msg git hook.
    ///
    /// Deletes the hook installed by `cocoa hook`. If a backup of a
    /// previous hook exists, it is restored automatically.
    #[command(
        about = "Remove the commit-msg git hook",
        after_help = "examples:
    cocoa unhook             # remove the hook
    cocoa --dry-run unhook   # show what would be removed without acting"
    )]
    Unhook,

    /// Create an annotated git tag for a version.
    ///
    /// Resolves the target version (from VERSION or by auto-detecting the
    /// appropriate bump from commits since the last tag), verifies the tag
    /// does not already exist, generates the changelog as the tag annotation
    /// message, and creates the annotated tag. GPG signing is applied when
    /// sign_tags = true in [version] config.
    ///
    /// Use --dry-run to print the tag name and message without creating it.
    #[command(
        about = "Create an annotated version tag",
        after_help = "examples:
    cocoa tag                # auto-detect version and tag
    cocoa tag 2.1.0          # tag a specific version
    cocoa tag v2.1.0         # v-prefix is stripped automatically
    cocoa --dry-run tag      # preview tag name and message"
    )]
    Tag {
        /// Version to tag.
        ///
        /// Accepts plain semver (2.1.0) or with a v-prefix (v2.1.0). When
        /// omitted, the version is auto-detected by analyzing conventional
        /// commits since the last tag.
        #[arg(
            value_name = "VERSION",
            help = "Version to tag (e.g. 1.2.3 or v1.2.3); auto-detected if omitted"
        )]
        version: Option<String>,
    },

    /// Migrate another tool's configuration to `.cocoa.toml`.
    ///
    /// Reads the configuration file produced by a supported tool (commitlint,
    /// conventional-changelog, or semantic-release), converts it to a
    /// `.cocoa.toml` file, and writes it to the current directory.
    ///
    /// Any existing `.cocoa.toml` is backed up to `.cocoa.toml.bak` before
    /// being replaced. Run with `--undo` to restore the backup.
    ///
    /// Use --dry-run to preview the converted config without writing anything.
    #[command(
        about = "Migrate another tool's config to .cocoa.toml",
        after_help = "examples:
    cocoa migrate                              # auto-detect source tool
    cocoa migrate --from commitlint            # migrate from commitlint
    cocoa migrate --from semantic-release      # migrate from semantic-release
    cocoa --dry-run migrate                    # preview without writing
    cocoa migrate --undo                       # restore previous .cocoa.toml"
    )]
    Migrate {
        /// Source tool to migrate from.
        ///
        /// One of: commitlint, conventional-changelog, semantic-release.
        /// When omitted, the source is auto-detected by looking for known
        /// configuration files in the current directory.
        #[arg(
            long,
            value_enum,
            value_name = "TOOL",
            help = "Source tool to migrate from (auto-detected if omitted)"
        )]
        from: Option<MigrateSourceArg>,

        /// Restore the previous `.cocoa.toml` from the backup.
        ///
        /// Renames `.cocoa.toml.bak` back to `.cocoa.toml`. Use this to
        /// undo a migration.
        #[arg(long, help = "Undo migration by restoring .cocoa.toml.bak")]
        undo: bool,
    },

    /// Run the full release workflow.
    ///
    /// Orchestrates the complete release process in order:
    ///   1. Detect or apply the version bump
    ///   2. Update version strings in configured files
    ///   3. Generate and write the changelog (unless --skip-changelog)
    ///   4. Stage changed files and create a version commit (unless
    ///      --skip-commit)
    ///   5. Create an annotated git tag (unless --skip-tag)
    ///
    /// Individual steps can be skipped with the corresponding flags.
    /// Use --dry-run to preview the full plan without making any changes.
    #[command(
        about = "Run the full release workflow (bump + changelog + commit + tag)",
        after_help = "examples:
    cocoa release                           # full auto release
    cocoa release minor                     # force a minor release
    cocoa --dry-run release                 # preview without changes
    cocoa release --skip-commit --skip-tag  # update files and changelog only
    cocoa release --skip-changelog          # skip changelog generation"
    )]
    Release {
        /// Bump type to apply.
        ///
        /// One of: major, minor, patch, or auto (default). Auto infers the
        /// bump type from conventional commits since the last version tag.
        #[arg(
            value_name = "BUMP_TYPE",
            help = "Bump type: major, minor, patch, or auto (default: auto)"
        )]
        bump_type: Option<String>,

        /// Skip changelog generation and writing.
        #[arg(long, help = "Skip changelog generation and writing")]
        skip_changelog: bool,

        /// Skip staging files and creating the version commit.
        #[arg(long, help = "Skip staging files and creating the version commit")]
        skip_commit: bool,

        /// Skip tag creation.
        #[arg(long, help = "Skip tag creation")]
        skip_tag: bool,
    },
}

/// The third-party tool to migrate from, as supplied on the command line.
#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum MigrateSourceArg {
    /// Migrate from a commitlint configuration file.
    Commitlint,
    /// Migrate from a conventional-changelog configuration file.
    #[value(name = "conventional-changelog")]
    ConventionalChangelog,
    /// Migrate from a semantic-release configuration file.
    #[value(name = "semantic-release")]
    SemanticRelease,
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
