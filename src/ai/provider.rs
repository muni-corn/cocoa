//! ai provider abstraction and implementations

use serde::{Deserialize, Serialize};
use std::fmt;

/// supported ai providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    OpenAi,
    Anthropic,
    Ollama,
    OpenRouter,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::OpenAi => write!(f, "openai"),
            Provider::Anthropic => write!(f, "anthropic"),
            Provider::Ollama => write!(f, "ollama"),
            Provider::OpenRouter => write!(f, "openrouter"),
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = ProviderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Provider::OpenAi),
            "anthropic" => Ok(Provider::Anthropic),
            "ollama" => Ok(Provider::Ollama),
            "openrouter" => Ok(Provider::OpenRouter),
            _ => Err(ProviderError::UnsupportedProvider(s.to_string())),
        }
    }
}

/// errors related to ai providers
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),

    #[error("api key not found: {0}")]
    ApiKeyNotFound(String),

    #[error("configuration error: {0}")]
    Configuration(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("api error: {0}")]
    Api(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::OpenAi.to_string(), "openai");
        assert_eq!(Provider::Anthropic.to_string(), "anthropic");
        assert_eq!(Provider::Ollama.to_string(), "ollama");
        assert_eq!(Provider::OpenRouter.to_string(), "openrouter");
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!("openai".parse::<Provider>().unwrap(), Provider::OpenAi);
        assert_eq!(
            "ANTHROPIC".parse::<Provider>().unwrap(),
            Provider::Anthropic
        );
        assert_eq!("Ollama".parse::<Provider>().unwrap(), Provider::Ollama);
        assert_eq!(
            "openrouter".parse::<Provider>().unwrap(),
            Provider::OpenRouter
        );
    }

    #[test]
    fn test_provider_from_str_invalid() {
        assert!("invalid".parse::<Provider>().is_err());
        assert!("".parse::<Provider>().is_err());
    }
}
