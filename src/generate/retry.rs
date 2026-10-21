//! Retry loop that asks a model to fix commit messages failing validation.

use crate::{
    ai::{Conversation, ProviderError},
    generate::GenerateError,
};

/// A source of commit messages that can be asked to correct its last reply.
///
/// Implemented by [`Conversation`] for real models, and by scripted fakes in
/// tests so the retry loop can run without network access.
#[allow(
    async_fn_in_trait,
    reason = "implementors are only awaited on the caller's own task"
)]
pub trait MessageSource {
    /// Produces the first commit message.
    async fn initial(&mut self) -> Result<String, ProviderError>;

    /// Produces a corrected message after the last one broke the given rules.
    async fn retry(&mut self, violations: &[String]) -> Result<String, ProviderError>;
}

impl MessageSource for Conversation<'_> {
    async fn initial(&mut self) -> Result<String, ProviderError> {
        self.send().await
    }

    async fn retry(&mut self, violations: &[String]) -> Result<String, ProviderError> {
        Conversation::retry(self, violations).await
    }
}

/// Describes a retry that is about to happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryNotice {
    /// The attempt about to be made, counting the first attempt as 1.
    pub attempt: u32,
    /// The most attempts that will be made, including the first.
    pub max_attempts: u32,
    /// Why the previous attempt was rejected.
    pub violations: Vec<String>,
}

/// A valid message along with how many attempts it took to get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The message that passed validation.
    pub message: String,
    /// How many attempts were made, counting the first as 1.
    pub attempts: u32,
}

/// Generates a message, asking the source to try again while validation fails.
///
/// # Params
///
/// - `source`: Where messages come from.
/// - `max_retries`: How many extra attempts are allowed after the first. Zero
///   disables retrying.
/// - `validate`: Returns every violation when a message is not acceptable.
/// - `on_retry`: Called just before each retry, so callers can report it.
///
/// # Errors
///
/// Returns [`GenerateError::AiGeneration`] if the source fails, and
/// [`GenerateError::Validation`] if the message is still invalid once the
/// attempts run out.
pub async fn generate_with_retries<S, V, N>(
    source: &mut S,
    max_retries: u32,
    validate: V,
    mut on_retry: N,
) -> Result<Outcome, GenerateError>
where
    S: MessageSource,
    V: Fn(&str) -> Result<(), Vec<String>>,
    N: FnMut(&RetryNotice),
{
    let max_attempts = max_retries.saturating_add(1);
    let mut attempt = 1;
    let mut message = source.initial().await.map_err(ai_error)?;

    loop {
        let violations = match validate(&message) {
            Ok(()) => {
                return Ok(Outcome {
                    message,
                    attempts: attempt,
                });
            }
            Err(violations) => violations,
        };

        if attempt >= max_attempts {
            return Err(exhausted_error(attempt, max_retries, &violations));
        }

        attempt += 1;
        on_retry(&RetryNotice {
            attempt,
            max_attempts,
            violations: violations.clone(),
        });

        message = source.retry(&violations).await.map_err(ai_error)?;
    }
}

fn ai_error(error: ProviderError) -> GenerateError {
    GenerateError::AiGeneration(error.to_string())
}

fn exhausted_error(attempts: u32, max_retries: u32, violations: &[String]) -> GenerateError {
    let joined = violations.join("; ");

    // only mention attempts when retrying was actually allowed
    if max_retries == 0 {
        GenerateError::Validation(joined)
    } else {
        GenerateError::Validation(format!("still invalid after {attempts} attempts: {joined}"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    /// Replies from a script, recording the violations sent with each retry.
    struct Scripted {
        replies: VecDeque<Result<String, ProviderError>>,
        retry_feedback: Vec<Vec<String>>,
        calls: u32,
    }

    impl Scripted {
        fn new(replies: Vec<Result<String, ProviderError>>) -> Self {
            Self {
                replies: replies.into(),
                retry_feedback: Vec::new(),
                calls: 0,
            }
        }

        fn ok(replies: &[&str]) -> Self {
            Self::new(replies.iter().map(|r| Ok(r.to_string())).collect())
        }

        fn next(&mut self) -> Result<String, ProviderError> {
            self.calls += 1;
            self.replies.pop_front().expect("script ran out of replies")
        }
    }

    impl MessageSource for Scripted {
        async fn initial(&mut self) -> Result<String, ProviderError> {
            self.next()
        }

        async fn retry(&mut self, violations: &[String]) -> Result<String, ProviderError> {
            self.retry_feedback.push(violations.to_vec());
            self.next()
        }
    }

    // accepts only "good", rejecting everything else with one violation
    fn only_good(message: &str) -> Result<(), Vec<String>> {
        if message == "good" {
            Ok(())
        } else {
            Err(vec![format!("rule: {message} is bad")])
        }
    }

    #[tokio::test]
    async fn test_valid_first_try_makes_one_call() {
        let mut source = Scripted::ok(&["good"]);
        let mut notices = Vec::new();

        let outcome = generate_with_retries(&mut source, 3, only_good, |n| notices.push(n.clone()))
            .await
            .unwrap();

        assert_eq!(outcome.message, "good");
        assert_eq!(outcome.attempts, 1);
        assert_eq!(source.calls, 1);
        assert!(notices.is_empty());
    }

    #[tokio::test]
    async fn test_valid_after_retries() {
        let mut source = Scripted::ok(&["bad1", "bad2", "good"]);
        let mut notices = Vec::new();

        let outcome = generate_with_retries(&mut source, 3, only_good, |n| notices.push(n.clone()))
            .await
            .unwrap();

        assert_eq!(outcome.message, "good");
        assert_eq!(outcome.attempts, 3);
        assert_eq!(source.calls, 3);
        assert_eq!(notices.len(), 2);
    }

    #[tokio::test]
    async fn test_retry_notices_report_progress_and_reasons() {
        let mut source = Scripted::ok(&["bad1", "good"]);
        let mut notices = Vec::new();

        generate_with_retries(&mut source, 3, only_good, |n| notices.push(n.clone()))
            .await
            .unwrap();

        assert_eq!(notices, vec![RetryNotice {
            attempt: 2,
            max_attempts: 4,
            violations: vec!["rule: bad1 is bad".to_string()],
        }]);
    }

    #[tokio::test]
    async fn test_violations_are_sent_back_to_the_source() {
        let mut source = Scripted::ok(&["bad1", "bad2", "good"]);

        generate_with_retries(&mut source, 3, only_good, |_| {})
            .await
            .unwrap();

        assert_eq!(source.retry_feedback, vec![
            vec!["rule: bad1 is bad".to_string()],
            vec!["rule: bad2 is bad".to_string()],
        ]);
    }

    #[tokio::test]
    async fn test_gives_up_after_max_retries() {
        let mut source = Scripted::ok(&["bad", "bad", "bad", "bad", "bad"]);

        let error = generate_with_retries(&mut source, 3, only_good, |_| {})
            .await
            .unwrap_err();

        // one initial call plus three retries
        assert_eq!(source.calls, 4);
        match error {
            GenerateError::Validation(msg) => {
                assert!(msg.contains("still invalid after 4 attempts"));
                assert!(msg.contains("rule: bad is bad"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_zero_retries_never_retries() {
        let mut source = Scripted::ok(&["bad", "good"]);
        let mut notices = Vec::new();

        let error = generate_with_retries(&mut source, 0, only_good, |n| notices.push(n.clone()))
            .await
            .unwrap_err();

        assert_eq!(source.calls, 1);
        assert!(notices.is_empty());
        match error {
            GenerateError::Validation(msg) => assert_eq!(msg, "rule: bad is bad"),
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_provider_error_on_first_call_is_ai_error() {
        let mut source = Scripted::new(vec![Err(ProviderError::Api("boom".to_string()))]);

        let error = generate_with_retries(&mut source, 3, only_good, |_| {})
            .await
            .unwrap_err();

        assert!(matches!(error, GenerateError::AiGeneration(msg) if msg.contains("boom")));
    }

    #[tokio::test]
    async fn test_provider_error_during_retry_is_ai_error() {
        let mut source = Scripted::new(vec![
            Ok("bad".to_string()),
            Err(ProviderError::Network("offline".to_string())),
        ]);

        let error = generate_with_retries(&mut source, 3, only_good, |_| {})
            .await
            .unwrap_err();

        assert!(matches!(error, GenerateError::AiGeneration(msg) if msg.contains("offline")));
        assert_eq!(source.calls, 2);
    }
}
