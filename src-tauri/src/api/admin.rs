// src/api/admin.rs

use crate::{
    api::{guard::AdminOnly, openapi::ApiErrorBody},
    error::ApiError,
    state::AppState,
    utils::security::{hash_password, verify_password},
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::AssertSqlSafe;
use tokio::fs;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(default_passwords))
        .routes(routes!(update_admin_password))
        .routes(routes!(update_vendor_default_password))
        .routes(routes!(reset_database_handler))
}

// ==========================================
// 0. 出厂默认密码是否还在用
// ==========================================
/// 两个全局密码是否仍是出厂默认值。
#[derive(Serialize, ToSchema)]
struct DefaultPasswordsResponse {
    /// 管理员密码仍是 `admin123`
    admin: bool,
    /// 全局摊主密码仍是 `vendor123`（它能进所有展会）
    vendor: bool,
}

async fn setting_is(state: &AppState, key: &str, default_password: &str) -> Result<bool, ApiError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&state.db)
        .await?;
    Ok(row.is_some_and(|(hash,)| verify_password(default_password, &hash)))
}

/// 管理后台据此常驻提醒改密码。默认密码写在公开文档里，而 LAN 上的任何设备都能访问登录页，
/// 所以这里只**提醒**、不拦登录（摊位现场临时借设备登录是正常用法）。
#[utoipa::path(
    get,
    path = "/default-passwords",
    tag = "admin",
    security(("bearer" = [])),
    responses(
        (status = 200, body = DefaultPasswordsResponse),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
    ),
)]
async fn default_passwords(
    State(state): State<AppState>,
    _: AdminOnly,
) -> Result<Json<DefaultPasswordsResponse>, ApiError> {
    Ok(Json(DefaultPasswordsResponse {
        admin: setting_is(&state, "admin_password", "admin123").await?,
        vendor: setting_is(&state, "vendor_password", "vendor123").await?,
    }))
}

// ==========================================
// 1. 更新管理员密码
// ==========================================
#[derive(Deserialize, ToSchema)]
struct UpdateAdminPasswordRequest {
    #[serde(rename = "oldPassword")]
    old_password: String,
    #[serde(rename = "newPassword")]
    new_password: String,
}

/// 管理员密码 / 默认摊主密码更新成功的响应。
///
/// 两个接口共用；`#[schema(as = ...)]` 加模块前缀，避免和别的模块的同名 schema 在
/// openapi.json 里互相覆盖。
#[derive(Serialize, ToSchema)]
#[schema(as = AdminMessageResponse)]
struct MessageResponse {
    message: String,
}

/// 修改管理员登录密码。需要管理员。
#[utoipa::path(
    put,
    path = "/password",
    tag = "admin",
    request_body = UpdateAdminPasswordRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = MessageResponse, description = "管理员密码已更新"),
        (status = 400, body = ApiErrorBody, description = "新密码少于 4 位"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, body = ApiErrorBody, description = "settings 缺行或数据库写入失败（部分分支为纯文本）"),
    ),
)]
async fn update_admin_password(
    State(state): State<AppState>,
    _: AdminOnly,
    Json(payload): Json<UpdateAdminPasswordRequest>,
) -> Response {
    // 不需要展会守卫：管理员密码存在全局 settings 表，不属于任何展会
    if payload.new_password.chars().count() < 4 {
        return ApiError::BadRequest("新密码至少 4 位".into()).into_response();
    }

    // 1. 获取当前密码 Hash
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = 'admin_password'")
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    let stored_hash = match row {
        Some((h,)) => h,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Admin password not set in database",
            )
                .into_response()
        }
    };

    // 2. 验证旧密码
    if !verify_password(&payload.old_password, &stored_hash) {
        // log::warn!(
        //     "[DEBUG] Admin password verification failed. Stored hash: {}",
        //     stored_hash
        // );
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "旧密码错误"})),
        )
            .into_response();
    }

    // 3. 更新新密码
    let new_hash = hash_password(&payload.new_password);
    //log::warn!("[DEBUG] Updating admin password. New hash: {}", new_hash);
    let result = sqlx::query("UPDATE settings SET value = ? WHERE key = 'admin_password'")
        .bind(new_hash)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            //log::warn!("[DEBUG] Admin password updated successfully");
            (
                StatusCode::OK,
                Json(MessageResponse {
                    message: "管理员密码已更新".to_string(),
                }),
            )
                .into_response()
        }
        Err(_e) => {
            //log::warn!("[DEBUG] Failed to update admin password: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 2. 更新默认摊主密码
// ==========================================
#[derive(Deserialize, ToSchema)]
struct UpdateVendorPasswordRequest {
    #[serde(rename = "newPassword")]
    new_password: String,
}

/// 修改全局默认摊主密码。需要管理员。
#[utoipa::path(
    put,
    path = "/vendor-default-password",
    tag = "admin",
    request_body = UpdateVendorPasswordRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = MessageResponse, description = "默认摊主密码已更新"),
        (status = 400, body = ApiErrorBody, description = "新密码少于 4 位"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, body = ApiErrorBody, description = "数据库写入失败（返回纯文本，非 error 形状）"),
    ),
)]
async fn update_vendor_default_password(
    State(state): State<AppState>,
    _: AdminOnly,
    Json(payload): Json<UpdateVendorPasswordRequest>,
) -> Response {
    // 不需要展会守卫：摊主默认密码存在全局 settings 表，不属于任何展会
    if payload.new_password.chars().count() < 4 {
        return ApiError::BadRequest("新密码至少 4 位".into()).into_response();
    }

    let new_hash = hash_password(&payload.new_password);
    // log::warn!(
    //     "[DEBUG] Updating global vendor password. New hash: {}",
    //     new_hash
    // );

    // 使用 INSERT OR REPLACE 确保 key 存在
    let result =
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('vendor_password', ?)")
            .bind(new_hash)
            .execute(&state.db)
            .await;

    match result {
        Ok(_) => {
            //log::warn!("[DEBUG] Global vendor password updated successfully");
            (
                StatusCode::OK,
                Json(MessageResponse {
                    message: "默认摊主密码已更新".to_string(),
                }),
            )
                .into_response()
        }
        Err(_e) => {
            //log::warn!("[DEBUG] Failed to update vendor password: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 3. 重置数据库（危险操作）
// ==========================================

/// 数据库重置成功的响应。`warning` 给前端弹窗展示「数据已清空」。
#[derive(Serialize, ToSchema)]
#[schema(as = AdminResetDatabaseResponse)]
struct ResetDatabaseResponse {
    message: String,
    warning: String,
}

/// 清空全部业务数据、删除上传的图片并把密码恢复为默认值（危险操作）。需要管理员。
#[utoipa::path(
    put,
    path = "/reset-database",
    tag = "admin",
    security(("bearer" = [])),
    responses(
        (status = 200, body = ResetDatabaseResponse, description = "数据库已完全重置"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, body = ApiErrorBody, description = "清理上传目录或数据库事务失败（部分分支为纯文本）"),
    ),
)]
async fn reset_database_handler(State(state): State<AppState>, _: AdminOnly) -> Response {
    // 不需要展会守卫：管理员全局重置，不属于任何展会，且本来就要清空全部展会
    //log::warn!("[WARNING] Database reset requested by admin");

    // --------------------------------------------------------
    // 第一步：删除物理文件（图片资源）
    // --------------------------------------------------------
    let uploads_dir = &state.upload_dir;
    if uploads_dir.exists() {
        //log::warn!("[INFO] Cleaning uploads directory: {:?}", uploads_dir);
        // 尝试删除目录
        match fs::remove_dir_all(uploads_dir).await {
            Ok(_) => {
                // 重新创建空目录
                if let Err(_e) = fs::create_dir_all(uploads_dir).await {
                    //log::warn!("[ERROR] Failed to recreate uploads directory: {:?}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "无法重创建上传目录"})),
                    )
                        .into_response();
                }
            }
            Err(_e) => {
                //log::warn!("[ERROR] Failed to delete uploads directory: {:?}", e);
                // 这里可以选择报错返回，或者仅仅打印日志继续清除数据库
            }
        }
    }

    // --------------------------------------------------------
    // 第二步：清空数据库表（保留连接池，使用事务）
    // --------------------------------------------------------

    // 开启一个事务
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(_e) => {
            //log::warn!("[ERROR] Failed to start transaction: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response();
        }
    };

    // 【关键】：按顺序清空表（需要考虑外键约束）
    // 先删除子表，再删除父表，避免外键冲突
    // 注意：不删除 settings 表中的管理员密码

    // 按外键依赖顺序清空（子表在前）。
    // ⚠️ 加新表时必须同步这里 —— 漏一张表会让「重置」留下孤儿数据，
    // 而 reset 是用户在「数据乱了」时的最后一根稻草。
    let tables_to_clear = vec![
        "stock_movements",
        "money_movements",
        "refunds",
        "order_lines",
        "order_lots",
        "advances",
        "settlement_adjustments",
        "journals",
        "orders",
        "lot_candidates",
        "lots",
        "event_products",
        "events",
        "master_products",
        // societies 不清：本社团那一行是迁移种进去的，清掉之后
        // owner_society_id 全部悬空，而且没有任何界面能把它建回来。
    ];

    for table in &tables_to_clear {
        let query = format!("DELETE FROM {}", table);
        // 已审计：table 只可能取自上面写死的 tables_to_clear 字面量，不含任何用户输入。
        // 表名无法用 bind 参数化，只能拼接。
        if let Err(_e) = sqlx::query(AssertSqlSafe(query)).execute(&mut *tx).await {
            //log::warn!("[ERROR] Failed to clear table {}: {:?}", table, e);
            let _ = tx.rollback().await;
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to clear data").into_response();
        }
        //log::warn!("[INFO] Cleared table: {}", table);
    }

    // 重置自增ID（可选，让下次插入数据从1开始）
    if let Err(_e) = sqlx::query("DELETE FROM sqlite_sequence")
        .execute(&mut *tx)
        .await
    {
        //log::warn!("[WARNING] Failed to reset auto-increment IDs: {:?}", e);
        // 这不是致命错误，继续执行
    }

    // 特殊处理 settings 表：重置为默认密码
    // 删除所有设置，然后重新插入默认密码
    if let Err(_e) = sqlx::query("DELETE FROM settings").execute(&mut *tx).await {
        //log::warn!("[ERROR] Failed to clear settings: {:?}", e);
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to reset settings",
        )
            .into_response();
    }

    // 重新初始化默认密码（admin123 和 vendor123）
    use crate::utils::security::hash_password;

    let admin_hash = hash_password("admin123");
    let vendor_hash = hash_password("vendor123");

    if let Err(_e) = sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('admin_password', ?), ('vendor_password', ?)",
    )
    .bind(&admin_hash)
    .bind(&vendor_hash)
    .execute(&mut *tx)
    .await
    {
        //log::warn!("[ERROR] Failed to reset passwords: {:?}", e);
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to reset passwords",
        )
            .into_response();
    }

    //log::warn!("[INFO] Reset passwords to default (admin123 / vendor123)");

    // 提交事务
    match tx.commit().await {
        Ok(_) => {
            //log::warn!("[INFO] Database reset successful (Data cleared)");

            // 可选：执行 VACUUM 释放磁盘空间（不能在事务中执行）
            // sqlx::query("VACUUM").execute(&state.db).await.ok();

            (
                StatusCode::OK,
                Json(ResetDatabaseResponse {
                    message: "数据库已完全重置，所有图片已删除，密码已恢复为 admin123 / vendor123"
                        .to_string(),
                    warning: "所有数据和文件已被清空，请重新登录".to_string(),
                }),
            )
                .into_response()
        }
        Err(_e) => {
            //log::warn!("[ERROR] Failed to commit transaction: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "数据库重置事务提交失败"})),
            )
                .into_response()
        }
    }
}

/// ③b 形状快照：钉住每个路由的 JSON 形状（键 + 类型），类型化前后必须一行不改照样绿。
#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, shape_of, test_router_with,
        vendor_token,
    };
    use axum::http::StatusCode;
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    /// 一场有商品、有展会的库，让管理接口跑在真实迁移后的 schema 上。
    async fn seeded() -> (Router, tempfile::TempDir) {
        let (router, dir, pool) = test_router_with().await;
        let _ = seed_event_and_product(&pool).await;
        (router, dir)
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
    async fn default_passwords_reports_each_password_and_is_admin_only() {
        let (router, _dir) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "GET",
            "/api/admin/default-passwords",
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{body}");
        assert_eq!(body, json!({"admin": true, "vendor": true}));

        let (s, _) = call(
            &router,
            "PUT",
            "/api/admin/vendor-default-password",
            Some(&t),
            json!({"newPassword": "stall-2026"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let (_, body) = call(
            &router,
            "GET",
            "/api/admin/default-passwords",
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(body, json!({"admin": true, "vendor": false}));

        let (s, _) = call(
            &router,
            "GET",
            "/api/admin/default-passwords",
            Some(&vendor_token(1)),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        let (s, _) = call(
            &router,
            "GET",
            "/api/admin/default-passwords",
            None,
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn shape_update_admin_password() {
        let (router, _dir) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/password",
            Some(&t),
            json!({"oldPassword": "admin123", "newPassword": "newpass"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!({"message": "string"}));

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/password",
            Some(&t),
            json!({"oldPassword": "admin123", "newPassword": "x"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/password",
            None,
            json!({"oldPassword": "admin123", "newPassword": "newpass"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/password",
            Some(&vendor_token(1)),
            json!({"oldPassword": "admin123", "newPassword": "newpass"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::FORBIDDEN, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_update_vendor_default_password() {
        let (router, _dir) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/vendor-default-password",
            Some(&t),
            json!({"newPassword": "newvendor"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!({"message": "string"}));

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/vendor-default-password",
            Some(&t),
            json!({"newPassword": "x"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/vendor-default-password",
            None,
            json!({"newPassword": "newvendor"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/vendor-default-password",
            Some(&vendor_token(1)),
            json!({"newPassword": "newvendor"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::FORBIDDEN, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_reset_database_handler() {
        let (router, _dir) = seeded().await;

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/reset-database",
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({"message": "string", "warning": "string"})
        );

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/reset-database",
            None,
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );

        let (s, body) = call(
            &router,
            "PUT",
            "/api/admin/reset-database",
            Some(&vendor_token(1)),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::FORBIDDEN, json!({"error": "string"}))
        );
    }
}
