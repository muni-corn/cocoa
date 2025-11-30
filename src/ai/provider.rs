//! ai provider abstraction and implementations

use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use genai::adapter::AdapterKind;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// supported ai providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Provider(#[serde(deserialize_with = "de_provider")] pub AdapterKind);

impl Display for Provider {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string().to_lowercase())
    }
}

impl FromStr for Provider {
    type Err = ProviderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let adapter_kind = AdapterKind::from_lower_str(s.to_lowercase().as_str())
            .ok_or(ProviderError::UnsupportedProvider(s.to_string()))?;
        Ok(Self(adapter_kind))
    }
}

/// custom deserializer for case-insensitive provider names
fn de_provider<'de, D>(deserializer: D) -> Result<AdapterKind, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    AdapterKind::from_lower_str(&s.to_lowercase())
        .ok_or_else(|| D::Error::custom(format!("unsupported provider: {}", s)))
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
        assert_eq!(Provider(AdapterKind::OpenAI).to_string(), "openai");
        assert_eq!(Provider(AdapterKind::Anthropic).to_string(), "anthropic");
        assert_eq!(Provider(AdapterKind::Ollama).to_string(), "ollama");
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!(
            "openai".parse::<Provider>().unwrap(),
            Provider(AdapterKind::OpenAI)
        );
        assert_eq!(
            "ANTHROPIC".parse::<Provider>().unwrap(),
            Provider(AdapterKind::Anthropic)
        );
        assert_eq!(
            "Ollama".parse::<Provider>().unwrap(),
            Provider(AdapterKind::Ollama)
        );
    }

    #[test]
    fn test_provider_from_str_invalid() {
        assert!("invalid".parse::<Provider>().is_err());
        assert!("".parse::<Provider>().is_err());
    }
}
