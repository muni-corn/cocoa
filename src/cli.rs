use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cocoa")]
#[command(about = "A conventional commit assistant")]
#[command(version, author)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, help = "Path to config file")]
    pub config: Option<String>,

    #[arg(long, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(long, help = "Suppress non-error output")]
    pub quiet: bool,

    #[arg(long, help = "Disable colored output")]
    pub no_color: bool,

    #[arg(long, help = "Output in JSON format")]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Initialize configuration")]
    Init,

    #[command(about = "Interactive commit creation")]
    Commit,

    #[command(about = "Generate commit from staged changes")]
    Generate,

    #[command(about = "Lint commit messages")]
    Lint {
        #[arg(help = "Commit message to lint, or git range (e.g., HEAD~5..HEAD)")]
        input: Option<String>,

        #[arg(long, help = "Read commit message from stdin")]
        stdin: bool,
    },

    #[command(about = "Generate changelog")]
    Changelog {
        #[arg(help = "Git range for changelog (e.g., v1.0.0..HEAD)")]
        range: Option<String>,
    },

    #[command(about = "Bump version")]
    Bump {
        #[arg(help = "Bump type: major, minor, patch, or auto")]
        bump_type: Option<String>,
    },

    #[command(about = "Create version tag")]
    Tag,

    #[command(about = "Full release (version + tag + changelog)")]
    Release,
}
