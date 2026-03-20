//! cocoa: the conventional commit assistant <3

// initialize the library-side translation backend from the bundled locale
// catalog; this must be at the crate root so `t!` resolves
// `crate::_rust_i18n_t`
rust_i18n::i18n!("locales");

pub mod ai;
pub mod changelog;
pub mod cli;
pub mod cmd;
pub mod commit;
pub mod config;
pub mod generate;
pub mod git_ops;
pub mod hook;
pub mod i18n;
pub mod init;
pub mod interactive;
pub mod lint;
pub mod migrate;
pub mod release;
pub mod security;
pub mod style;
pub mod tag;
pub mod version;

// re-export commonly used types
pub use config::Config;
pub use lint::{LintResult, Linter};
