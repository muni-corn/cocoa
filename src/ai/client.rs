//! Generic AI client wrapper for commit message generation.

use std::fmt::Debug;

use genai::{
    Client as GenaiClient, ModelName,
    chat::{ChatMessage, ChatOptions, ChatRequest},
};

use super::{AiConfig, ProviderError};
use crate::security;

/// Generic AI client for generating commit messages.
pub struct Client {
    config: AiConfig,
    client: GenaiClient,
}

impl Client {
    /// Creates a new AI client with the given configuration.
    pub fn new(config: AiConfig) -> Result<Self, ProviderError> {
        // validate and get api key
        let api_key = config.secret.resolve_api_key()?;

        // create genai client
        let mut client_builder = GenaiClient::builder()
            .with_auth_resolver_fn(|_| Ok(Some(genai::resolver::AuthData::from_single(api_key))));

        if let Some(provider) = config.provider {
            let model_name = ModelName::from(&config.model);
            let adapter_kind = provider.0;
            client_builder = client_builder.with_model_mapper_fn(move |_| {
                Ok(genai::ModelIden {
                    adapter_kind,
                    model_name,
                })
            })
        }

        let client = client_builder.build();

        Ok(Self { config, client })
    }

    /// Generates a commit message from the given context.
    pub async fn generate_commit_message(
        &self,
        staged_changes: &str,
        context: &CommitContext,
    ) -> Result<String, ProviderError> {
        let model_name = &self.config.model;
        let prompt = self.build_prompt(staged_changes, context);

        let messages = vec![
            ChatMessage::system(
                "you are an expert software engineer who writes excellent conventional commit messages. respond with only the commit message and body, no explanation or additional text.",
            ),
            ChatMessage::user(prompt),
        ];

        let chat_request = ChatRequest::new(messages);
        let chat_options = ChatOptions::default()
            .with_temperature(self.config.temperature as f64)
            .with_max_tokens(self.config.max_tokens);

        let response = self
            .client
            .exec_chat(model_name, chat_request, Some(&chat_options))
            .await
            // redact the error string before surfacing it; the underlying HTTP
            // response body from the provider could in rare cases echo back
            // authentication details or other sensitive content
            .map_err(|e| ProviderError::Api(security::redact(&format!("genai error: {e}"))))?;

        let content = response.texts().join("");

        Ok(content.trim().to_string())
    }

    /// Builds the prompt for commit generation.
    fn build_prompt(&self, staged_changes: &str, context: &CommitContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("generate a conventional commit message for these staged changes:\n\n");
        prompt.push_str("```diff\n");
        prompt.push_str(staged_changes);
        prompt.push_str("\n```\n\n");

        if let Some(branch) = &context.branch_name {
            prompt.push_str(&format!("branch name: {}\n\n", branch));
        }

        if !context.recent_commits.is_empty() {
            prompt.push_str("recent commit messages for context:\n");
            for commit in context.recent_commits.iter().take(5) {
                prompt.push_str(&format!("- {}\n", commit));
            }
            prompt.push('\n');
        }

        prompt.push_str("requirements:\n");
        prompt.push_str("- use conventional commits format (type: description)\n");
        prompt.push_str("- type must be one of: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert\n");
        prompt.push_str("- keep subject line under 50 characters if possible\n");
        prompt.push_str("- use lowercase for the description\n");
        prompt.push_str("- be concise and descriptive\n");
        prompt.push_str("- respond with only the commit message, nothing else\n");

        prompt
    }
}

/// Context information for commit generation.
#[derive(Debug, Clone, Default)]
pub struct CommitContext {
    pub branch_name: Option<String>,
    pub recent_commits: Vec<String>,
    pub repository_name: Option<String>,
    pub is_merge: bool,
    pub is_rebase: bool,
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use genai::adapter::AdapterKind;

    use super::*;
    use crate::ai::{Provider, config::SecretConfig};

    // serialise tests that mutate env vars to prevent parallel-test race conditions
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_config() -> AiConfig {
        // caller is responsible for holding ENV_LOCK while calling this
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        AiConfig {
            provider: Some(Provider(AdapterKind::OpenAI)),
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
        let _guard = ENV_LOCK.lock().unwrap();
        let config = test_config();
        let result = Client::new(config);

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert!(result.is_ok());
    }

    #[test]
    fn test_client_new_invalid_key() {
        let _guard = ENV_LOCK.lock().unwrap();
        let config = AiConfig {
            secret: SecretConfig::Env {
                env: "NONEXISTENT_KEY".to_string(),
            },
            ..test_config()
        };

        let result = Client::new(config);

        unsafe {
            std::env::remove_var("TEST_API_KEY");
        }

        assert!(result.is_err());
    }

    #[test]
    fn test_get_model_name_openai() {
        let _guard = ENV_LOCK.lock().unwrap();

        let config = AiConfig {
            provider: Some(Provider(AdapterKind::OpenAI)),
            model: "gpt-4".to_string(),
            ..test_config()
        };
        let client = Client::new(config).unwrap();
        assert_eq!(client.config.model, "gpt-4");

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn test_get_model_name_anthropic() {
        let _guard = ENV_LOCK.lock().unwrap();

        let config = AiConfig {
            provider: Some(Provider(AdapterKind::Anthropic)),
            model: "claude-3-sonnet".to_string(),
            ..test_config()
        };
        let client = Client::new(config).unwrap();
        assert_eq!(client.config.model, "claude-3-sonnet");

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
    }

    #[test]
    fn test_build_prompt_with_context() {
        let _guard = ENV_LOCK.lock().unwrap();

        let client = Client::new(test_config()).unwrap();
        let context = CommitContext {
            branch_name: Some("feature/test".to_string()),
            recent_commits: vec!["feat: previous commit".to_string()],
            repository_name: Some("test-repo".to_string()),
            is_merge: false,
            is_rebase: false,
        };

        let prompt = client.build_prompt("test diff", &context);

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert!(prompt.contains("test diff"));
        assert!(prompt.contains("feature/test"));
        assert!(prompt.contains("feat: previous commit"));
        assert!(prompt.contains("conventional commits"));
    }

    #[test]
    fn test_build_prompt_without_branch() {
        let _guard = ENV_LOCK.lock().unwrap();

        let client = Client::new(test_config()).unwrap();
        let context = CommitContext::default();

        let prompt = client.build_prompt("test diff", &context);

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert!(prompt.contains("test diff"));
        assert!(!prompt.contains("branch name:"));
    }

    #[test]
    fn test_commit_context_default() {
        let context = CommitContext::default();

        assert!(context.branch_name.is_none());
        assert!(context.recent_commits.is_empty());
        assert!(context.repository_name.is_none());
        assert!(!context.is_merge);
        assert!(!context.is_rebase);
    }
}
