//! AI module for commit message generation using multiple LLM providers.

pub mod client;
pub mod config;
pub mod conversation;
mod prompt;
pub mod provider;
pub mod reasoning;

pub use client::Client;
pub use config::{AiConfig, SecretConfig};
pub use conversation::Conversation;
pub use provider::{Provider, ProviderError};
pub use reasoning::Reasoning;
