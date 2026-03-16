//! Interactive configuration initialisation for cocoa.
//!
//! Implements the `cocoa init` command, which walks the user through a series
//! of prompts and writes a `.cocoa.toml` file to the current directory.

use std::path::Path;

use thiserror::Error;

use crate::config::Config;

/// Errors that can occur during `cocoa init`.
#[derive(Debug, Error)]
pub enum InitError {
    /// Failed to write the config file to disk.
    #[error("failed to write config file: {0}")]
    Write(#[from] std::io::Error),

    /// TOML serialisation of the constructed config failed.
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),

    /// An interactive prompt returned an error (e.g., terminal I/O failure).
    #[error("interactive prompt failed: {0}")]
    Prompt(String),

    /// The user declined to overwrite an existing config file.
    #[error("init aborted by user")]
    Aborted,

    /// A `.cocoa.toml` already exists and we are running non-interactively.
    #[error(".cocoa.toml already exists; delete it first or run interactively to overwrite")]
    FileExists,
}

/// Returns `true` when the process is attached to an interactive terminal.
///
/// Used to skip interactive prompts in CI or piped contexts.
fn is_interactive() -> bool {
    console::Term::stderr().is_term()
}

/// Runs the `cocoa init` wizard.
///
/// In interactive mode (TTY attached) the user is guided through a series of
/// prompts to configure commit rules and optional AI generation. In
/// non-interactive mode default values are used silently.
///
/// If `dry_run` is `true` the resulting TOML is printed to stdout and no file
/// is written.
pub fn init(dry_run: bool) -> Result<(), InitError> {
    let (commit_config, ai_config) = if is_interactive() {
        let commit = prompts::commit_config()?;
        let ai = prompts::ai_config()?;
        (commit, ai)
    } else {
        (crate::config::CommitConfig::default(), None)
    };

    let config = Config {
        commit: commit_config,
        ai: ai_config,
        changelog: None,
        version: None,
    };

    write_config(&config, dry_run)
}

/// Serialises `config` to TOML and either prints it (dry-run) or writes it to
/// `.cocoa.toml` in the current directory.
fn write_config(config: &Config, dry_run: bool) -> Result<(), InitError> {
    let toml = toml::to_string_pretty(config)?;

    if dry_run {
        println!("{}", toml);
        return Ok(());
    }

    let path = Path::new(".cocoa.toml");

    if path.exists() {
        if is_interactive() {
            use dialoguer::{Confirm, theme::ColorfulTheme};
            use rust_i18n::t;
            let overwrite = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(t!("init.prompt.overwrite").as_ref())
                .default(false)
                .interact()
                .unwrap_or(false);
            if !overwrite {
                return Err(InitError::Aborted);
            }
        } else {
            return Err(InitError::FileExists);
        }
    }

    std::fs::write(path, toml)?;
    Ok(())
}

/// Interactive prompt implementations, kept in a sub-module to keep imports
/// scoped.
pub(crate) mod prompts {
    use std::collections::HashSet;

    use dialoguer::{Confirm, Input, MultiSelect, Select, theme::ColorfulTheme};
    use rust_i18n::t;

    use crate::{
        ai::{AiConfig, Provider, SecretConfig},
        config::{CommitConfig, CommitRules, RuleLevel},
        init::InitError,
    };

    const DEFAULT_COMMIT_TYPES: &[&str] = &[
        "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style",
        "test",
    ];

    const AI_PROVIDERS: &[&str] = &["openai", "anthropic", "ollama", "groq", "gemini"];

    /// Prompts the user for commit configuration settings.
    pub fn commit_config() -> Result<CommitConfig, InitError> {
        let theme = ColorfulTheme::default();

        // --- commit type selection ---
        let all_checked: Vec<bool> = vec![true; DEFAULT_COMMIT_TYPES.len()];
        let selected = MultiSelect::with_theme(&theme)
            .with_prompt(t!("init.prompt.types").as_ref())
            .items(DEFAULT_COMMIT_TYPES)
            .defaults(&all_checked)
            .interact()
            .unwrap_or_else(|_| (0..DEFAULT_COMMIT_TYPES.len()).collect());

        let types: HashSet<String> = selected
            .into_iter()
            .map(|i| DEFAULT_COMMIT_TYPES[i].to_string())
            .collect();

        // --- optional scope restriction ---
        let restrict_scopes = Confirm::with_theme(&theme)
            .with_prompt(t!("init.prompt.restrict_scopes").as_ref())
            .default(false)
            .interact()
            .unwrap_or(false);

        let scopes = if restrict_scopes {
            let raw: String = Input::with_theme(&theme)
                .with_prompt(t!("init.prompt.allowed_scopes").as_ref())
                .interact_text()
                .map_err(|e| InitError::Prompt(e.to_string()))?;

            let set: HashSet<String> = raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            Some(set)
        } else {
            None
        };

        // --- rule thresholds ---
        let warn_subject: usize = Input::with_theme(&theme)
            .with_prompt(t!("init.prompt.warn_subject").as_ref())
            .default(50usize)
            .interact_text()
            .unwrap_or(50);

        let deny_subject: usize = Input::with_theme(&theme)
            .with_prompt(t!("init.prompt.deny_subject").as_ref())
            .default(72usize)
            .interact_text()
            .unwrap_or(72);

        let warn_body: usize = Input::with_theme(&theme)
            .with_prompt(t!("init.prompt.warn_body").as_ref())
            .default(250usize)
            .interact_text()
            .unwrap_or(250);

        let deny_body: usize = Input::with_theme(&theme)
            .with_prompt(t!("init.prompt.deny_body").as_ref())
            .default(500usize)
            .interact_text()
            .unwrap_or(500);

        let warn = RuleLevel {
            subject_length: Some(warn_subject),
            body_length: Some(warn_body),
            no_scope: Some(true),
            no_body: Some(false),
            no_type: Some(true),
            no_breaking_change_footer: Some(true),
            regex_patterns: Some(vec![]),
        };

        let deny = RuleLevel {
            subject_length: Some(deny_subject),
            body_length: Some(deny_body),
            no_scope: Some(false),
            no_body: Some(false),
            no_type: Some(true),
            no_breaking_change_footer: Some(false),
            regex_patterns: Some(vec![]),
        };

        Ok(CommitConfig {
            types,
            scopes,
            rules: CommitRules {
                warn,
                deny,
                ..CommitRules::default()
            },
        })
    }

    /// Prompts the user for optional AI generation configuration.
    ///
    /// Returns `None` if the user declines AI setup.
    pub fn ai_config() -> Result<Option<AiConfig>, InitError> {
        let theme = ColorfulTheme::default();

        let configure_ai = Confirm::with_theme(&theme)
            .with_prompt(t!("init.prompt.configure_ai").as_ref())
            .default(false)
            .interact()
            .unwrap_or(false);

        if !configure_ai {
            return Ok(None);
        }

        let provider_idx = Select::with_theme(&theme)
            .with_prompt(t!("init.prompt.ai_provider").as_ref())
            .items(AI_PROVIDERS)
            .default(0)
            .interact()
            .unwrap_or(0);

        let provider_str = AI_PROVIDERS[provider_idx];
        let provider: Provider = provider_str
            .parse()
            .map_err(|e: crate::ai::ProviderError| InitError::Prompt(e.to_string()))?;

        let model: String = Input::with_theme(&theme)
            .with_prompt(t!("init.prompt.model_name").as_ref())
            .interact_text()
            .map_err(|e| InitError::Prompt(e.to_string()))?;

        let use_env = Confirm::with_theme(&theme)
            .with_prompt(t!("init.prompt.use_env_var").as_ref())
            .default(true)
            .interact()
            .unwrap_or(true);

        let secret = if use_env {
            let env_var: String = Input::with_theme(&theme)
                .with_prompt(t!("init.prompt.env_var_name").as_ref())
                .default(default_env_var(provider_str).to_string())
                .interact_text()
                .map_err(|e| InitError::Prompt(e.to_string()))?;
            SecretConfig::Env { env: env_var }
        } else {
            let file_path: String = Input::with_theme(&theme)
                .with_prompt(t!("init.prompt.api_key_file").as_ref())
                .interact_text()
                .map_err(|e| InitError::Prompt(e.to_string()))?;
            SecretConfig::File {
                file: file_path.into(),
            }
        };

        Ok(Some(AiConfig {
            provider: Some(provider),
            model,
            secret,
            ..AiConfig::default()
        }))
    }

    /// Returns the conventional environment variable name for a given provider.
    fn default_env_var(provider: &str) -> &str {
        match provider {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "groq" => "GROQ_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            _ => "COCOA_API_KEY",
        }
    }
}
