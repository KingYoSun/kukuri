use crate::shared::validation::ValidationFailureKind;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Validation error ({kind}): {message}")]
    ValidationError {
        kind: ValidationFailureKind,
        message: String,
    },
    #[error("Nostr error: {0}")]
    NostrError(String),
    #[error("P2P error: {0}")]
    P2PError(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn validation(kind: ValidationFailureKind, message: impl Into<String>) -> Self {
        AppError::ValidationError {
            kind,
            message: message.into(),
        }
    }

    pub fn validation_kind(&self) -> Option<ValidationFailureKind> {
        match self {
            AppError::ValidationError { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    pub fn validation_message(&self) -> Option<&str> {
        match self {
            AppError::ValidationError { message, .. } => Some(message.as_str()),
            _ => None,
        }
    }

    pub fn validation_mapper(kind: ValidationFailureKind) -> impl FnOnce(String) -> Self {
        move |message| AppError::validation(kind, message)
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Network(_) => "NETWORK_ERROR",
            AppError::Crypto(_) => "CRYPTO_ERROR",
            AppError::Storage(_) => "STORAGE_ERROR",
            AppError::Auth(_) => "AUTH_ERROR",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::InvalidInput(_) => "INVALID_INPUT",
            AppError::ValidationError { .. } => "VALIDATION_ERROR",
            AppError::NostrError(_) => "NOSTR_ERROR",
            AppError::P2PError(_) => "P2P_ERROR",
            AppError::ConfigurationError(_) => "CONFIGURATION_ERROR",
            AppError::SerializationError(_) => "SERIALIZATION_ERROR",
            AppError::DeserializationError(_) => "DESERIALIZATION_ERROR",
            AppError::NotImplemented(_) => "NOT_IMPLEMENTED",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            AppError::Database(_) => "Database operation failed".to_string(),
            AppError::Network(_) => "Network request failed".to_string(),
            AppError::Crypto(_) => "Cryptographic operation failed".to_string(),
            AppError::Storage(_) => "Storage access failed".to_string(),
            AppError::Auth(_) => "Authentication failed".to_string(),
            AppError::Unauthorized(_) => {
                "You are not authorized to perform this action".to_string()
            }
            AppError::NotFound(_) => "The requested resource was not found".to_string(),
            AppError::InvalidInput(_) => "Input data is invalid".to_string(),
            AppError::ValidationError { message, .. } => {
                format!("Validation failed: {}", message)
            }
            AppError::NostrError(_) => "Nostr operation failed".to_string(),
            AppError::P2PError(_) => "Peer-to-peer operation failed".to_string(),
            AppError::ConfigurationError(_) => "Configuration error detected".to_string(),
            AppError::SerializationError(_) => "Serialization error occurred".to_string(),
            AppError::DeserializationError(_) => "Deserialization error occurred".to_string(),
            AppError::NotImplemented(_) => "This feature is not implemented".to_string(),
            AppError::Internal(_) => "An internal error occurred".to_string(),
        }
    }
}

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
        AppError::P2PError(format!("Endpoint builder error: {err:?}"))
    }
}

impl From<nostr_sdk::key::vanity::Error> for AppError {
    fn from(err: nostr_sdk::key::vanity::Error) -> Self {
        AppError::NostrError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
