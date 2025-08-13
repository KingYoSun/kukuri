use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Database(String),
    Network(String),
    Crypto(String),
    Storage(String),
    Auth(String),
    NotFound(String),
    InvalidInput(String),
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
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
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

pub type Result<T> = std::result::Result<T, AppError>;