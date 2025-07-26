use thiserror::Error;

#[derive(Debug, Error)]
pub enum P2PError {
    #[error("Failed to initialize endpoint: {0}")]
    EndpointInit(String),
    
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    
    #[error("Failed to broadcast message: {0}")]
    BroadcastFailed(String),
    
    #[error("Invalid peer address: {0}")]
    InvalidPeerAddr(String),
    
    #[error("Failed to join topic: {0}")]
    JoinTopicFailed(String),
    
    #[error("Failed to leave topic: {0}")]
    LeaveTopicFailed(String),
    
    #[error("Message serialization failed: {0}")]
    SerializationError(String),
    
    #[error("Message signature verification failed")]
    SignatureVerificationFailed,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for P2PError {
    fn from(err: anyhow::Error) -> Self {
        P2PError::Internal(err.to_string())
    }
}

impl From<std::io::Error> for P2PError {
    fn from(err: std::io::Error) -> Self {
        P2PError::Internal(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, P2PError>;