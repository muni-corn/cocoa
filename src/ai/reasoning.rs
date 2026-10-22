//! Reasoning effort configuration for models that can think before replying.

use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use genai::chat::ReasoningEffort;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use super::ProviderError;

/// How much a model should reason before it replies.
///
/// Parsed from the `reasoning` key in the `[ai]` configuration section. Accepts
/// `auto`, `zero`, `low`, `medium`, `high`, `xhigh`, `max`, `minimal`, or a
/// number for a provider-specific token budget.
#[derive(Debug, Clone)]
pub enum Reasoning {
    /// Sends no reasoning setting, leaving the choice to the provider.
    ///
    /// Use this for models that cannot reason, since some providers reject a
    /// reasoning setting for them.
    Auto,
    /// Sends the given reasoning effort to the provider.
    Effort(ReasoningEffort),
}

impl Reasoning {
    /// Returns the effort to send to the provider, if one should be sent.
    pub fn effort(&self) -> Option<&ReasoningEffort> {
        match self {
            Self::Auto => None,
            Self::Effort(effort) => Some(effort),
        }
    }
}

impl Default for Reasoning {
    fn default() -> Self {
        // commit messages rarely need deep thinking, and a small amount keeps
        // thinking models from spending their whole output budget on it
        Self::Effort(ReasoningEffort::Low)
    }
}

impl PartialEq for Reasoning {
    fn eq(&self, other: &Self) -> bool {
        // genai's effort type has no equality, but its display form is unique
        self.to_string() == other.to_string()
    }
}

impl Eq for Reasoning {}

impl Display for Reasoning {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Effort(effort) => write!(f, "{effort}"),
        }
    }
}

impl FromStr for Reasoning {
    type Err = ProviderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase();

        if normalized == "auto" {
            return Ok(Self::Auto);
        }

        normalized
            .parse::<ReasoningEffort>()
            .map(Self::Effort)
            .map_err(|_| {
                ProviderError::Configuration(format!(
                    "unknown reasoning level '{s}' - use auto, zero, low, medium, high, xhigh, \
                     max, or a token budget"
                ))
            })
    }
}

impl Serialize for Reasoning {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Reasoning {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // toml bare numbers arrive as integers, so accept both shapes
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Text(String),
            Budget(u32),
        }

        match Raw::deserialize(deserializer)? {
            Raw::Text(text) => text.parse().map_err(D::Error::custom),
            Raw::Budget(budget) => Ok(Self::Effort(ReasoningEffort::Budget(budget))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_low() {
        assert_eq!(Reasoning::default().to_string(), "low");
        assert!(matches!(
            Reasoning::default().effort(),
            Some(ReasoningEffort::Low)
        ));
    }

    #[test]
    fn test_auto_sends_no_effort() {
        let reasoning: Reasoning = "auto".parse().unwrap();
        assert!(reasoning.effort().is_none());
    }

    #[test]
    fn test_parses_every_keyword() {
        for keyword in [
            "zero", "none", "minimal", "low", "medium", "high", "xhigh", "max",
        ] {
            let reasoning: Reasoning = keyword.parse().unwrap();
            assert!(
                reasoning.effort().is_some(),
                "{keyword} should send an effort"
            );
        }
    }

    #[test]
    fn test_parse_is_case_insensitive() {
        assert_eq!("HIGH".parse::<Reasoning>().unwrap().to_string(), "high");
        assert_eq!("Auto".parse::<Reasoning>().unwrap(), Reasoning::Auto);
    }

    #[test]
    fn test_parses_token_budget() {
        let reasoning: Reasoning = "2048".parse().unwrap();
        assert!(matches!(
            reasoning.effort(),
            Some(ReasoningEffort::Budget(2048))
        ));
    }

    #[test]
    fn test_parse_rejects_unknown_level() {
        let error = "very-high".parse::<Reasoning>().unwrap_err();
        assert!(error.to_string().contains("very-high"));
    }

    #[test]
    fn test_serde_round_trips_lowercase() {
        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            reasoning: Reasoning,
        }

        let parsed: Wrapper = toml::from_str(r#"reasoning = "medium""#).unwrap();
        assert_eq!(parsed.reasoning.to_string(), "medium");
        assert_eq!(
            toml::to_string(&parsed).unwrap().trim(),
            r#"reasoning = "medium""#
        );
    }

    #[test]
    fn test_deserializes_bare_number_as_budget() {
        #[derive(Deserialize)]
        struct Wrapper {
            reasoning: Reasoning,
        }

        let parsed: Wrapper = toml::from_str("reasoning = 4096").unwrap();
        assert!(matches!(
            parsed.reasoning.effort(),
            Some(ReasoningEffort::Budget(4096))
        ));
    }

    #[test]
    fn test_deserialize_rejects_unknown_level() {
        #[derive(Deserialize)]
        struct Wrapper {
            #[allow(dead_code)]
            reasoning: Reasoning,
        }

        assert!(toml::from_str::<Wrapper>(r#"reasoning = "bogus""#).is_err());
    }
}
