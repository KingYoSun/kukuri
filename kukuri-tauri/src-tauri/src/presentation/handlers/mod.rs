pub mod post_handler;
pub mod topic_handler;
pub mod auth_handler;
pub mod user_handler;
pub mod secure_storage_handler;

pub use auth_handler::AuthHandler;
pub use post_handler::PostHandler;
pub use topic_handler::TopicHandler;
pub use user_handler::UserHandler;
pub use secure_storage_handler::SecureStorageHandler;

use crate::shared::error::AppError;

/// エラーをAPIレスポンスに変換
pub fn handle_error<T>(result: Result<T, AppError>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

/// 入力検証を実行
pub fn validate_input<T: super::dto::Validate>(input: &T) -> Result<(), String> {
    input.validate()
}