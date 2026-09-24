// src/api/guard.rs

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
    utils::security::{AuthError, Claims},
};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use jsonwebtoken::{decode, DecodingKey, Validation};

// 这是一个"空结构体"包装器，专门用于标记"必须是管理员"
#[allow(dead_code)]
pub struct AdminOnly(pub Claims);

// 1. 实现 Axum 的 FromRequestParts trait
// 这让我们可以直接在 Handler 签名里写: async fn handler(claims: Claims)
#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // A. 尝试从 Authorization Header 获取 (Bearer Token)
        let token = if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) {
            auth_header
                .to_str()
                .map_err(|_| AuthError::InvalidToken)?
                .strip_prefix("Bearer ")
                .unwrap_or_default()
        } else {
            ""
        };

        // B. 如果 Header 没有，尝试从 Cookie 获取
        // 为了不引入 axum-extra 增加复杂度，这里手动解析一下 Cookie
        let token = if token.is_empty() {
            parts
                .headers
                .get(header::COOKIE)
                .and_then(|value| value.to_str().ok())
                .and_then(|cookie_str| {
                    cookie_str.split(';').find_map(|s| {
                        let s = s.trim();
                        if s.starts_with("access_token_cookie=") {
                            Some(s.trim_start_matches("access_token_cookie="))
                        } else {
                            None
                        }
                    })
                })
                .ok_or(AuthError::WrongCredentials)?
        } else {
            token
        };

        if token.is_empty() {
            return Err(AuthError::WrongCredentials);
        }

        // C. 解码并验证 JWT
        let decoding_key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
        let validation = Validation::default();

        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

// 2. 实现 AdminOnly 的提取逻辑
// 只有当 Token 有效 且 role == "admin" 时通过
#[async_trait]
impl FromRequestParts<AppState> for AdminOnly {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 复用上面的 Claims 提取逻辑
        let claims = Claims::from_request_parts(parts, state).await?;

        if claims.role == "admin" {
            Ok(AdminOnly(claims))
        } else {
            // [修复] 非管理员返回 403 而非 401 ✓
            // 401: 认证失败（无 token 或 token 无效）
            // 403: 认证成功但权限不足
            Err(AuthError::Forbidden)
        }
    }
}

/// 读权限：admin 全通；vendor + `access = "all"` 全通；vendor + `access = "event"`
/// 只能碰 token 里钉着的那一场。
///
/// **这份实现原本在 `api/order.rs` 和 `api/product.rs` 里各抄了一份一模一样的**，
/// `api/lot.rs` 会是第三份。收到这里来，三处共用。
pub fn check_read_permission(claims: &Claims, event_id: i64) -> ApiResult<()> {
    if claims.role == "admin" {
        return Ok(());
    }
    if claims.role == "vendor" {
        if claims.access == "all" {
            return Ok(());
        }
        if let Some(eid) = claims.event_id {
            if eid == event_id {
                return Ok(());
            }
        }
    }
    Err(ApiError::Forbidden)
}

/// 写权限目前与读权限一致。保留两个名字是因为调用点读起来意图不同，
/// 将来要收紧写权限时也有地方下手。
pub fn check_write_permission(claims: &Claims, event_id: i64) -> ApiResult<()> {
    check_read_permission(claims, event_id)
}

/// 已结算的展会账已冻结，不能再改账。
///
/// **判据是「不是已结算」，不是「必须进行中」**——筹备阶段本来就要能选品、
/// 改价、录带货数。要求「必须进行中」的只有下单一处（`api/order.rs` 的
/// `create_order`，②-1 Task 7 加的），那条更严的检查保持独立，不要合并进来。
///
/// 参数是 `&mut SqliteConnection` 而不是 `&SqlitePool`：在事务里检查才是真的守住，
/// 事务外查一遍再进事务写，中间隔着一个可以被 `settle` 插进来的窗口。
///
/// 冻结之后**仍然允许**三件事，它们不调用本函数（spec 偏离 3）：
/// 垫付、结算调整、收摊清点。改动那三处前先读 spec 3.2。
pub async fn require_event_open(conn: &mut sqlx::SqliteConnection, event_id: i64) -> ApiResult<()> {
    let status: Option<String> = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&mut *conn)
        .await?;
    match status.as_deref() {
        None => Err(ApiError::NotFound("展会不存在".into())),
        Some("已结算") => Err(ApiError::Conflict("展会已结算，不能再改账".into())),
        Some(_) => Ok(()),
    }
}
