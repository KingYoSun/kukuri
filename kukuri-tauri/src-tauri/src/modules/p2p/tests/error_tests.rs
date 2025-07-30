#[cfg(test)]
mod tests {
    use crate::modules::p2p::error::*;
    use std::io;

    #[test]
    fn test_error_display() {
        let err = P2PError::EndpointInit("Failed to bind".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to initialize endpoint: Failed to bind"
        );

        let err = P2PError::TopicNotFound("test-topic".to_string());
        assert_eq!(err.to_string(), "Topic not found: test-topic");

        let err = P2PError::BroadcastFailed("Network error".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to broadcast message: Network error"
        );

        let err = P2PError::InvalidPeerAddr("invalid:addr".to_string());
        assert_eq!(err.to_string(), "Invalid peer address: invalid:addr");
    }

    #[test]
    fn test_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("Test error");
        let p2p_err: P2PError = anyhow_err.into();

        match p2p_err {
            P2PError::Internal(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let p2p_err: P2PError = io_err.into();

        match p2p_err {
            P2PError::Internal(msg) => assert!(msg.contains("File not found")),
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_result_type() {
        fn test_function() -> Result<String> {
            Ok("Success".to_string())
        }

        let result = test_function();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
    }

    #[test]
    fn test_all_error_variants() {
        let errors = vec![
            P2PError::EndpointInit("test".to_string()),
            P2PError::TopicNotFound("test".to_string()),
            P2PError::BroadcastFailed("test".to_string()),
            P2PError::InvalidPeerAddr("test".to_string()),
            P2PError::JoinTopicFailed("test".to_string()),
            P2PError::LeaveTopicFailed("test".to_string()),
            P2PError::SerializationError("test".to_string()),
            P2PError::SignatureVerificationFailed,
            P2PError::Internal("test".to_string()),
        ];

        for err in errors {
            // すべてのエラーがDebugとDisplayを実装していることを確認
            let _ = format!("{:?}", err);
            let _ = err.to_string();
        }
    }
}
