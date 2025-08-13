pub mod post_handler;
pub mod topic_handler;
pub mod auth_handler;
pub mod user_handler;

use crate::shared::error::AppError;
use super::dto::ApiResponse;

/// エラーをAPIレスポンスに変換
pub fn handle_error<T>(result: Result<T, AppError>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

/// 入力検証を実行
pub fn validate_input<T: super::dto::Validate>(input: &T) -> Result<(), String> {
    input.validate()
}