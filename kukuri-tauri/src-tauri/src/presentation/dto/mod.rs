// DTOモジュール
pub mod auth_dto;
pub mod community_node_dto;
pub mod direct_message_dto;
pub mod event;
pub mod offline;
pub mod p2p;
pub mod post_dto;
pub mod profile_avatar_dto;
pub mod topic_dto;
pub mod user_dto;

// 共通のレスポンス型
use crate::shared::AppError;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub error_details: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            error_code: None,
            error_details: None,
        }
    }

    pub fn from_app_error(error: AppError) -> Self {
        let error_details = match error {
            AppError::RateLimited {
                retry_after_seconds,
                ..
            } => Some(json!({ "retry_after_seconds": retry_after_seconds })),
            _ => None,
        };

        Self {
            success: false,
            data: None,
            error: Some(error.user_message()),
            error_code: Some(error.code().to_string()),
            error_details,
        }
    }

    pub fn from_result(result: crate::shared::Result<T>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(err) => Self::from_app_error(err),
        }
    }
}

// ページネーション
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginationRequest {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for PaginationRequest {
    fn default() -> Self {
        Self {
            limit: Some(50),
            offset: Some(0),
        }
    }
}

// バリデーショントレイト
pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}
