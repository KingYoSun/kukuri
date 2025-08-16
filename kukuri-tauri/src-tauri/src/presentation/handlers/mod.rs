pub mod topic_handler;
pub mod post_handler;
pub mod auth_handler;
pub mod user_handler;
pub mod secure_storage_handler;
pub mod event_handler;
pub mod p2p_handler;
pub mod offline_handler;

pub use topic_handler::TopicHandler;
pub use post_handler::PostHandler;
pub use auth_handler::AuthHandler;
pub use user_handler::UserHandler;
pub use secure_storage_handler::SecureStorageHandler;
pub use event_handler::EventHandler;
pub use p2p_handler::P2PHandler;
pub use offline_handler::OfflineHandler;

use crate::shared::error::AppError;

/// エラーをAPIレスポンスに変換
pub fn handle_error<T>(result: Result<T, AppError>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

/// 入力検証を実行
pub fn validate_input<T: super::dto::Validate>(input: &T) -> Result<(), String> {
    input.validate()
}