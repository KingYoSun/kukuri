// DTOモジュール
pub mod auth_dto;
pub mod event;
pub mod offline;
pub mod p2p;
pub mod post_dto;
pub mod topic_dto;
pub mod user_dto;

// 共通のレスポンス型
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
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
