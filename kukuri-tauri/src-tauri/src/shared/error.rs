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
    #[error("Validation error: {0}")]
    ValidationError(String),
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
            AppError::ValidationError(_) => "VALIDATION_ERROR",
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
            AppError::Database(_) => "データベース処理中にエラーが発生しました。",
            AppError::Network(_) => "ネットワーク通信でエラーが発生しました。",
            AppError::Crypto(_) => "暗号処理でエラーが発生しました。",
            AppError::Storage(_) => "ストレージ操作でエラーが発生しました。",
            AppError::Auth(_) => "認証処理に失敗しました。",
            AppError::Unauthorized(_) => "この操作を行うにはログインが必要です。",
            AppError::NotFound(_) => "対象のデータが見つかりませんでした。",
            AppError::InvalidInput(_) => "入力値に誤りがあります。",
            AppError::ValidationError(_) => "入力の検証でエラーが発生しました。",
            AppError::NostrError(_) => "Nostr処理でエラーが発生しました。",
            AppError::P2PError(_) => "P2P処理でエラーが発生しました。",
            AppError::ConfigurationError(_) => "アプリ設定に問題があります。",
            AppError::SerializationError(_) => "データ変換でエラーが発生しました。",
            AppError::DeserializationError(_) => "データ読み込みでエラーが発生しました。",
            AppError::NotImplemented(_) => "この機能はまだ実装されていません。",
            AppError::Internal(_) => "内部エラーが発生しました。",
        }
        .to_string()
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
