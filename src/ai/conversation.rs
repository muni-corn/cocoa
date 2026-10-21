//! Multi-turn chat state for commit message generation.

use genai::chat::ChatMessage;

use super::{Client, ProviderError};

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

    /// Returns how many messages the conversation holds.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Returns true when the conversation holds no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
