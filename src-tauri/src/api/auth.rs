use axum::body::Bytes;
use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::openapi::ApiErrorBody,
    error::ApiResult,
    state::AppState,
    utils::security::{self, AuthError},
};

// 自定义反序列化函数：接受字符串、数字或 null 的 i64
fn deserialize_i64_from_str<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Deserialize};

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
                // 4. 解析失败时只记录长度和错误，不打印明文 Body —— 这里包含登录密码
                eprintln!("========================================");
                eprintln!("[DEBUG] JSON Parsing FAILED!");
                eprintln!("[DEBUG] Error: {}", e);
                eprintln!("[DEBUG] Body length: {} bytes", bytes.len());
                eprintln!("========================================");

                // 返回 400 Bad Request 给前端，而不是 422
                Err((StatusCode::BAD_REQUEST, format!("JSON Parse Error: {}", e)).into_response())
            }
        }
    }
}

// 1. 请求体 DTO
#[derive(Deserialize, ToSchema)]
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
#[derive(Serialize, ToSchema)]
struct LoginResponse {
    message: String,
    role: String,
    access: String,
    #[serde(rename = "eventId", skip_serializing_if = "Option::is_none")]
    event_id: Option<i64>,
    token: String, // 我们把 token 直接放在 Body 里方便前端拿
}

#[derive(Serialize, ToSchema)]
struct IsDefaultAdminPasswordResponse {
    is_default: bool,
}

/// 退出登录的成功体。形状和登录响应里的 `message` 一样，单独建类型只是为了文档。
#[derive(Serialize, ToSchema)]
struct LogoutResponse {
    message: String,
}

// 3. 路由定义
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login_handler))
        .routes(routes!(logout_handler))
        .routes(routes!(is_default_admin_password))
}

// 4. 处理器逻辑
/// 登录：管理员校验全局密码，摊主可校验全局密码或展会专属密码。
///
/// 成功时在 Body 里回 token，同时下发 HttpOnly 的 `access_token_cookie`。
/// `eventId` 只在摊主用「展会专属密码」登录成功时回填。
#[utoipa::path(
    post,
    path = "/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, body = LoginResponse, description = "登录成功；同时 Set-Cookie 下发 access_token_cookie"),
        (status = 400, body = String, description = "请求体不是合法 JSON：纯文本 `JSON Parse Error: …`"),
        (status = 401, body = ApiErrorBody, description = "角色或密码错误"),
        (status = 500, body = ApiErrorBody, description = "令牌创建失败"),
    ),
)]
async fn login_handler(
    State(state): State<AppState>,
    DebugJson(payload): DebugJson<LoginRequest>,
) -> Result<Response, AuthError> {
    // 不需要展会守卫：登录不属于任何展会
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
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie_val);

    response
}

/// 退出登录：清掉 HttpOnly 的 `access_token_cookie`。
#[utoipa::path(
    post,
    path = "/logout",
    tag = "auth",
    responses(
        (status = 200, body = LogoutResponse, description = "已退出；同时 Set-Cookie 清除 access_token_cookie"),
    ),
)]
async fn logout_handler() -> Response {
    // 不需要展会守卫：登出只清 cookie，不碰任何展会的账
    let cookie_str = "access_token_cookie=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0";
    let mut response = Json(LogoutResponse {
        message: "Logged out".into(),
    })
    .into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, HeaderValue::from_static(cookie_str));
    response
}

/// 管理员密码是否仍是出厂默认值 `admin123`（登录页据此提示改密码）。
#[utoipa::path(
    get,
    path = "/is-default-admin-password",
    tag = "auth",
    responses(
        (status = 200, body = IsDefaultAdminPasswordResponse, description = "未设置密码时也返回 false"),
    ),
)]
async fn is_default_admin_password(
    State(state): State<AppState>,
) -> ApiResult<Json<IsDefaultAdminPasswordResponse>> {
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

#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        json_request, read_json, seed_event_and_product, shape_of, test_router_with,
    };
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    /// 默认管理员密码是 admin123；再给展会挂一个专属摊主密码，
    /// 让 `vendor` + `event` 登录成功时响应里的 `eventId` 出现一次非空。
    async fn seeded() -> (Router, tempfile::TempDir, i64) {
        let (router, dir, pool) = test_router_with().await;
        let (event_id, _ep_a, _ep_b) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET vendor_password = ? WHERE id = ?")
            .bind(crate::utils::security::hash_password("eventpw"))
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        (router, dir, event_id)
    }

    async fn call(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(json_request(method, uri, token, body))
            .await
            .unwrap();
        let status = res.status();
        (status, read_json(res).await)
    }

    #[tokio::test]
    async fn shape_login_handler() {
        let (router, _dir, event_id) = seeded().await;

        // 管理员登录：`eventId` 字段整个缺省（不是 null）。
        let (s, body) = call(
            &router,
            "POST",
            "/api/auth/login",
            None,
            json!({"role": "admin", "password": "admin123"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "message": "string", "role": "string", "access": "string", "token": "string"
            })
        );

        // 展会摊主登录：access=event，eventId 非空。
        // eventId 故意用字符串——前端 route.query.eventId 传过来的就是字符串。
        let (s, body) = call(
            &router,
            "POST",
            "/api/auth/login",
            None,
            json!({"role": "vendor", "password": "eventpw", "eventId": event_id.to_string()}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "message": "string", "role": "string", "access": "string",
                "eventId": "int", "token": "string"
            })
        );

        // 密码错误：401 + `{"error":"密码错误"}`。
        let (s, body) = call(
            &router,
            "POST",
            "/api/auth/login",
            None,
            json!({"role": "admin", "password": "wrong"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );
        // 前端 authStore 读的就是 `error.response.data.error`，文字一并钉死。
        assert_eq!(body, json!({"error": "密码错误"}));

        // DebugJson 解析失败：400 纯文本 `JSON Parse Error: …`，不是 JSON。
        let req = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from("{not json"))
            .unwrap();
        let res = router.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "text/plain; charset=utf-8"
        );
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.starts_with("JSON Parse Error: "), "got: {text}");
    }

    #[tokio::test]
    async fn shape_logout_handler() {
        let (router, _dir, _event_id) = seeded().await;
        let (s, body) = call(&router, "POST", "/api/auth/logout", None, json!(null)).await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::OK, json!({"message": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_is_default_admin_password() {
        let (router, _dir, _event_id) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            "/api/auth/is-default-admin-password",
            None,
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::OK, json!({"is_default": "bool"}))
        );
    }
}
