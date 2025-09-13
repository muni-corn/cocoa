//! ai module for commit message generation using multiple llm providers

pub mod client;
pub mod config;
pub mod provider;

pub use client::Client;
pub use config::AiConfig;
pub use provider::{Provider, ProviderError};
