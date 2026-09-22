//! 统一的 API 错误类型。
//!
//! 这是 ③a「48 个 route 的响应与错误收口成具名类型」的**基建部分**，提前到 ② 来做：
//! ② 写的每个新 handler 一律用它，② 不改的老 route 维持现状，等 ③a 本体统一收。
//! 为什么这么切，见 plan 头部「与 ③a 的关系」。
//!
//! **响应体形状必须是 `{"error": "..."}`**：前端 `frontend/src/services/api.js` 的
//! 自定义 adapter 会把非 2xx 响应手工包成 axios 风格的 `error.response`，而全仓到处在读
//! `err.response?.data?.error`。换形状会让所有错误提示静默变成 undefined。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// 请求本身不合法：字段缺失、数量为负、枚举值不认识。
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    /// 业务状态冲突：库存不足、展会已冻结、订单已取消。
    /// 与 BadRequest 的区别是「请求没毛病，是世界的状态不允许」。
    #[error("{0}")]
    Conflict(String),

    /// 权限不足由 handler 主动返回。Task 5 的 `api/product.rs::check_write_permission`
    /// 才会用到，在那之前它是本枚举里唯一的非 test 未构造变体。
    #[allow(dead_code)]
    #[error("权限不足")]
    Forbidden,

    /// sqlx 的原始错误**绝不进响应体**——它可能带表名、列名甚至具体值。
    /// 只写 stderr，客户端统一看到「数据库错误」。
    #[error("数据库错误")]
    Db(#[from] sqlx::Error),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // 数据库错误在这里落日志——handler 里不需要再各自 eprintln! 一遍。
        if let ApiError::Db(ref e) = self {
            eprintln!("[api] database error: {e}");
        }
        (self.status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
