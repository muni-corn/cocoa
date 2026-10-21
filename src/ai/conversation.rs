//! Multi-turn chat state for commit message generation.

use genai::chat::ChatMessage;

use super::{Client, ProviderError, prompt::build_retry_prompt};

/// A running chat with a model, holding every message exchanged so far.
///
/// Each reply from the model is appended to the history so later turns can
/// refer back to it.
pub struct Conversation<'a> {
    client: &'a Client,
    messages: Vec<ChatMessage>,
}

impl<'a> Conversation<'a> {
    /// Creates a conversation from an initial set of messages.
    pub(super) fn new(client: &'a Client, messages: Vec<ChatMessage>) -> Self {
        Self { client, messages }
    }

    /// Sends the history to the model and records its reply.
    ///
    /// # Returns
    ///
    /// The model's reply with surrounding whitespace trimmed.
    pub async fn send(&mut self) -> Result<String, ProviderError> {
        let reply = self.client.complete(self.messages.clone()).await?;

        // keep the reply so a follow-up turn has the full exchange as context
        self.messages.push(ChatMessage::assistant(reply.clone()));

        Ok(reply)
    }

    /// Tells the model its last reply failed validation and asks it to try
    /// again.
    ///
    /// # Params
    ///
    /// - `violations`: Human-readable descriptions of each problem found.
    ///
    /// # Returns
    ///
    /// The model's new reply with surrounding whitespace trimmed.
    pub async fn retry(&mut self, violations: &[String]) -> Result<String, ProviderError> {
        self.push_retry_feedback(violations);
        self.send().await
    }

    fn push_retry_feedback(&mut self, violations: &[String]) {
        self.messages
            .push(ChatMessage::user(build_retry_prompt(violations)));
    }

    /// Returns how many messages the conversation holds.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Returns true when the conversation holds no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use genai::adapter::AdapterKind;

    use super::*;
    use crate::ai::{AiConfig, Provider, SecretConfig};

    // serialise tests that mutate env vars to prevent parallel-test race
    // conditions
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_push_retry_feedback_appends_one_message() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CONVERSATION_TEST_API_KEY", "test-key");
        }

        let client = Client::new(AiConfig {
            provider: Some(Provider(AdapterKind::OpenAI)),
            model: "gpt-4".to_string(),
            secret: SecretConfig::Env {
                env: "CONVERSATION_TEST_API_KEY".to_string(),
            },
            ..AiConfig::default()
        })
        .unwrap();

        unsafe {
            std::env::remove_var("CONVERSATION_TEST_API_KEY");
        }

        let mut conversation = Conversation::new(&client, vec![ChatMessage::user("hi")]);
        conversation.push_retry_feedback(&["type-enum: bad type".to_string()]);

        assert_eq!(conversation.len(), 2);
    }
}
