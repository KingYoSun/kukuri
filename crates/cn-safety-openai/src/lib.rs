//! Dedicated OpenAI Moderation adapter. No chat completion or implicit credentials.
mod budget;
mod client;
mod config;
mod provider;
mod response;

pub use budget::{BudgetConfig, MemoryModerationBudget, ModerationBudget};
pub use client::{ModerationClient, ModerationInput};
pub use config::{API_KEY_ENV, ModerationConfig, ModerationCredentials};
pub use provider::OpenAiModerationProvider;
pub use response::{CategoryAssessment, ModerationAssessment};

pub const PROVIDER_NAME: &str = "openai-moderation";
pub const PREPROCESSING_VERSION: &str = "openai-moderation-v1";
