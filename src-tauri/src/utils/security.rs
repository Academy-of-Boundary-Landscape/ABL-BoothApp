use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bcrypt::{hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;

// 自定义错误类型，用于统一返回 HTTP 错误
pub enum AuthError {
    WrongCredentials,
    TokenCreation,
    InvalidToken,
    Forbidden, // [新增] 用于权限不足 (403)
}

// 实现 IntoResponse，这样 Handler 可以直接返回 Result<T, AuthError>
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "密码错误"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "令牌创建失败"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "无效的令牌"),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "权限不足"), // [新增] 403
        };
        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

// JWT 的载荷 (Payload) 结构，对应 Flask 的 Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // 用户标识 (admin 或 vendor)
    pub role: String,   // "admin" | "vendor"
    pub access: String, // "all" | "event"
    pub event_id: Option<i64>,
    pub exp: usize, // 过期时间戳
}

// 1. 密码哈希（本地应用使用 cost=6 以提升性能）
pub fn hash_password(password: &str) -> String {
    hash(password, 6).expect("Failed to hash password")
}

// 2. 密码验证
pub fn verify_password(password: &str, hash: &str) -> bool {
    verify(password, hash).unwrap_or(false)
}

// 3. 生成 JWT
pub fn create_jwt(
    role: &str,
    access: &str,
    event_id: Option<i64>,
    secret: &str,
) -> Result<String, AuthError> {
    // 设置过期时间 (例如 24 小时)
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: role.to_owned(),
        role: role.to_owned(),
        access: access.to_owned(),
        event_id,
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AuthError::TokenCreation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let h = hash_password("admin123");
        assert!(verify_password("admin123", &h));
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let h = hash_password("admin123");
        assert!(!verify_password("admin124", &h));
    }

    #[test]
    fn same_plaintext_hashes_differently() {
        // bcrypt 每次用新的 salt，两次哈希不该相同
        assert_ne!(hash_password("same"), hash_password("same"));
    }

    #[test]
    fn verify_rejects_malformed_hash() {
        // verify 内部是 unwrap_or(false)，畸形 hash 不能 panic
        assert!(!verify_password("anything", "not-a-bcrypt-hash"));
    }
}
