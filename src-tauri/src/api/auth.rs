use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::{
    async_trait,
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    state::AppState,
    utils::security::{self, AuthError},
};

// 自定义反序列化函数：接受字符串、数字或 null 的 i64
fn deserialize_i64_from_str<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Deserialize, Deserializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrIntOrNull {
        Null,
        String(String),
        Int(i64),
    }

    match StringOrIntOrNull::deserialize(deserializer)? {
        StringOrIntOrNull::Null => Ok(None),
        StringOrIntOrNull::String(s) => {
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse::<i64>()
                    .map(Some)
                    .map_err(|_| de::Error::custom(format!("invalid i64 value: {}", s)))
            }
        }
        StringOrIntOrNull::Int(i) => Ok(Some(i)),
    }
}

// 自定义JSON提取器：委托给内置 Json 提取器，避免重复读取 Body 导致空内容
pub struct DebugJson<T>(pub T);
#[async_trait]
impl<T, S> axum::extract::FromRequest<S> for DebugJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    // 修改为 Response 以便返回自定义错误响应
    type Rejection = Response;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        // 1. 提取原始字节
        let bytes = Bytes::from_request(req, state).await.map_err(|e| {
            eprintln!("[DEBUG] Failed to read body bytes: {}", e);
            e.into_response()
        })?;

        // 2. 调试打印 (仅在调试模式或出错时打印，避免刷屏)
        // println!("[DEBUG] Body received (len={}): {:?}", bytes.len(), bytes);

        // 3. 尝试反序列化
        match serde_json::from_slice::<T>(&bytes) {
            Ok(value) => Ok(DebugJson(value)),
            Err(e) => {
                // 4. 只有在解析失败时，才打印详细的 Body 内容，方便排查
                let body_str = String::from_utf8_lossy(&bytes);
                eprintln!("========================================");
                eprintln!("[DEBUG] JSON Parsing FAILED!");
                eprintln!("[DEBUG] Error: {}", e);
                eprintln!("[DEBUG] Raw Body Content: '{}'", body_str);
                eprintln!("========================================");

                // 返回 400 Bad Request 给前端，而不是 422
                Err((StatusCode::BAD_REQUEST, format!("JSON Parse Error: {}", e)).into_response())
            }
        }
    }
}

// 1. 请求体 DTO
#[derive(Deserialize)]
struct LoginRequest {
    role: String, // "admin" | "vendor"
    password: String,
    #[serde(
        rename = "eventId",
        deserialize_with = "deserialize_i64_from_str",
        default
    )] // 接受字符串或数字
    event_id: Option<i64>,
}

// 2. 响应体 DTO
#[derive(Serialize)]
struct LoginResponse {
    message: String,
    role: String,
    access: String,
    #[serde(rename = "eventId", skip_serializing_if = "Option::is_none")]
    event_id: Option<i64>,
    token: String, // 我们把 token 直接放在 Body 里方便前端拿
}

#[derive(Serialize)]
struct IsDefaultAdminPasswordResponse {
    is_default: bool,
}

// 3. 路由定义
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/logout", post(logout_handler))
        .route("/is-default-admin-password", get(is_default_admin_password))
}

// 4. 处理器逻辑
async fn login_handler(
    State(state): State<AppState>,
    DebugJson(payload): DebugJson<LoginRequest>,
) -> Result<Response, AuthError> {
    // [调试] 确认成功解析 payload
    // println!("[DEBUG] Login handler called successfully");
    // println!(
    //     "[DEBUG] Parsed payload - role: {}, password length: {}, event_id: {:?}",
    //     payload.role,
    //     payload.password.len(),
    //     payload.event_id
    // );

    // event_id 直接从 payload 中获取
    let event_id = payload.event_id;

    match payload.role.as_str() {
        "admin" => {
            // --- 管理员登录逻辑 ---
            // 从数据库获取存储的 hash
            let row: Option<(String,)> =
                sqlx::query_as("SELECT value FROM settings WHERE key = 'admin_password'")
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);

            let stored_hash = row.ok_or(AuthError::WrongCredentials)?.0;

            // [调试] 打印密码验证信息
            // let input_password_hash = security::hash_password(&payload.password);
            // let admin123_hash = security::hash_password("admin123");
            // println!("[DEBUG] Admin Login Attempt:");
            // println!("  Input Password: {}", &payload.password);
            // println!("  Input Password Hash: {}", input_password_hash);
            // println!("  admin123 Hash: {}", admin123_hash);
            // println!("  Stored Hash: {}", stored_hash);
            // println!(
            //     "  Verify Result: {}",
            //     security::verify_password(&payload.password, &stored_hash)
            // );

            if security::verify_password(&payload.password, &stored_hash) {
                let token = security::create_jwt("admin", "all", None, &state.jwt_secret)?;
                return Ok(build_success_response("admin", "all", None, token));
            }
        }
        "vendor" => {
            // --- 摊主登录逻辑 ---

            // [调试] 打印密码验证信息
            // let input_password_hash = security::hash_password(&payload.password);
            // let admin123_hash = security::hash_password("admin123");
            // println!("[DEBUG] Vendor Login Attempt:");
            // println!("  Input Password: {}", &payload.password);
            // println!("  Input Password Hash: {}", input_password_hash);
            // println!("  admin123 Hash: {}", admin123_hash);

            // A. 先尝试全局 Admin 密码 (允许摊主用管理员密码登录)
            let admin_row: Option<(String,)> =
                sqlx::query_as("SELECT value FROM settings WHERE key = 'admin_password'")
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);

            if let Some((hash,)) = admin_row {
                // println!("  Admin Password Hash: {}", &hash);
                // println!(
                //     "  Verify Against Admin Hash: {}",
                //     security::verify_password(&payload.password, &hash)
                // );
                if security::verify_password(&payload.password, &hash) {
                    let token = security::create_jwt("vendor", "all", None, &state.jwt_secret)?;
                    return Ok(build_success_response("vendor", "all", None, token));
                }
            }

            // B. 尝试通用的 Vendor 密码 (如果在 settings 表里配置了的话)
            let vendor_row: Option<(String,)> =
                sqlx::query_as("SELECT value FROM settings WHERE key = 'vendor_password'")
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);

            if let Some((hash,)) = vendor_row {
                // println!("  Vendor Password Hash: {}", &hash);
                // println!(
                //     "  Verify Against Vendor Hash: {}",
                //     security::verify_password(&payload.password, &hash)
                // );
                if security::verify_password(&payload.password, &hash) {
                    let token = security::create_jwt("vendor", "all", None, &state.jwt_secret)?;
                    return Ok(build_success_response("vendor", "all", None, token));
                }
            }

            // C. 尝试特定 Event 的密码
            if let Some(eid) = event_id {
                // [修复] 验证 event 是否存在 ✓
                let event_exists: Option<(i64,)> =
                    sqlx::query_as("SELECT id FROM events WHERE id = ?")
                        .bind(eid)
                        .fetch_optional(&state.db)
                        .await
                        .unwrap_or(None);

                if event_exists.is_none() {
                    // Event 不存在，返回 404
                    return Err(AuthError::WrongCredentials);
                }

                let event_row: Option<(Option<String>,)> =
                    sqlx::query_as("SELECT vendor_password FROM events WHERE id = ?")
                        .bind(eid)
                        .fetch_optional(&state.db)
                        .await
                        .unwrap_or(None);

                // 注意：vendor_password 在数据库里是 nullable 的
                if let Some((Some(event_pass_hash),)) = event_row {
                    if security::verify_password(&payload.password, &event_pass_hash) {
                        let token =
                            security::create_jwt("vendor", "event", Some(eid), &state.jwt_secret)?;
                        return Ok(build_success_response("vendor", "event", Some(eid), token));
                    }
                }
            }
        }

        _ => return Err(AuthError::WrongCredentials),
    }

    Err(AuthError::WrongCredentials)
}

// 辅助函数：构建包含 Cookie 和 JSON Body 的响应
fn build_success_response(
    role: &str,
    access: &str,
    event_id: Option<i64>,
    token: String,
) -> Response {
    let body = LoginResponse {
        message: "Login successful".into(),
        role: role.into(),
        access: access.into(),
        event_id,
        token: token.clone(),
    };

    // [修改策略]
    // 1. Path=/: 全局有效
    // 2. SameSite=Lax: 现代浏览器默认值。
    //    虽然在跨域 POST 时不会自动带上，但对 GET 导航有效，留着作为兜底。
    // 3. 移除 Secure: 因为你是局域网 HTTP，加上 Secure 浏览器反而会拒收 Cookie。
    // 4. HttpOnly: 防止 JS 读取，安全。
    let cookie_str = format!(
        "access_token_cookie={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=86400",
        token
    );
    // 注意：上面去掉了 "; Secure"

    let mut response = Json(body).into_response();
    let cookie_val = match HeaderValue::from_str(&cookie_str) {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid cookie value").into_response()
        }
    };
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie_val,
    );

    response
}

// 退出登录：清除 HttpOnly Cookie
async fn logout_handler() -> Response {
    let cookie_str = "access_token_cookie=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0";
    let mut response = Json(serde_json::json!({"message": "Logged out"})).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, HeaderValue::from_static(cookie_str));
    response
}

// 检查管理员密码是否仍为默认值 admin123
async fn is_default_admin_password(
    State(state): State<AppState>,
) -> Result<Json<IsDefaultAdminPasswordResponse>, AuthError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'admin_password'")
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    if let Some((hash,)) = row {
        let is_default = security::verify_password("admin123", &hash);
        Ok(Json(IsDefaultAdminPasswordResponse { is_default }))
    } else {
        // 未设置密码时返回 false，避免误报
        Ok(Json(IsDefaultAdminPasswordResponse { is_default: false }))
    }
}
