//! Translates cocoa's AI configuration into per-request chat options.

use genai::chat::ChatOptions;

use super::AiConfig;

/// Builds the chat options to send with each request.
///
/// Only settings the user configured are included, so providers apply their own
/// defaults for the rest.
pub fn build_chat_options(config: &AiConfig) -> ChatOptions {
    let mut options = ChatOptions::default();

    if let Some(max_tokens) = config.max_tokens {
        options = options.with_max_tokens(max_tokens);
    }

    if let Some(temperature) = config.temperature {
        options = options.with_temperature(temperature as f64);
    }

    if let Some(effort) = config.reasoning.effort() {
        options = options.with_reasoning_effort(effort.clone());
    }

    options
}

#[cfg(test)]
mod tests {
    use genai::chat::ReasoningEffort;

    use super::*;
    use crate::ai::Reasoning;

    #[test]
    fn test_default_config_sends_low_reasoning_only() {
        let options = build_chat_options(&AiConfig::default());

        assert!(matches!(
            options.reasoning_effort,
            Some(ReasoningEffort::Low)
        ));
        assert_eq!(options.max_tokens, None);
        assert_eq!(options.temperature, None);
    }

    #[test]
    fn test_auto_reasoning_sends_no_effort() {
        let config = AiConfig {
            reasoning: Reasoning::Auto,
            ..AiConfig::default()
        };

        assert!(build_chat_options(&config).reasoning_effort.is_none());
    }

    #[test]
    fn test_configured_reasoning_is_sent() {
        let config = AiConfig {
            reasoning: "high".parse().unwrap(),
            ..AiConfig::default()
        };

        assert!(matches!(
            build_chat_options(&config).reasoning_effort,
            Some(ReasoningEffort::High)
        ));
    }

    #[test]
    fn test_max_tokens_is_sent_only_when_set() {
        let config = AiConfig {
            max_tokens: Some(1234),
            ..AiConfig::default()
        };

        assert_eq!(build_chat_options(&config).max_tokens, Some(1234));
    }

    #[test]
    fn test_temperature_is_sent_only_when_set() {
        let config = AiConfig {
            temperature: Some(0.5),
            ..AiConfig::default()
        };

        assert_eq!(build_chat_options(&config).temperature, Some(0.5));
    }
}
