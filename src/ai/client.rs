//! generic ai client wrapper

use std::fmt::Debug;

use genai::{
    Client as GenaiClient,
    adapter::AdapterKind,
    chat::{ChatMessage, ChatOptions, ChatRequest},
};

use super::{AiConfig, ProviderError};

/// generic ai client for generating commit messages
pub struct Client {
    config: AiConfig,
    client: GenaiClient,
}

impl Client {
    /// create a new ai client with the given configuration
    pub fn new(config: AiConfig) -> Result<Self, ProviderError> {
        // validate and get api key
        let api_key = config.secret.resolve_api_key()?;

        // create genai client
        let client = GenaiClient::default();

        Ok(Self { config, client })
    }

    /// generate a commit message from the given context
    pub async fn generate_commit_message(
        &self,
        staged_changes: &str,
        context: &CommitContext,
    ) -> Result<String, ProviderError> {
        let model_name = &self.config.model;
        let prompt = self.build_prompt(staged_changes, context);

        let messages = vec![
            ChatMessage::system(
                "you are an expert software engineer who writes excellent conventional commit messages. respond with only the commit message, no explanation or additional text.",
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
            .map_err(|e| ProviderError::Api(format!("genai error: {e}")))?;

        let content = response.texts().join("");

        Ok(content.trim().to_string())
    }

    /// build the prompt for commit generation
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

/// context information for commit generation
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
    use super::*;
    use crate::ai::{Provider, config::SecretConfig};

    fn test_config() -> AiConfig {
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        AiConfig {
            provider: Provider(AdapterKind::OpenAI),
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
        let result = Client::new(config);
        assert!(result.is_ok());

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
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

        let result = Client::new(config);
        assert!(result.is_err());

        unsafe {
            std::env::remove_var("TEST_API_KEY");
        }
    }

    #[test]
    fn test_get_model_name_openai() {
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        let config = AiConfig {
            provider: Provider(AdapterKind::OpenAI),
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
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        let config = AiConfig {
            provider: Provider(AdapterKind::Anthropic),
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
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        let client = Client::new(test_config()).unwrap();
        let context = CommitContext {
            branch_name: Some("feature/test".to_string()),
            recent_commits: vec!["feat: previous commit".to_string()],
            repository_name: Some("test-repo".to_string()),
            is_merge: false,
            is_rebase: false,
        };

        let prompt = client.build_prompt("test diff", &context);

        assert!(prompt.contains("test diff"));
        assert!(prompt.contains("feature/test"));
        assert!(prompt.contains("feat: previous commit"));
        assert!(prompt.contains("conventional commits"));

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    #[test]
    fn test_build_prompt_without_branch() {
        unsafe {
            std::env::set_var("TEST_API_KEY", "test-key");
        }

        let client = Client::new(test_config()).unwrap();
        let context = CommitContext::default();

        let prompt = client.build_prompt("test diff", &context);

        assert!(prompt.contains("test diff"));
        assert!(!prompt.contains("branch name:"));

        unsafe {
            std::env::remove_var("TEST_API_KEY");
            std::env::remove_var("OPENAI_API_KEY");
        }
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
