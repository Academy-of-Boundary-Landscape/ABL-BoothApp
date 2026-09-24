use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};

const EVENT_UPLOAD_LIMIT_BYTES: usize = 10 * 1024 * 1024;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query, query_as, query_scalar};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{guard::AdminOnly, openapi::ApiErrorBody},
    db::models::Event,
    error::{ApiError, ApiResult},
    state::AppState,
    utils::{
        file::{delete_file, save_upload_file},
        security::hash_password,
    },
};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_events, create_event))
        .routes(routes!(get_event, update_event, delete_event))
        .routes(routes!(update_status))
        .layer(DefaultBodyLimit::max(EVENT_UPLOAD_LIMIT_BYTES))
}

// ==========================================
// DTOs (Data Transfer Objects)
// ==========================================

// 1. 用于查询参数解析
#[derive(Deserialize, IntoParams)]
struct ListEventsQuery {
    /// 只列这个状态的；不传则全部。
    #[param(value_type = Option<crate::api::openapi::EventStatus>)]
    status: Option<String>,
}

// 2. 用于 API 响应的结构体 (解决 qrcode_url 问题，避免 flatten 导致的序列化问题)
#[derive(Serialize, ToSchema)]
struct EventResponse {
    pub id: i64,
    pub name: String,
    #[serde(rename = "date")]
    pub event_date: String,
    pub location: Option<String>,
    #[schema(value_type = crate::api::openapi::EventStatus)]
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
    let Some(raw) = raw.as_deref() else {
        return vec![];
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return vec![];
    }
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
    let Some(raw) = raw.as_deref() else {
        return vec![];
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return vec![];
    }

    let paths: Vec<String> = if raw.starts_with('[') {
        serde_json::from_str(raw).unwrap_or_else(|_| vec![raw.to_string()])
    } else {
        vec![raw.to_string()]
    };

    paths
        .into_iter()
        .filter(|p| !p.is_empty())
        .map(|p| {
            if p.starts_with("/uploads/") {
                p
            } else {
                format!("/uploads/{}", p)
            }
        })
        .collect()
}

/// 仅用于 OpenAPI 文档：创建展会的 multipart 表单字段。
#[derive(ToSchema)]
#[allow(dead_code)]
struct CreateEventForm {
    name: String,
    date: String,
    location: Option<String>,
    vendor_password: Option<String>,
    #[schema(value_type = Option<String>, format = Binary)]
    payment_qr_code_wechat: Option<Vec<u8>>,
    #[schema(value_type = Option<String>, format = Binary)]
    payment_qr_code_alipay: Option<Vec<u8>>,
}

/// 仅用于 OpenAPI 文档：更新展会的 multipart 表单字段。
#[derive(ToSchema)]
#[allow(dead_code)]
struct UpdateEventForm {
    name: Option<String>,
    date: Option<String>,
    location: Option<String>,
    vendor_password: Option<String>,
    remove_payment_qr_code: Option<bool>,
    #[schema(value_type = Option<String>, format = Binary)]
    payment_qr_code_wechat: Option<Vec<u8>>,
    #[schema(value_type = Option<String>, format = Binary)]
    payment_qr_code_alipay: Option<Vec<u8>>,
}

/// `DELETE /events/{id}` 的成功响应体。
#[derive(Serialize, ToSchema)]
struct EventDeletedResponse {
    message: String,
}

// ==========================================
// 1. 获取漫展列表 (Public) [已修复过滤]
// ==========================================
/// 展会列表，可选按 `status` 过滤。公开接口。
#[utoipa::path(
    get,
    path = "/",
    tag = "event",
    params(ListEventsQuery),
    responses(
        (status = 200, body = Vec<EventResponse>, description = "按 event_date 倒序"),
    ),
)]
async fn list_events(
    State(state): State<AppState>,
    Query(params): Query<ListEventsQuery>, // [修复] 接收 Query 参数
) -> ApiResult<Json<Vec<EventResponse>>> {
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

    Ok(Json(response))
}

// ==========================================
// 2. 获取单个漫展 (Public) [已修复响应]
// ==========================================
/// 单个展会详情。公开接口。
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "event",
    params(("id" = i64, Path, description = "展会 id")),
    responses(
        (status = 200, body = EventResponse),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<EventResponse>> {
    let event: Option<Event> = query_as::<_, Event>("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    match event {
        // [修复] 转换响应结构
        Some(e) => Ok(Json(EventResponse::from_model(e))),
        None => Err(ApiError::NotFound("Event not found".into())),
    }
}

// ==========================================
// 3. 创建漫展 (Admin Only - Multipart)
// ==========================================
/// 创建展会（multipart 表单，可传多张收款码）。需要管理员。
#[utoipa::path(
    post,
    path = "/",
    tag = "event",
    request_body(content = CreateEventForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 201, body = EventResponse),
        (status = 400, body = ApiErrorBody, description = "缺少 name 或 date"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, body = ApiErrorBody, description = "保存上传文件或写库失败"),
    ),
)]
async fn create_event(
    State(state): State<AppState>,
    _: AdminOnly,
    mut multipart: Multipart,
) -> Response {
    // 不需要展会守卫：展会本身还不存在，没有可守卫的状态
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
                    log::warn!("Upload Failed: {}", e);
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
                "vendor_password" if !value.is_empty() => {
                    vendor_password = Some(hash_password(&value));
                }
                _ => {}
            }
        }
    }

    if name.is_empty() || date.is_empty() {
        return ApiError::BadRequest("Name and Date are required".into()).into_response();
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
        VALUES (?, ?, ?, ?, ?, '筹备')
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
            log::warn!("DB Error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 4. 更新漫展 (Admin Only - Multipart)
// ==========================================
/// 更新展会（multipart 表单；POST 与 PUT 等价）。需要管理员。
#[utoipa::path(
    method(post, put),
    path = "/{id}",
    tag = "event",
    params(("id" = i64, Path, description = "展会 id")),
    request_body(content = UpdateEventForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 200, body = EventResponse),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, body = ApiErrorBody, description = "展会不存在（现状为纯文本响应）"),
        (status = 500, body = ApiErrorBody, description = "上传或写库失败"),
    ),
)]
async fn update_event(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> Response {
    // 不需要展会守卫：改的是展会的展示与访问属性（名称、日期、地点、摊主密码、
    // 收款二维码），不写 journals / stock_movements / money_movements / order_lines /
    // event_products，动不了账，展会冻没冻结都不影响这些字段。
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
                    log::warn!("Update Upload Failed: {}", e);
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
                "remove_payment_qr_code" if value == "true" => {
                    should_remove_qr = true;
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
            log::warn!("Update DB Error: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
        }
    }
}

// ==========================================
// 5. 更新状态 (Admin Only - JSON) [已修复状态验证]
// ==========================================
#[derive(Deserialize, ToSchema)]
#[schema(as = EventUpdateStatusRequest)]
struct UpdateStatusRequest {
    #[schema(value_type = crate::api::openapi::EventStatus)]
    status: String,
}

/// 修改展会状态（筹备 / 进行中）。需要管理员。
///
/// 已结算的展会不能通过这里解冻，也不能直接改成已结算——必须走收摊流程。
#[utoipa::path(
    put,
    path = "/{id}/status",
    tag = "event",
    params(("id" = i64, Path, description = "展会 id")),
    request_body = UpdateStatusRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = EventResponse),
        (status = 400, body = ApiErrorBody, description = "状态值不合法，或试图直接改成已结算"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
        (status = 409, body = ApiErrorBody, description = "已结算的展会不能解冻"),
    ),
)]
async fn update_status(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Response {
    // 不需要展会守卫：本 handler 自己管状态迁移，两道检查见下方
    // [修复] 验证状态值只能是允许的值 ✓
    let target = payload.status.as_str();
    if !matches!(target, "筹备" | "进行中" | "已结算") {
        // 无效的状态值
        return ApiError::BadRequest("Invalid status. Must be one of: 筹备, 进行中, 已结算".into())
            .into_response();
    }

    // 冻结语义的第一道：只有 POST /closing/settle 能进已结算。
    // 这个口子开着的话，清 pending 和现场仓归零两道检查都能绕过去。
    if target == "已结算" {
        return ApiError::BadRequest("请走收摊流程结算（清 pending → 盘点 → 带回）".into())
            .into_response();
    }

    // 冻结语义的第二道：已结算的展会不能静默解冻。
    //
    // 检查折进写入本身，不留「读完到写」之间的窗口——否则一个
    // POST /closing/settle 可以在两步之间提交，把刚结算完的展会重新打开，
    // 而这正是这道守卫存在的理由。
    let updated: Option<Event> = match query_as(
        r#"
        UPDATE events
        SET status = ?
        WHERE id = ? AND status <> '已结算'
        RETURNING *
        "#,
    )
    .bind(&payload.status)
    .bind(id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(row) => row,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response(),
    };

    match updated {
        Some(event) => {
            let response = EventResponse::from_model(event);
            (StatusCode::OK, Json(response)).into_response()
        }
        // 没更新到行：要么展会不存在（404），要么它已经是已结算（409）。
        // 便宜的存在性探测把这两种情况分开。
        None => {
            let exists: Option<i64> = match query_scalar("SELECT id FROM events WHERE id = ?")
                .bind(id)
                .fetch_optional(&state.db)
                .await
            {
                Ok(row) => row,
                Err(_) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
                }
            };
            if exists.is_some() {
                ApiError::Conflict("已结算的展会不能解冻".into()).into_response()
            } else {
                ApiError::NotFound("Event not found".into()).into_response()
            }
        }
    }
}

// ==========================================
// 6. 删除漫展 (Admin Only) [已修复级联删除]
// ==========================================
/// 删除展会，连同它的账一起级联删除。需要管理员。
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "event",
    params(("id" = i64, Path, description = "展会 id")),
    security(("bearer" = [])),
    responses(
        (status = 200, body = EventDeletedResponse, description = "删除成功"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
        (status = 500, body = ApiErrorBody, description = "数据库错误"),
    ),
)]
async fn delete_event(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
) -> Response {
    // 不需要展会守卫：删的是展会本身（连同它的账一起级联删掉），不是结算后的追加写
    let event: Option<Event> = query_as("SELECT * FROM events WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let Some(e) = event else {
        return ApiError::NotFound("Event not found".into()).into_response();
    };

    // 使用事务确保级联删除的原子性
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            log::warn!("Failed to begin transaction: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
                .into_response();
        }
    };

    // 只删 events 这一行。新 schema 里 event_products / orders / journals / lots
    // 对 events 都是 ON DELETE CASCADE，order_lines / order_lots 又对各自父表
    // CASCADE，所以旧代码手工按 order_items -> orders -> products 逐表删子表已经
    // 不需要了。保留事务是为了「要么全删、要么全留」的原子性。
    if let Err(err) = query("DELETE FROM events WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
    {
        log::warn!("Failed to delete event {}: {:?}", id, err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete event"})),
        )
            .into_response();
    }

    // 提交事务 — 失败时所有删除都会回滚
    if let Err(err) = tx.commit().await {
        log::warn!(
            "Transaction commit failed for delete event {}: {:?}",
            id,
            err
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Transaction commit failed"})),
        )
            .into_response();
    }

    // 事务成功后才清理物理文件 (文件删除无法回滚，所以放在事务之后)
    for path in parse_raw_qr_paths(&e.payment_qr_code_path) {
        let _ = delete_file(&state.upload_dir, &path).await;
    }

    (
        StatusCode::OK,
        Json(EventDeletedResponse {
            message: "Event deleted".into(),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        admin_token, json_request, seed_event_and_product, test_router, test_router_with,
    };
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt; // for oneshot

    #[tokio::test]
    async fn list_events_returns_empty_array_on_fresh_db() {
        let (router, _dir) = test_router().await;

        let res = router
            .oneshot(
                Request::builder()
                    .uri("/api/events")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json.as_array().map(|a| a.len()), Some(0));
    }

    #[tokio::test]
    async fn admin_only_route_rejects_request_without_token() {
        // 证明 guard.rs 的 AdminOnly 提取器确实挂在链路上
        let (router, _dir) = test_router().await;

        let res = router
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/events/1/status")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"status":"active"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn cannot_settle_an_event_by_setting_its_status_directly() {
        // 整个 ②-3 的冻结语义建立在「只有 POST /closing/settle 能进已结算」之上。
        // 这个口子开着的话，清 pending 和现场仓归零两道检查都能绕过去。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/status"),
                Some(&admin_token()),
                json!({"status": "已结算"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let status: String = sqlx::query_scalar("SELECT status FROM events WHERE id = ?")
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "进行中");
    }

    #[tokio::test]
    async fn a_settled_event_cannot_be_thawed() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, _, _) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/status"),
                Some(&admin_token()),
                json!({"status": "进行中"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn updating_status_of_a_missing_event_is_404() {
        // 以前拿不到行会一路走到 fetch_one 然后报 500「Database Error」。
        let (router, _dir, _pool) = test_router_with().await;
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                "/api/events/9999/status",
                Some(&admin_token()),
                json!({"status": "进行中"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }
}

/// ③b 形状快照：钉住本模块每个路由×方法的 JSON 形状（键 + 类型），
/// 类型化前后必须一行不改照样绿。
#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, shape_of, test_router_with,
        vendor_token,
    };
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use serde_json::{json, Value};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    /// 一场有地点、有两张收款码的进行中展会，让 `EventResponse` 的每个可选字段
    /// 和数组至少出现一次非空。
    async fn seeded() -> (Router, tempfile::TempDir, SqlitePool, i64, i64, i64) {
        let (router, dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        sqlx::query("UPDATE events SET location = '上海', payment_qr_code_path = ? WHERE id = ?")
            .bind(r#"["events/a.jpg","events/b.jpg"]"#)
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        (router, dir, pool, event_id, ep_a, ep_b)
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

    /// 发一个手写的 multipart 请求并读回 JSON。
    async fn call_multipart(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str)],
        files: &[(&str, &str, &str)],
    ) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(multipart_request(method, uri, token, fields, files))
            .await
            .unwrap();
        let status = res.status();
        (status, read_json(res).await)
    }

    /// 发一个手写的 multipart 请求并读回原始文本（钉非 JSON 错误体用）。
    async fn call_multipart_raw(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str)],
    ) -> (StatusCode, String) {
        let res = router
            .clone()
            .oneshot(multipart_request(method, uri, token, fields, &[]))
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).to_string())
    }

    fn multipart_request(
        method: &str,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str)],
        files: &[(&str, &str, &str)],
    ) -> Request<Body> {
        let boundary = "X-BOUNDARY";
        let mut body = String::new();
        for (name, value) in fields {
            body.push_str(&format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            ));
        }
        for (name, filename, content) in files {
            body.push_str(&format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n{content}\r\n"
            ));
        }
        body.push_str(&format!("--{boundary}--\r\n"));

        let mut builder = Request::builder().method(method).uri(uri).header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        );
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        builder.body(Body::from(body)).unwrap()
    }

    fn event_shape() -> Value {
        json!({
            "id": "int",
            "name": "string",
            "date": "string",
            "location": "string",
            "status": "string",
            "qrcode_url": "string",
            "qrcode_urls": ["string"],
        })
    }

    #[tokio::test]
    async fn shape_list_events() {
        let (router, _dir, _pool, _, _, _) = seeded().await;
        let (s, body) = call(&router, "GET", "/api/events", None, json!(null)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!([event_shape()]));

        // status 过滤走另一条 SQL，形状不变
        let (s, body) = call(
            &router,
            "GET",
            "/api/events?status=进行中",
            None,
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!([event_shape()]));
    }

    #[tokio::test]
    async fn shape_get_event() {
        let (router, _dir, _pool, event_id, _, _) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            &format!("/api/events/{event_id}"),
            None,
            json!(null),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::OK, event_shape()));

        let (s, body) = call(&router, "GET", "/api/events/9999", None, json!(null)).await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::NOT_FOUND, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_create_event() {
        let (router, _dir, _pool, _, _, _) = seeded().await;
        let (s, body) = call_multipart(
            &router,
            "POST",
            "/api/events",
            Some(&admin_token()),
            &[
                ("name", "新展"),
                ("date", "2026-11-01"),
                ("location", "北京"),
                ("vendor_password", "pw"),
            ],
            &[("payment_qr_code_wechat", "wechat.png", "fake-bytes")],
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(shape_of(&body), event_shape());

        // 缺 name/date -> 400
        let (s, body) = call_multipart(
            &router,
            "POST",
            "/api/events",
            Some(&admin_token()),
            &[],
            &[],
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );

        // 无 token -> 401
        let (s, body) = call_multipart(&router, "POST", "/api/events", None, &[], &[]).await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_update_event() {
        let (router, _dir, _pool, event_id, _, _) = seeded().await;
        // 两个方法都挂在 /{id} 上，都要钉
        for method in ["POST", "PUT"] {
            let (s, body) = call_multipart(
                &router,
                method,
                &format!("/api/events/{event_id}"),
                Some(&admin_token()),
                &[
                    ("name", "改名"),
                    ("date", "2026-12-01"),
                    ("location", "广州"),
                ],
                &[],
            )
            .await;
            assert_eq!(s, StatusCode::OK, "{method}");
            assert_eq!(shape_of(&body), event_shape(), "{method}");
        }

        // 不存在的展会：现状是纯文本 404
        let (s, text) = call_multipart_raw(
            &router,
            "POST",
            "/api/events/9999",
            Some(&admin_token()),
            &[("name", "x"), ("date", "2026-12-01")],
        )
        .await;
        assert_eq!(
            (s, text.as_str()),
            (StatusCode::NOT_FOUND, "Event not found")
        );

        // 删除收款码：qrcode_url 变 null，qrcode_urls 变空数组
        let (s, body) = call_multipart(
            &router,
            "POST",
            &format!("/api/events/{event_id}"),
            Some(&admin_token()),
            &[
                ("name", "改名"),
                ("date", "2026-12-01"),
                ("remove_payment_qr_code", "true"),
            ],
            &[],
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        let mut empty_qr = event_shape();
        empty_qr["qrcode_url"] = json!("null");
        empty_qr["qrcode_urls"] = json!(["empty"]);
        assert_eq!(shape_of(&body), empty_qr);
    }

    #[tokio::test]
    async fn shape_update_status() {
        let (router, _dir, pool, event_id, _, _) = seeded().await;
        let t = admin_token();
        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/events/{event_id}/status"),
            Some(&t),
            json!({"status": "筹备"}),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::OK, event_shape()));

        // 非法状态值 -> 400
        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/events/{event_id}/status"),
            Some(&t),
            json!({"status": "active"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );

        // 不存在的展会 -> 404
        let (s, body) = call(
            &router,
            "PUT",
            "/api/events/9999/status",
            Some(&t),
            json!({"status": "进行中"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::NOT_FOUND, json!({"error": "string"}))
        );

        // 摊主 token -> 403
        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/events/{event_id}/status"),
            Some(&vendor_token(event_id)),
            json!({"status": "筹备"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::FORBIDDEN, json!({"error": "string"}))
        );

        // 已结算不能解冻 -> 409
        sqlx::query("UPDATE events SET status = '已结算' WHERE id = ?")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();
        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/events/{event_id}/status"),
            Some(&t),
            json!({"status": "进行中"}),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::CONFLICT, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_delete_event() {
        let (router, _dir, _pool, event_id, _, _) = seeded().await;
        let (s, body) = call(
            &router,
            "DELETE",
            &format!("/api/events/{event_id}"),
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::OK, json!({"message": "string"}))
        );

        let (s, body) = call(
            &router,
            "DELETE",
            "/api/events/9999",
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::NOT_FOUND, json!({"error": "string"}))
        );
    }
}
