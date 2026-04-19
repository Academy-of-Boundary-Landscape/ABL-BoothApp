use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};

const EVENT_UPLOAD_LIMIT_BYTES: usize = 10 * 1024 * 1024;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query, query_as};

use crate::{
    api::guard::AdminOnly,
    db::models::Event,
    state::AppState,
    utils::{
        file::{delete_file, save_upload_file},
        security::hash_password,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        // 公开接口
        .route("/", get(list_events))
        .route("/:id", get(get_event))
        // 管理员接口
        .route("/", post(create_event))
        .route("/:id", post(update_event).put(update_event))
        .route("/:id/status", put(update_status))
        .route("/:id", delete(delete_event))
        .layer(DefaultBodyLimit::max(EVENT_UPLOAD_LIMIT_BYTES))
}

// ==========================================
// DTOs (Data Transfer Objects)
// ==========================================

// 1. 用于查询参数解析
#[derive(Deserialize)]
struct ListEventsQuery {
    status: Option<String>,
}

// 2. 用于 API 响应的结构体 (解决 qrcode_url 问题，避免 flatten 导致的序列化问题)
#[derive(Serialize)]
struct EventResponse {
    pub id: i64,
    pub name: String,
    #[serde(rename = "date")]
    pub event_date: String,
    pub location: Option<String>,
    pub status: String,
    /// 向后兼容：保留单个 URL（取第一个），旧前端不会崩
    pub qrcode_url: Option<String>,
    /// 新字段：所有收款码 URL 数组
    pub qrcode_urls: Vec<String>,
}

impl EventResponse {
    fn from_model(event: Event) -> Self {
        let urls = parse_qr_paths(&event.payment_qr_code_path);

        Self {
            id: event.id,
            name: event.name,
            event_date: event.event_date,
            location: event.location,
            status: event.status,
            qrcode_url: urls.first().cloned(),
            qrcode_urls: urls,
        }
    }
}

/// 解析 payment_qr_code_path 原始值为路径列表（不加 /uploads/ 前缀，用于文件删除）
fn parse_raw_qr_paths(raw: &Option<String>) -> Vec<String> {
    let Some(raw) = raw.as_deref() else { return vec![] };
    let raw = raw.trim();
    if raw.is_empty() { return vec![]; }
    if raw.starts_with('[') {
        serde_json::from_str(raw).unwrap_or_else(|_| vec![raw.to_string()])
    } else {
        vec![raw.to_string()]
    }
}

/// 解析 payment_qr_code_path 字段：
/// - JSON 数组 `["events/a.jpg","events/b.jpg"]` → 多个 URL
/// - 纯字符串 `"events/a.jpg"` → 单个 URL（向后兼容旧数据）
fn parse_qr_paths(raw: &Option<String>) -> Vec<String> {
    let Some(raw) = raw.as_deref() else { return vec![] };
    let raw = raw.trim();
    if raw.is_empty() { return vec![]; }

    let paths: Vec<String> = if raw.starts_with('[') {
        serde_json::from_str(raw).unwrap_or_else(|_| vec![raw.to_string()])
    } else {
        vec![raw.to_string()]
    };

    paths.into_iter()
        .filter(|p| !p.is_empty())
        .map(|p| {
            if p.starts_with("/uploads/") { p } else { format!("/uploads/{}", p) }
        })
        .collect()
}

// ==========================================
// 1. 获取漫展列表 (Public) [已修复过滤]
// ==========================================
async fn list_events(
    State(state): State<AppState>,
    Query(params): Query<ListEventsQuery>, // [修复] 接收 Query 参数
) -> impl IntoResponse {
    // 根据是否传了 status 决定 SQL
    // 【关键】所有情况下都使用 unwrap_or_default() 确保返回空数组而不是 null
    let events: Vec<Event> = if let Some(status) = params.status {
        query_as::<_, Event>("SELECT * FROM events WHERE status = ? ORDER BY event_date DESC")
            .bind(status)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
    } else {
        query_as::<_, Event>("SELECT * FROM events ORDER BY event_date DESC")
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
    };

    // [修复] 转换为包含 qrcode_url 的 Response 对象
    // 即使 events 为空，也会返回 [] (空数组) 而不是 null
    let response: Vec<EventResponse> = events.into_iter().map(EventResponse::from_model).collect();

    Json(response)
}

// ==========================================
// 2. 获取单个漫展 (Public) [已修复响应]
// ==========================================
async fn get_event(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let event: Option<Event> = query_as::<_, Event>("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    match event {
        // [修复] 转换响应结构
        Some(e) => {
            let response = EventResponse::from_model(e);
            (StatusCode::OK, Json(response)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Event not found"})),
        )
            .into_response(),
    }
}

// ==========================================
// 3. 创建漫展 (Admin Only - Multipart)
// ==========================================
async fn create_event(
    State(state): State<AppState>,
    _: AdminOnly,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut name = String::new();
    let mut date = String::new();
    let mut location = String::new();
    let mut vendor_password = None;
    let mut qr_paths: Vec<String> = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        // 支持多个收款码：payment_qr_code / payment_qr_code_wechat / payment_qr_code_alipay
        if field_name.starts_with("payment_qr_code") {
            match save_upload_file(&state.upload_dir, field, Some("events")).await {
                Ok(path) => qr_paths.push(path),
                Err(e) => {
                    eprintln!("Upload Failed: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Failed to save file"})),
                    )
                        .into_response();
                }
            }
        } else {
            let value = field.text().await.unwrap_or_default();
            match field_name.as_str() {
                "name" => name = value,
                "date" => date = value,
                "location" => location = value,
                "vendor_password" => {
                    if !value.is_empty() {
                        vendor_password = Some(hash_password(&value));
                    }
                }
                _ => {}
            }
        }
    }

    if name.is_empty() || date.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Name and Date are required"})),
        )
            .into_response();
    }

    // 存储为 JSON 数组（多码）或 None（无码）
    let qr_code_path = if qr_paths.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&qr_paths).unwrap_or_default())
    };

    let result = query_as::<_, Event>(
        r#"
        INSERT INTO events (name, event_date, location, vendor_password, payment_qr_code_path, status)
        VALUES (?, ?, ?, ?, ?, '未进行')
        RETURNING *
        "#
    )
    .bind(&name)
    .bind(&date)
    .bind(if location.is_empty() { None } else { Some(&location) })
    .bind(&vendor_password)
    .bind(&qr_code_path)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(event) => {
            let response = EventResponse::from_model(event);
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            eprintln!("DB Error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 4. 更新漫展 (Admin Only - Multipart)
// ==========================================
async fn update_event(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let old_event: Event = match query_as("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Event not found").into_response(),
    };

    let mut name = old_event.name;
    let mut date = old_event.event_date;
    let mut location = old_event.location;
    let mut vendor_password_hash = old_event.vendor_password;
    let mut should_remove_qr = false;
    let mut new_qr_paths: Vec<String> = Vec::new();
    let mut has_new_qr_upload = false;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name.starts_with("payment_qr_code") && !field_name.starts_with("remove_") {
            match save_upload_file(&state.upload_dir, field, Some("events")).await {
                Ok(new_path) => {
                    new_qr_paths.push(new_path);
                    has_new_qr_upload = true;
                }
                Err(e) => {
                    eprintln!("Update Upload Failed: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "File upload failed")
                        .into_response();
                }
            }
        } else {
            let value = field.text().await.unwrap_or_default();
            match field_name.as_str() {
                "name" => name = value,
                "date" => date = value,
                "location" => location = if value.is_empty() { None } else { Some(value) },
                "vendor_password" => {
                    if !value.is_empty() {
                        vendor_password_hash = Some(hash_password(&value));
                    }
                }
                "remove_payment_qr_code" => {
                    if value == "true" {
                        should_remove_qr = true;
                    }
                }
                _ => {}
            }
        }
    }

    // 清理旧文件（删除所有旧码）
    let old_raw_paths = parse_raw_qr_paths(&old_event.payment_qr_code_path);
    if should_remove_qr || has_new_qr_upload {
        for old_path in &old_raw_paths {
            let _ = delete_file(&state.upload_dir, old_path).await;
        }
    }

    let qr_code_path = if should_remove_qr {
        None
    } else if has_new_qr_upload {
        Some(serde_json::to_string(&new_qr_paths).unwrap_or_default())
    } else {
        old_event.payment_qr_code_path
    };

    // [修复] 使用 RETURNING 子句原子地获取更新后的数据
    let result = query_as::<_, Event>(
        r#"
        UPDATE events 
        SET name = ?, event_date = ?, location = ?, vendor_password = ?, payment_qr_code_path = ?
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(date)
    .bind(location)
    .bind(vendor_password_hash)
    .bind(qr_code_path)
    .bind(id)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(event) => {
            let response = EventResponse::from_model(event);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            eprintln!("Update DB Error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 5. 更新状态 (Admin Only - JSON) [已修复状态验证]
// ==========================================
#[derive(Deserialize)]
struct UpdateStatusRequest {
    status: String,
}

async fn update_status(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    // [修复] 验证状态值只能是允许的值 ✓
    match payload.status.as_str() {
        "未进行" | "进行中" | "已结束" => {
            // 使用 RETURNING 子句原子地更新并获取完整数据
            let result = query_as::<_, Event>(
                r#"
                UPDATE events 
                SET status = ? 
                WHERE id = ?
                RETURNING *
                "#,
            )
            .bind(&payload.status)
            .bind(id)
            .fetch_one(&state.db)
            .await;

            match result {
                Ok(event) => {
                    let response = EventResponse::from_model(event);
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response(),
            }
        }
        _ => {
            // 无效的状态值
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid status. Must be one of: 未进行, 进行中, 已结束"})),
            )
                .into_response()
        }
    }
}

// ==========================================
// 6. 删除漫展 (Admin Only) [已修复级联删除]
// ==========================================
async fn delete_event(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let event: Option<Event> = query_as("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let Some(e) = event else {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "Event not found"}))).into_response();
    };

    // 使用事务确保级联删除的原子性
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("Failed to begin transaction: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))).into_response();
        }
    };

    // 按外键依赖顺序删除: order_items -> orders -> products -> events
    // 每一步都检查错误，失败则回滚
    if let Err(err) = sqlx::query(
        "DELETE FROM order_items WHERE order_id IN (SELECT id FROM orders WHERE event_id = ?)",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    {
        eprintln!("Failed to delete order_items for event {}: {:?}", id, err);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to delete order items"}))).into_response();
    }

    if let Err(err) = sqlx::query("DELETE FROM orders WHERE event_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
    {
        eprintln!("Failed to delete orders for event {}: {:?}", id, err);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to delete orders"}))).into_response();
    }

    if let Err(err) = sqlx::query("DELETE FROM products WHERE event_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
    {
        eprintln!("Failed to delete products for event {}: {:?}", id, err);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to delete products"}))).into_response();
    }

    if let Err(err) = query("DELETE FROM events WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
    {
        eprintln!("Failed to delete event {}: {:?}", id, err);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to delete event"}))).into_response();
    }

    // 提交事务 — 失败时所有删除都会回滚
    if let Err(err) = tx.commit().await {
        eprintln!("Transaction commit failed for delete event {}: {:?}", id, err);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Transaction commit failed"}))).into_response();
    }

    // 事务成功后才清理物理文件 (文件删除无法回滚，所以放在事务之后)
    for path in parse_raw_qr_paths(&e.payment_qr_code_path) {
        let _ = delete_file(&state.upload_dir, &path).await;
    }

    (StatusCode::OK, Json(json!({"message": "Event deleted"}))).into_response()
}
