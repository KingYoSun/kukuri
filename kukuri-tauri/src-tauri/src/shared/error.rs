use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Database(String),
    Network(String),
    Crypto(String),
    Storage(String),
    Auth(String),
    Unauthorized(String),
    NotFound(String),
    InvalidInput(String),
    ValidationError(String),
    NostrError(String),
    P2PError(String),
    ConfigurationError(String),
    SerializationError(String),
    DeserializationError(String),
    NotImplemented(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(msg) => write!(f, "Database error: {}", msg),
            AppError::Network(msg) => write!(f, "Network error: {}", msg),
            AppError::Crypto(msg) => write!(f, "Crypto error: {}", msg),
            AppError::Storage(msg) => write!(f, "Storage error: {}", msg),
            AppError::Auth(msg) => write!(f, "Auth error: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::NostrError(msg) => write!(f, "Nostr error: {}", msg),
            AppError::P2PError(msg) => write!(f, "P2P error: {}", msg),
            AppError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            AppError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            AppError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for AppError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::Internal(err)
    }
}

impl From<&str> for AppError {
    fn from(err: &str) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<nostr_sdk::prelude::EventId> for AppError {
    fn from(_: nostr_sdk::prelude::EventId) -> Self {
        AppError::NostrError("Invalid EventId conversion".to_string())
    }
}

impl From<nostr_sdk::event::Error> for AppError {
    fn from(err: nostr_sdk::event::Error) -> Self {
        AppError::NostrError(err.to_string())
    }
}

impl From<nostr_sdk::prelude::secp256k1::Error> for AppError {
    fn from(err: nostr_sdk::prelude::secp256k1::Error) -> Self {
        AppError::Crypto(err.to_string())
    }
}

impl From<nostr_sdk::key::Error> for AppError {
    fn from(err: nostr_sdk::key::Error) -> Self {
        AppError::NostrError(err.to_string())
    }
}

impl From<nostr_sdk::event::builder::Error> for AppError {
    fn from(err: nostr_sdk::event::builder::Error) -> Self {
        AppError::NostrError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::P2PError(err.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<iroh::endpoint::Builder> for AppError {
    fn from(err: iroh::endpoint::Builder) -> Self {
        AppError::P2PError(format!("Endpoint builder error: {:?}", err))
    }
}

impl From<nostr_sdk::key::vanity::Error> for AppError {
    fn from(err: nostr_sdk::key::vanity::Error) -> Self {
        AppError::NostrError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
