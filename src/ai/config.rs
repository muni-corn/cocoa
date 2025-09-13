//! ai configuration parsing and management

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Provider, ProviderError};

const DEFAULT_TEMPERATURE: f32 = 0.7;
const DEFAULT_MAX_TOKENS: u32 = 500;

/// ai configuration section from .cocoa.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: Provider,
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
            provider: Provider::OpenAi,
            model: String::new(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
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
    /// resolve the api key from environment variable or file
    pub fn resolve_api_key(&self) -> Result<String, ProviderError> {
        match self {
            SecretConfig::Env { env } => {
                std::env::var(env).map_err(|_| ProviderError::ApiKeyNotFound(env.clone()))
            }
            SecretConfig::File { file } => std::fs::read_to_string(file)
                .map(|s| s.trim().to_string())
                .map_err(|e| {
                    ProviderError::ApiKeyNotFound(format!("file {}: {}", file.display(), e))
                }),
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
    use super::*;

    #[test]
    fn test_ai_config_default() {
        let config = AiConfig::default();
        assert_eq!(config.provider, Provider::OpenAi);
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 500);
    }

    #[test]
    fn test_secret_config_env_resolve() {
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key-123");
        }

        let config = SecretConfig::Env {
            env: "TEST_API_KEY".to_string(),
        };

        assert_eq!(config.resolve_api_key().unwrap(), "test-key-123");

        unsafe {
            std::env::remove_var("TEST_API_KEY");
        }
    }

    #[test]
    fn test_secret_config_env_not_found() {
        let config = SecretConfig::Env {
            env: "NONEXISTENT_KEY".to_string(),
        };

        assert!(config.resolve_api_key().is_err());
    }
}
