//! cocoa - conventional commits made easy

pub mod ai;
pub mod commit;
pub mod config;
pub mod generate;
pub mod git_ops;
pub mod lint;
pub(crate) mod style;

// re-export commonly used types
pub use config::Config;
pub use lint::{LintResult, Linter};
