//! generic ai client wrapper

use super::{AiConfig, Provider, ProviderError};

/// generic ai client for generating commit messages
pub struct Client {
    config: AiConfig,
}

impl Client {
    /// create a new ai client with the given configuration
    pub fn new(config: AiConfig) -> Result<Self, ProviderError> {
        // validate configuration
        config.secret.resolve_api_key()?;

        Ok(Self { config })
    }

    /// generate a commit message from the given context
    pub async fn generate_commit_message(
        &self,
        _staged_changes: &str,
        _context: &CommitContext,
    ) -> Result<String, ProviderError> {
        // placeholder implementation
        match self.config.provider {
            Provider::OpenAi => Ok("feat: placeholder commit message".to_string()),
            Provider::Anthropic => Ok("feat: placeholder commit message".to_string()),
            Provider::Ollama => Ok("feat: placeholder commit message".to_string()),
            Provider::OpenRouter => Ok("feat: placeholder commit message".to_string()),
        }
    }
}

/// context information for commit generation
pub struct CommitContext {
    pub branch_name: Option<String>,
    pub recent_commits: Vec<String>,
}

impl Default for CommitContext {
    fn default() -> Self {
        Self {
            branch_name: None,
            recent_commits: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::SecretConfig;

    fn test_config() -> AiConfig {
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        AiConfig {
            provider: Provider::OpenAi,
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 500,
            secret: SecretConfig::Env {
                env: "TEST_API_KEY".to_string(),
            },
        }
    }

    #[test]
    fn test_client_new() {
        let config = test_config();
        let client = Client::new(config);
        assert!(client.is_ok());

        unsafe {
            std::env::remove_var("TEST_API_KEY");
        }
    }

    #[test]
    fn test_client_new_invalid_key() {
        let config = AiConfig {
            secret: SecretConfig::Env {
                env: "NONEXISTENT_KEY".to_string(),
            },
            ..test_config()
        };

        let client = Client::new(config);
        assert!(client.is_err());

        unsafe {
            std::env::remove_var("TEST_API_KEY");
        }
    }
}
