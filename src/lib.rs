//! cocoa: the conventional commit assistant <3

#![allow(dead_code)]

pub mod ai;
pub mod changelog;
pub mod commit;
pub mod config;
pub mod generate;
pub mod git_ops;
pub mod hook;
pub mod i18n;
pub mod init;
pub mod interactive;
pub mod lint;
pub mod release;
pub mod security;
pub(crate) mod style;
pub mod tag;
pub mod version;

// re-export commonly used types
pub use config::Config;
pub use lint::{LintResult, Linter};
