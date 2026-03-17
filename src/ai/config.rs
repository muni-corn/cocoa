//! AI configuration parsing and management.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Provider, ProviderError};

const DEFAULT_TEMPERATURE: f32 = 0.7;
const DEFAULT_MAX_TOKENS: u32 = 500;

/// AI configuration section from .cocoa.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Optional, because providers can be inferred from `model`
    #[serde(default)]
    pub provider: Option<Provider>,

    pub model: String,

    #[serde(default = "default_temperature")]
    pub temperature: f32,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    pub secret: SecretConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: None,
            model: String::new(),
            temperature: DEFAULT_TEMPERATURE,
            max_tokens: DEFAULT_MAX_TOKENS,
            secret: SecretConfig::default(),
        }
    }
}

/// secret configuration - either from env var or file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SecretConfig {
    Env { env: String },
    File { file: PathBuf },
}

impl Default for SecretConfig {
    fn default() -> Self {
        Self::Env {
            env: "COCOA_API_KEY".to_string(),
        }
    }
}

impl SecretConfig {
    /// Resolves the API key from the configured source.
    ///
    /// # Security
    ///
    /// Error messages are deliberately limited to metadata (the env-var name or
    /// the file path plus the OS error), never the key value itself. Callers
    /// must not log or display the returned `String` in any error path.
    pub fn resolve_api_key(&self) -> Result<String, ProviderError> {
        match self {
            SecretConfig::Env { env } => {
                // include only the variable name, not the value, in the error
                std::env::var(env).map_err(|_| ProviderError::ApiKeyNotFound(env.clone()))
            }
            SecretConfig::File { file } => {
                // the error path runs only when reading fails, so it can never
                // expose the file contents (i.e. the key value)
                std::fs::read_to_string(file)
                    .map(|s| s.trim().to_string())
                    .map_err(|e| {
                        ProviderError::ApiKeyNotFound(format!("file {}: {}", file.display(), e))
                    })
            }
        }
    }
}

fn default_temperature() -> f32 {
    DEFAULT_TEMPERATURE
}

fn default_max_tokens() -> u32 {
    DEFAULT_MAX_TOKENS
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    // serialise tests that mutate env vars to prevent parallel-test race conditions
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_ai_config_default() {
        let config = AiConfig::default();
        assert_eq!(config.provider, None);
        assert_eq!(config.model, "");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 500);
    }

    #[test]
    fn test_secret_config_env_resolve() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key-123");
        }

        let config = SecretConfig::Env {
            env: "TEST_API_KEY".to_string(),
        };

        let result = config.resolve_api_key();

        unsafe {
            std::env::remove_var("TEST_API_KEY");
        }

        assert_eq!(result.unwrap(), "test-key-123");
    }

    #[test]
    fn test_secret_config_env_not_found() {
        let config = SecretConfig::Env {
            env: "NONEXISTENT_KEY".to_string(),
        };

        assert!(config.resolve_api_key().is_err());
    }
}
