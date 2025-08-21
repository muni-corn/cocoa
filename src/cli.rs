use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cocoa")]
#[command(about = "the conventional commit assistant")]
#[command(version, author)]
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
