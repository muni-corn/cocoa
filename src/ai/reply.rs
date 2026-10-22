//! Extracts the text of a model's reply, rejecting replies that cannot be used.

use genai::chat::ChatResponse;

use super::ProviderError;

/// Returns the trimmed text of a model's reply.
///
/// # Errors
///
/// - [`ProviderError::MaxTokensReached`] when the model stopped because it hit
///   the token limit. Checked first, since models that think can use the whole
///   budget on thinking and leave nothing, or a cut-off message, behind.
/// - [`ProviderError::EmptyReply`] when the reply has no text.
pub fn extract_reply(response: &ChatResponse) -> Result<String, ProviderError> {
    if response
        .stop_reason
        .as_ref()
        .is_some_and(|reason| reason.is_max_tokens())
    {
        return Err(ProviderError::MaxTokensReached);
    }

    let text = response.texts().join("");
    let text = text.trim();

    if text.is_empty() {
        return Err(ProviderError::EmptyReply);
    }

    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use genai::{
        ModelIden,
        adapter::AdapterKind,
        chat::{MessageContent, StopReason, Usage},
    };

    use super::*;

    fn response(text: &str, stop_reason: Option<&str>) -> ChatResponse {
        let model = ModelIden::new(AdapterKind::Anthropic, "test-model");

        ChatResponse {
            content: MessageContent::from_text(text),
            reasoning_content: None,
            model_iden: model.clone(),
            provider_model_iden: model,
            stop_reason: stop_reason.map(|reason| StopReason::from(reason.to_string())),
            usage: Usage::default(),
            captured_raw_body: None,
            response_id: None,
        }
    }

    #[test]
    fn test_returns_trimmed_text() {
        let reply = extract_reply(&response("  feat: add thing\n", Some("end_turn"))).unwrap();
        assert_eq!(reply, "feat: add thing");
    }

    #[test]
    fn test_missing_stop_reason_is_accepted() {
        assert_eq!(
            extract_reply(&response("fix: bug", None)).unwrap(),
            "fix: bug"
        );
    }

    #[test]
    fn test_empty_text_is_an_empty_reply() {
        let error = extract_reply(&response("", Some("end_turn"))).unwrap_err();
        assert!(matches!(error, ProviderError::EmptyReply));
    }

    #[test]
    fn test_whitespace_only_text_is_an_empty_reply() {
        let error = extract_reply(&response(" \n\t ", Some("end_turn"))).unwrap_err();
        assert!(matches!(error, ProviderError::EmptyReply));
    }

    #[test]
    fn test_response_with_no_content_parts_is_an_empty_reply() {
        let mut empty = response("", Some("end_turn"));
        empty.content = MessageContent::default();

        let error = extract_reply(&empty).unwrap_err();
        assert!(matches!(error, ProviderError::EmptyReply));
    }

    #[test]
    fn test_max_tokens_with_no_text_is_max_tokens_reached() {
        let error = extract_reply(&response("", Some("max_tokens"))).unwrap_err();
        assert!(matches!(error, ProviderError::MaxTokensReached));
    }

    #[test]
    fn test_max_tokens_with_partial_text_is_max_tokens_reached() {
        // a cut-off message must not slip through as if it were complete
        let error =
            extract_reply(&response("feat: add support for", Some("max_tokens"))).unwrap_err();
        assert!(matches!(error, ProviderError::MaxTokensReached));
    }

    #[test]
    fn test_openai_length_stop_reason_is_max_tokens_reached() {
        let error = extract_reply(&response("feat: add", Some("length"))).unwrap_err();
        assert!(matches!(error, ProviderError::MaxTokensReached));
    }
}
