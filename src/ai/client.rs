//! Generic AI client wrapper for commit message generation.

use std::fmt::Debug;

use genai::{
    Client as GenaiClient, ModelName,
    chat::{ChatMessage, ChatOptions, ChatRequest},
};

use super::{AiConfig, Conversation, ProviderError};
use crate::{
    ai::prompt::{build_prompt, build_system_prompt},
    config::CommitConfig,
    security,
};

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

        let client = client_builder
            .build()
            .map_err(|e| ProviderError::Api(format!("client build error: {e}")))?;

        Ok(Self { config, client })
    }

    /// Starts a conversation seeded with the prompts for the given changes.
    ///
    /// The returned conversation has not contacted the model yet; call
    /// [`Conversation::send`] to get the first reply.
    pub fn start_conversation(
        &self,
        staged_changes: &str,
        context: &CommitContext,
        config: &CommitConfig,
    ) -> Conversation<'_> {
        let system_prompt = build_system_prompt();
        let prompt = build_prompt(staged_changes, context, config);

        Conversation::new(self, vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(prompt),
        ])
    }

    /// Generates a commit message from the given context.
    pub async fn generate_commit_message(
        &self,
        staged_changes: &str,
        context: &CommitContext,
        config: &CommitConfig,
    ) -> Result<String, ProviderError> {
        self.start_conversation(staged_changes, context, config)
            .send()
            .await
    }

    /// Sends the given messages to the model and returns the trimmed reply.
    pub(super) async fn complete(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<String, ProviderError> {
        let model_name = &self.config.model;
        let chat_request = ChatRequest::new(messages);
        let mut chat_options = ChatOptions::default();

        if let Some(max_tokens) = self.config.max_tokens {
            chat_options = chat_options.with_max_tokens(max_tokens);
        }

        if let Some(temperature) = self.config.temperature {
            chat_options = chat_options.with_temperature(temperature as f64)
        }

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
    use crate::ai::{Provider, Reasoning, config::SecretConfig};

    // serialise tests that mutate env vars to prevent parallel-test race
    // conditions
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_config() -> AiConfig {
        // caller is responsible for holding ENV_LOCK while calling this
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        AiConfig {
            provider: Some(Provider(AdapterKind::OpenAI)),
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(500),
            reasoning: Reasoning::default(),
            max_retries: 3,
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
    fn test_start_conversation_seeds_system_and_user_messages() {
        let _guard = ENV_LOCK.lock().unwrap();
        let client = Client::new(test_config()).unwrap();

        let conversation =
            client.start_conversation("diff", &CommitContext::default(), &CommitConfig::default());

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }

        assert_eq!(conversation.len(), 2);
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
