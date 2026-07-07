//! ドメイン横断の共通エラー写像。ドメイン固有の写像(auth の admission 拒否、
//! channel secret の衝突など)は各ハンドラファイルに置く。

use kukuri_cn_core::ApiError;

pub(crate) fn internal_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        error.to_string(),
    )
}
