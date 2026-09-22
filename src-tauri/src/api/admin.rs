// src/api/admin.rs

use crate::{
    api::guard::AdminOnly,
    state::AppState,
    utils::security::{hash_password, verify_password},
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::put,
    Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::AssertSqlSafe;
use tokio::fs;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/password", put(update_admin_password))
        .route(
            "/vendor-default-password",
            put(update_vendor_default_password),
        )
        .route("/reset-database", put(reset_database_handler))
}

// ==========================================
// 1. 更新管理员密码
// ==========================================
#[derive(Deserialize)]
struct UpdateAdminPasswordRequest {
    #[serde(rename = "oldPassword")]
    old_password: String,
    #[serde(rename = "newPassword")]
    new_password: String,
}

async fn update_admin_password(
    State(state): State<AppState>,
    _: AdminOnly,
    Json(payload): Json<UpdateAdminPasswordRequest>,
) -> impl IntoResponse {
    if payload.new_password.chars().count() < 4 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "新密码至少 4 位"})),
        )
            .into_response();
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
        // eprintln!(
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
    //eprintln!("[DEBUG] Updating admin password. New hash: {}", new_hash);
    let result = sqlx::query("UPDATE settings SET value = ? WHERE key = 'admin_password'")
        .bind(new_hash)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            //eprintln!("[DEBUG] Admin password updated successfully");
            (StatusCode::OK, Json(json!({"message": "管理员密码已更新"}))).into_response()
        }
        Err(_e) => {
            //eprintln!("[DEBUG] Failed to update admin password: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 2. 更新默认摊主密码
// ==========================================
#[derive(Deserialize)]
struct UpdateVendorPasswordRequest {
    #[serde(rename = "newPassword")]
    new_password: String,
}

async fn update_vendor_default_password(
    State(state): State<AppState>,
    _: AdminOnly,
    Json(payload): Json<UpdateVendorPasswordRequest>,
) -> impl IntoResponse {
    if payload.new_password.chars().count() < 4 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "新密码至少 4 位"})),
        )
            .into_response();
    }

    let new_hash = hash_password(&payload.new_password);
    // eprintln!(
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
            //eprintln!("[DEBUG] Global vendor password updated successfully");
            (
                StatusCode::OK,
                Json(json!({"message": "默认摊主密码已更新"})),
            )
                .into_response()
        }
        Err(_e) => {
            //eprintln!("[DEBUG] Failed to update vendor password: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 3. 重置数据库（危险操作）
// ==========================================
async fn reset_database_handler(State(state): State<AppState>, _: AdminOnly) -> impl IntoResponse {
    //eprintln!("[WARNING] Database reset requested by admin");

    // --------------------------------------------------------
    // 第一步：删除物理文件（图片资源）
    // --------------------------------------------------------
    let uploads_dir = &state.upload_dir;
    if uploads_dir.exists() {
        //eprintln!("[INFO] Cleaning uploads directory: {:?}", uploads_dir);
        // 尝试删除目录
        match fs::remove_dir_all(uploads_dir).await {
            Ok(_) => {
                // 重新创建空目录
                if let Err(_e) = fs::create_dir_all(uploads_dir).await {
                    //eprintln!("[ERROR] Failed to recreate uploads directory: {:?}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "无法重创建上传目录"})),
                    )
                        .into_response();
                }
            }
            Err(_e) => {
                //eprintln!("[ERROR] Failed to delete uploads directory: {:?}", e);
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
            //eprintln!("[ERROR] Failed to start transaction: {:?}", e);
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
            //eprintln!("[ERROR] Failed to clear table {}: {:?}", table, e);
            let _ = tx.rollback().await;
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to clear data").into_response();
        }
        //eprintln!("[INFO] Cleared table: {}", table);
    }

    // 重置自增ID（可选，让下次插入数据从1开始）
    if let Err(_e) = sqlx::query("DELETE FROM sqlite_sequence")
        .execute(&mut *tx)
        .await
    {
        //eprintln!("[WARNING] Failed to reset auto-increment IDs: {:?}", e);
        // 这不是致命错误，继续执行
    }

    // 特殊处理 settings 表：重置为默认密码
    // 删除所有设置，然后重新插入默认密码
    if let Err(_e) = sqlx::query("DELETE FROM settings").execute(&mut *tx).await {
        //eprintln!("[ERROR] Failed to clear settings: {:?}", e);
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
        //eprintln!("[ERROR] Failed to reset passwords: {:?}", e);
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to reset passwords",
        )
            .into_response();
    }

    //eprintln!("[INFO] Reset passwords to default (admin123 / vendor123)");

    // 提交事务
    match tx.commit().await {
        Ok(_) => {
            //eprintln!("[INFO] Database reset successful (Data cleared)");

            // 可选：执行 VACUUM 释放磁盘空间（不能在事务中执行）
            // sqlx::query("VACUUM").execute(&state.db).await.ok();

            (
                StatusCode::OK,
                Json(json!({
                    "message": "数据库已完全重置，所有图片已删除，密码已恢复为 admin123 / vendor123",
                    "warning": "所有数据和文件已被清空，请重新登录"
                })),
            ).into_response()
        }
        Err(_e) => {
            //eprintln!("[ERROR] Failed to commit transaction: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "数据库重置事务提交失败"})),
            )
                .into_response()
        }
    }
}
