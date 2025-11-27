//! mock ai client for testing commit generation without real api calls

use cocoa::ai::config::SecretConfig;
use cocoa::ai::{AiConfig, Provider};

/// mock ai response generator for testing
pub struct MockAiClient {
    /// predefined responses to return
    pub responses: Vec<String>,
    /// current response index
    pub call_count: usize,
}

impl MockAiClient {
    /// create a new mock client with predefined responses
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: 0,
        }
    }

    /// create a mock with a single response
    pub fn with_response(response: String) -> Self {
        Self::new(vec![response])
    }

    /// get the next response (cycles through responses)
    pub fn next_response(&mut self) -> String {
        let response = self.responses[self.call_count % self.responses.len()].clone();
        self.call_count += 1;
        response
    }

    /// reset call count
    pub fn reset(&mut self) {
        self.call_count = 0;
    }
}

/// create a test ai configuration (does not require real api key)
pub fn test_ai_config() -> AiConfig {
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

/// common mock responses for testing
pub mod responses {
    pub const VALID_FEAT: &str = "feat: add new feature";
    pub const VALID_FIX: &str = "fix: resolve bug in parser";
    pub const VALID_DOCS: &str = "docs: update readme";
    pub const WITH_SCOPE: &str = "feat(api): add new endpoint";
    pub const WITH_BREAKING: &str = "feat!: breaking change\n\nBREAKING CHANGE: removed old api";
    pub const INVALID: &str = "bad commit message format";
    pub const LONG_SUBJECT: &str = "feat: this is a very long subject line that exceeds the maximum allowed length and should trigger a validation warning or error";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_client_single_response() {
        let mut mock = MockAiClient::with_response("test response".to_string());

        assert_eq!(mock.next_response(), "test response");
        assert_eq!(mock.call_count, 1);
    }

    #[test]
    fn test_mock_client_multiple_responses() {
        let mut mock = MockAiClient::new(vec![
            "response 1".to_string(),
            "response 2".to_string(),
            "response 3".to_string(),
        ]);

        assert_eq!(mock.next_response(), "response 1");
        assert_eq!(mock.next_response(), "response 2");
        assert_eq!(mock.next_response(), "response 3");
        assert_eq!(mock.call_count, 3);
    }

    #[test]
    fn test_mock_client_cycles_responses() {
        let mut mock = MockAiClient::new(vec!["response 1".to_string(), "response 2".to_string()]);

        assert_eq!(mock.next_response(), "response 1");
        assert_eq!(mock.next_response(), "response 2");
        assert_eq!(mock.next_response(), "response 1"); // cycles back
        assert_eq!(mock.call_count, 3);
    }

    #[test]
    fn test_mock_client_reset() {
        let mut mock = MockAiClient::with_response("test".to_string());

        mock.next_response();
        assert_eq!(mock.call_count, 1);

        mock.reset();
        assert_eq!(mock.call_count, 0);
    }

    #[test]
    fn test_ai_config_creation() {
        let config = test_ai_config();
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.7);
    }
}
