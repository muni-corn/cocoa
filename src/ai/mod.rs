//! AI module for commit message generation using multiple LLM providers.

pub mod client;
pub mod config;
mod prompt;
pub mod provider;

pub use client::Client;
pub use config::{AiConfig, SecretConfig};
pub use provider::{Provider, ProviderError};
