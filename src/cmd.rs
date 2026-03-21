use clap::Subcommand;

use crate::cmd::{
    bump::BumpArgs, changelog::ChangelogArgs, lint::LintArgs, migrate::MigrateArgs,
    release::ReleaseArgs, tag::TagArgs,
};

pub mod bump;
pub mod changelog;
pub mod commit;
pub mod generate;
pub mod hook;
pub mod init;
pub mod lint;
pub mod man;
pub mod migrate;
pub mod release;
pub mod tag;
pub mod unhook;

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
    Lint(LintArgs),

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
    Changelog(ChangelogArgs),

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
    Bump(BumpArgs),

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
    Tag(TagArgs),

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
    Migrate(MigrateArgs),

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
    Release(ReleaseArgs),

    #[command(about = "Generate man pages for cocoa")]
    Man,
}
