use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use tokio::time::{timeout, Duration};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::openapi::ApiErrorBody,
    error::{ApiError, ApiResult},
    state::AppState,
    vision::{index, store::VisionStore},
};

const VISION_UPLOAD_LIMIT_BYTES: usize = 10 * 1024 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_vision_status))
        .routes(routes!(search_by_image))
        .routes(routes!(rebuild_index))
        .routes(routes!(list_models))
        .routes(routes!(install_model))
        .routes(routes!(get_install_task))
        .routes(routes!(activate_model))
        .routes(routes!(delete_model))
        .routes(routes!(get_ep_setting, set_ep_setting))
        .layer(DefaultBodyLimit::max(VISION_UPLOAD_LIMIT_BYTES))
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionStatusResponse {
    model_id: String,
    model_version: String,
    index_version: i64,
    index_size: i64,
    last_rebuild_at: Option<String>,
    is_ready: bool,
    is_rebuilding: bool,
    reason: Option<String>,
    rebuild_processed: i64,
    rebuild_total: i64,
    execution_provider: String,
    /// 最近一次重建失败的原因（下一次成功后清空）
    last_rebuild_error: Option<String>,
    /// ONNX Runtime 加载失败的原因；有值时识别整体不可用，`is_ready` 恒为 false
    runtime_error: Option<String>,
}

/// Vision 运行时状态：模型、索引版本/大小、重建进度与当前推理设备。
#[utoipa::path(
    get,
    path = "/status",
    tag = "vision",
    responses((status = 200, body = VisionStatusResponse)),
)]
async fn get_vision_status(State(state): State<AppState>) -> Json<VisionStatusResponse> {
    let mut snapshot = state.vision_runtime.snapshot().await;
    let runtime_error = crate::vision::session::runtime_load_error();
    if runtime_error.is_some() {
        snapshot.is_ready = false;
        snapshot.reason = Some("VISION_RUNTIME_UNAVAILABLE".to_string());
    }

    Json(VisionStatusResponse {
        model_id: snapshot.model_id,
        model_version: snapshot.model_version,
        index_version: snapshot.index_version,
        index_size: snapshot.index_size,
        last_rebuild_at: snapshot.last_rebuild_at,
        is_ready: snapshot.is_ready,
        is_rebuilding: snapshot.is_rebuilding,
        reason: snapshot.reason,
        rebuild_processed: snapshot.rebuild_processed,
        rebuild_total: snapshot.rebuild_total,
        execution_provider: crate::vision::session::get_active_ep_name(),
        last_rebuild_error: snapshot.last_rebuild_error,
        runtime_error,
    })
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionSearchResult {
    master_product_id: i64,
    product_code: String,
    name: String,
    score: f32,
    thumb_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionSearchResponse {
    model_id: String,
    model_version: String,
    index_version: i64,
    is_uncertain: bool,
    results: Vec<VisionSearchResult>,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionModelItem {
    model_id: String,
    model_version: String,
    description: Option<String>,
    tier: Option<String>,
    dim: usize,
    input_size: usize,
    size_mb: Option<f64>,
    installed: bool,
    is_active: bool,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionModelsResponse {
    active_model_id: String,
    models: Vec<VisionModelItem>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(as = VisionActivateModelRequest)]
struct ActivateModelRequest {
    model_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(as = VisionInstallModelRequest)]
struct InstallModelRequest {
    model_id: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(as = VisionRebuildRequest)]
struct RebuildRequest {
    force_full: Option<bool>,
}

#[derive(Debug)]
struct SearchRequestContext {
    image_bytes: Vec<u8>,
    top_k: i64,
    mode: Option<String>,
    master_product_ids: Option<Vec<i64>>,
    event_id: Option<i64>,
    // 已用 validate_roi 校验格式，但裁剪逻辑还没接进实际的向量检索里；roi 功能上线前留着。
    #[allow(dead_code)]
    roi: Option<String>,
}

/// 仅用于 OpenAPI 文档：`/search` 的 multipart 表单字段。
#[derive(ToSchema)]
#[allow(dead_code)]
struct VisionSearchForm {
    /// 查询图片，必填。
    #[schema(value_type = String, format = Binary)]
    image: Vec<u8>,
    /// 返回前 K 个结果，1..=20，缺省 5。
    top_k: Option<String>,
    /// `order` | `admin_event` | `admin_master`。
    mode: Option<String>,
    /// mode 为 order/admin_event 时必填。
    event_id: Option<String>,
    /// JSON 数组或逗号分隔的商品 id。
    master_product_ids: Option<String>,
    /// ROI JSON：`{"x":..,"y":..,"w":..,"h":..}`。
    roi: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionRebuildResponse {
    ok: bool,
    message: String,
    force_full: bool,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionInstallModelResponse {
    ok: bool,
    task_id: String,
    model_id: String,
    source: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionInstallTaskResponse {
    task_id: String,
    model_id: String,
    status: String,
    progress: i32,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionActivateModelResponse {
    ok: bool,
    active_model_id: String,
    rebuild_started: bool,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionDeleteModelResponse {
    ok: bool,
    deleted_model_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionGpuDevice {
    device_id: i32,
    name: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionEpSettingResponse {
    configured: String,
    active: String,
    platform: String,
    gpu_devices: Vec<VisionGpuDevice>,
}

#[derive(Debug, Serialize, ToSchema)]
struct VisionSetEpResponse {
    ok: bool,
    execution_provider: String,
    active: String,
    message: String,
}

fn validate_mode(mode: &Option<String>, event_id: Option<i64>) -> Result<(), (StatusCode, String)> {
    match mode.as_deref() {
        Some("order") | Some("admin_master") | Some("admin_event") | None => {}
        Some(other) => return Err((StatusCode::BAD_REQUEST, format!("invalid mode: {}", other))),
    }

    if matches!(mode.as_deref(), Some("order") | Some("admin_event")) && event_id.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "event_id is required when mode is order/admin_event".to_string(),
        ));
    }

    Ok(())
}

fn parse_master_product_ids(raw: &str) -> Result<Vec<i64>, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    if let Ok(ids) = serde_json::from_str::<Vec<i64>>(raw) {
        return Ok(ids.into_iter().filter(|id| *id > 0).collect());
    }

    let mut out = Vec::new();
    for token in raw.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let id = token
            .parse::<i64>()
            .map_err(|_| format!("invalid master_product_id: {}", token))?;
        if id > 0 {
            out.push(id);
        }
    }
    Ok(out)
}

fn validate_roi(roi: &Option<String>) -> Result<(), (StatusCode, String)> {
    if let Some(roi_raw) = roi {
        let value: serde_json::Value = serde_json::from_str(roi_raw).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "roi must be a valid JSON string".to_string(),
            )
        })?;

        for key in ["x", "y", "w", "h"] {
            let maybe_num = value.get(key).and_then(|v| v.as_f64());
            if maybe_num.is_none() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("roi.{} is required and must be number", key),
                ));
            }
        }
    }

    Ok(())
}

async fn parse_search_request(
    mut multipart: Multipart,
) -> Result<SearchRequestContext, (StatusCode, String)> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut top_k = 5_i64;
    let mut mode: Option<String> = None;
    let mut master_product_ids: Option<Vec<i64>> = None;
    let mut event_id: Option<i64> = None;
    let mut roi: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "image" {
            // 查询图只在内存里用一次，不落盘：顾客端用前置摄像头，拍到的多半是人脸。
            let bytes = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            image_bytes = Some(bytes.to_vec());
            continue;
        }

        let value = field.text().await.unwrap_or_default();
        match field_name.as_str() {
            "top_k" => {
                let parsed = value.parse::<i64>().unwrap_or(5);
                top_k = parsed.clamp(1, 20);
            }
            "mode" => {
                if !value.is_empty() {
                    mode = Some(value);
                }
            }
            "master_product_ids" => {
                if !value.is_empty() {
                    let ids = parse_master_product_ids(&value)
                        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
                    master_product_ids = Some(ids);
                }
            }
            "event_id" => {
                event_id = value.parse::<i64>().ok();
            }
            "roi" if !value.is_empty() => {
                roi = Some(value);
            }
            _ => {}
        }
    }

    let image_bytes = image_bytes.ok_or((
        StatusCode::BAD_REQUEST,
        "image field is required".to_string(),
    ))?;

    validate_mode(&mode, event_id)?;
    validate_roi(&roi)?;

    Ok(SearchRequestContext {
        image_bytes,
        top_k,
        mode,
        master_product_ids,
        event_id,
        roi,
    })
}

/// 以图搜图：multipart 上传查询图片，返回按相似度排序的商品候选。
///
/// 索引未就绪或正在重建时返回 503；`mode` 为 `order`/`admin_event` 时 `event_id` 必填。
#[utoipa::path(
    post,
    path = "/search",
    tag = "vision",
    request_body(content = VisionSearchForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, body = VisionSearchResponse),
        (status = 400, body = ApiErrorBody, description = "缺少图片、mode/roi/master_product_ids 非法"),
        (status = 408, body = ApiErrorBody, description = "识别超时"),
        (status = 500, body = ApiErrorBody, description = "上传或推理失败"),
        (status = 503, body = ApiErrorBody, description = "索引重建中、未就绪或并发已满"),
    ),
)]
async fn search_by_image(State(state): State<AppState>, multipart: Multipart) -> Response {
    // 不需要展会守卫：只读识别，不写任何账
    let snapshot = state.vision_runtime.snapshot().await;

    if snapshot.is_rebuilding {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "VISION_REBUILDING",
                "reason": "vision index is rebuilding"
            })),
        )
            .into_response();
    }

    if let Some(e) = crate::vision::session::runtime_load_error() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "VISION_NOT_READY",
                "reason": e
            })),
        )
            .into_response();
    }

    if !snapshot.is_ready {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "VISION_NOT_READY",
                "reason": snapshot.reason.unwrap_or_else(|| "runtime is not ready".to_string())
            })),
        )
            .into_response();
    }

    let req = match parse_search_request(multipart).await {
        Ok(req) => req,
        Err((code, msg)) => {
            return (code, Json(json!({ "error": msg }))).into_response();
        }
    };

    let permit = match state.vision_runtime.semaphore().acquire_owned().await {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "VISION_BUSY"})),
            )
                .into_response()
        }
    };

    let query_future = async {
        let query_vec = state
            .vision_runtime
            .embed_query(&req.image_bytes)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        let store = VisionStore::new(state.db.clone());
        let candidates = store
            .load_search_candidates(&snapshot.model_version, req.mode.as_deref(), req.event_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let candidates = if let Some(ids) = req.master_product_ids.as_ref() {
            let id_set = ids.iter().copied().collect::<HashSet<_>>();
            candidates
                .into_iter()
                .filter(|item| id_set.contains(&item.master_product_id))
                .collect::<Vec<_>>()
        } else {
            candidates
        };

        let hits = index::search_top_k(&query_vec, &candidates, req.top_k as usize);

        let (top1_min, gap_min) = state
            .vision_runtime
            .thresholds_for_mode(req.mode.as_deref())
            .await;

        let is_uncertain = compute_uncertainty(top1_min, gap_min, &hits);

        let results = hits
            .into_iter()
            .map(|item| VisionSearchResult {
                master_product_id: item.master_product_id,
                product_code: item.product_code,
                name: item.name,
                score: item.score,
                thumb_url: item.thumb_url,
            })
            .collect::<Vec<_>>();

        Ok::<(bool, Vec<VisionSearchResult>), (StatusCode, String)>((is_uncertain, results))
    };

    let timeout_ms = state.vision_runtime.timeout_ms().await;
    let queried = timeout(Duration::from_millis(timeout_ms.max(1000)), query_future).await;
    drop(permit);

    let (is_uncertain, results) = match queried {
        Ok(Ok(ok)) => ok,
        Ok(Err((code, msg))) => return (code, Json(json!({"error": msg}))).into_response(),
        Err(_) => {
            return (
                StatusCode::REQUEST_TIMEOUT,
                Json(json!({"error": "VISION_TIMEOUT"})),
            )
                .into_response()
        }
    };

    Json(VisionSearchResponse {
        model_id: snapshot.model_id,
        model_version: snapshot.model_version,
        index_version: snapshot.index_version,
        is_uncertain,
        results,
    })
    .into_response()
}

fn compute_uncertainty(top1_min: f32, gap_min: f32, results: &[index::ProductSearchHit]) -> bool {
    if results.is_empty() {
        return true;
    }

    let top1 = results[0].score;
    if top1 < top1_min {
        return true;
    }

    if results.len() > 1 {
        let gap = top1 - results[1].score;
        if gap < gap_min {
            return true;
        }
    }

    false
}

/// 列出注册表中的所有 Vision 模型及其安装/激活状态。
#[utoipa::path(
    get,
    path = "/models",
    tag = "vision",
    responses((status = 200, body = VisionModelsResponse)),
)]
async fn list_models(State(state): State<AppState>) -> Json<VisionModelsResponse> {
    let models = state
        .vision_runtime
        .list_models()
        .await
        .into_iter()
        .map(|(item, installed, active)| VisionModelItem {
            model_id: item.model_id,
            model_version: item.model_version,
            description: item.description,
            tier: item.tier,
            dim: item.embed_dim,
            input_size: item.input_size,
            size_mb: item.size_mb,
            installed,
            is_active: active,
        })
        .collect::<Vec<_>>();

    let active_model_id = models
        .iter()
        .find(|item| item.is_active)
        .map(|item| item.model_id.clone())
        .unwrap_or_default();

    Json(VisionModelsResponse {
        active_model_id,
        models,
    })
}

/// 安装（下载）一个 Vision 模型，返回后台任务 id。
#[utoipa::path(
    post,
    path = "/models/install",
    tag = "vision",
    request_body = InstallModelRequest,
    responses(
        (status = 202, body = VisionInstallModelResponse),
        (status = 400, body = ApiErrorBody, description = "source 不受支持、模型不存在或已安装"),
    ),
)]
async fn install_model(
    State(state): State<AppState>,
    Json(payload): Json<InstallModelRequest>,
) -> Response {
    // 不需要展会守卫：模型文件管理，不属于任何展会
    if let Some(source) = &payload.source {
        let supported = ["auto", "github", "hf", "hf_mirror"];
        if !supported.iter().any(|item| item == source) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "unsupported source" })),
            )
                .into_response();
        }
    }

    match state
        .vision_runtime
        .create_install_task(&payload.model_id, payload.source.clone())
        .await
    {
        Ok(task_id) => (
            StatusCode::ACCEPTED,
            Json(VisionInstallModelResponse {
                ok: true,
                task_id,
                model_id: payload.model_id,
                source: payload.source.unwrap_or_else(|| "auto".to_string()),
            }),
        )
            .into_response(),
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": msg })),
        )
            .into_response(),
    }
}

/// 查询模型安装任务的状态。
#[utoipa::path(
    get,
    path = "/models/tasks/{task_id}",
    tag = "vision",
    params(("task_id" = String, Path, description = "安装任务 id")),
    responses(
        (status = 200, body = VisionInstallTaskResponse),
        (status = 404, body = ApiErrorBody, description = "任务不存在"),
    ),
)]
async fn get_install_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<Json<VisionInstallTaskResponse>> {
    match state.vision_runtime.get_install_task(&task_id).await {
        Some(task) => Ok(Json(VisionInstallTaskResponse {
            task_id: task.task_id,
            model_id: task.model_id,
            status: task.status,
            progress: task.progress,
            message: task.message,
            error: task.error,
        })),
        None => Err(ApiError::NotFound("task not found".into())),
    }
}

/// 激活一个已安装的 Vision 模型，并触发索引重建。
#[utoipa::path(
    post,
    path = "/models/activate",
    tag = "vision",
    request_body = ActivateModelRequest,
    responses(
        (status = 202, body = VisionActivateModelResponse),
        (status = 400, body = ApiErrorBody, description = "模型不存在或未安装"),
    ),
)]
async fn activate_model(
    State(state): State<AppState>,
    Json(payload): Json<ActivateModelRequest>,
) -> Response {
    // 不需要展会守卫：模型选择，不属于任何展会
    let result = state
        .vision_runtime
        .activate_model(state.db.clone(), &payload.model_id)
        .await;

    match result {
        Ok(_) => {
            state.vision_runtime.clone().start_rebuild_task(
                state.db.clone(),
                state.upload_dir.clone(),
                false,
                None,
            );

            (
                StatusCode::ACCEPTED,
                Json(VisionActivateModelResponse {
                    ok: true,
                    active_model_id: payload.model_id,
                    rebuild_started: true,
                }),
            )
                .into_response()
        }
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": msg
            })),
        )
            .into_response(),
    }
}

/// 触发 Vision 索引重建；请求体可选，`force_full` 决定全量还是增量。
#[utoipa::path(
    post,
    path = "/rebuild",
    tag = "vision",
    request_body = Option<RebuildRequest>,
    responses(
        (status = 202, body = VisionRebuildResponse),
        (status = 409, body = ApiErrorBody, description = "索引正在重建"),
    ),
)]
async fn rebuild_index(
    State(state): State<AppState>,
    payload: Option<Json<RebuildRequest>>,
) -> ApiResult<(StatusCode, Json<VisionRebuildResponse>)> {
    // 不需要展会守卫：重建全局识别索引，不写展会的账
    let snapshot = state.vision_runtime.snapshot().await;
    if snapshot.is_rebuilding {
        return Err(ApiError::Conflict("VISION_REBUILDING".into()));
    }

    let force_full = payload
        .as_ref()
        .and_then(|value| value.force_full)
        .unwrap_or(false);

    state.vision_runtime.clone().start_rebuild_task(
        state.db.clone(),
        state.upload_dir.clone(),
        force_full,
        None,
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(VisionRebuildResponse {
            ok: true,
            message: if force_full {
                "full rebuild started".to_string()
            } else {
                "incremental rebuild started".to_string()
            },
            force_full,
        }),
    ))
}

/// 删除一个已安装且非当前激活的 Vision 模型文件。
#[utoipa::path(
    delete,
    path = "/models/{model_id}",
    tag = "vision",
    params(("model_id" = String, Path, description = "模型 id")),
    responses(
        (status = 200, body = VisionDeleteModelResponse),
        (status = 400, body = ApiErrorBody, description = "不能删除激活模型、模型不存在或未安装"),
    ),
)]
async fn delete_model(State(state): State<AppState>, Path(model_id): Path<String>) -> Response {
    // 不需要展会守卫：模型文件管理，不属于任何展会
    match state.vision_runtime.delete_model(&model_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(VisionDeleteModelResponse {
                ok: true,
                deleted_model_id: model_id,
            }),
        )
            .into_response(),
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": msg })),
        )
            .into_response(),
    }
}

// ==================== EP 设置 ====================

/// 读取 Vision 推理设备（execution provider）配置与可用 GPU 列表。
#[utoipa::path(
    get,
    path = "/settings/ep",
    tag = "vision",
    responses((status = 200, body = VisionEpSettingResponse)),
)]
async fn get_ep_setting(State(state): State<AppState>) -> Json<VisionEpSettingResponse> {
    let ep = state
        .vision_runtime
        .model_manager
        .get_runtime_config()
        .await
        .map(|c| c.execution_provider)
        .unwrap_or_else(|| "auto".to_string());

    let current = crate::vision::session::get_active_ep_name();
    let gpu_devices = crate::vision::session::probe_gpu_devices()
        .into_iter()
        .map(|device| VisionGpuDevice {
            device_id: device.device_id,
            name: device.name,
        })
        .collect();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "other"
    };

    Json(VisionEpSettingResponse {
        configured: ep,
        active: current,
        platform: platform.to_string(),
        gpu_devices,
    })
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(as = VisionSetEpRequest)]
struct SetEpRequest {
    execution_provider: String,
}

/// 设置 Vision 推理设备并立即重新加载模型。
#[utoipa::path(
    put,
    path = "/settings/ep",
    tag = "vision",
    request_body = SetEpRequest,
    responses(
        (status = 200, body = VisionSetEpResponse),
        (status = 400, body = ApiErrorBody, description = "EP 取值非法"),
        (status = 500, body = ApiErrorBody, description = "配置保存或加载失败"),
    ),
)]
async fn set_ep_setting(
    State(state): State<AppState>,
    Json(payload): Json<SetEpRequest>,
) -> Response {
    // 不需要展会守卫：全局 OCR 设置，不属于任何展会
    let ep = &payload.execution_provider;
    let valid = ep == "auto" || ep == "cpu" || ep == "nnapi" || ep.starts_with("gpu:");
    if !valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid EP. Use: auto, cpu, gpu:0, gpu:1, ..." })),
        )
            .into_response();
    }

    if let Err(e) = state
        .vision_runtime
        .model_manager
        .set_execution_provider(ep)
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to save: {}", e) })),
        )
            .into_response();
    }

    // 清除缓存并立即重新加载模型，使新 EP 立即生效
    state.vision_runtime.session_cache.clear().await;

    // 触发模型重新加载（用新的 EP 配置）
    let reload_result = async {
        let snapshot = state.vision_runtime.snapshot().await;
        let manifest = state
            .vision_runtime
            .model_manager
            .get_manifest_for_model(&snapshot.model_id)
            .await
            .ok_or_else(|| "active model not found".to_string())?;

        let model_path = crate::vision::download::model_abs_path(
            state.vision_runtime.model_manager.app_data_dir(),
            &manifest,
        );
        if !model_path.exists() {
            return Err("model file not found".to_string());
        }

        let model_id = manifest.model_id.clone();
        let model_version = manifest.model_version.clone();
        state
            .vision_runtime
            .session_cache
            .get_or_load_with_check(&model_id, &model_version, &model_path, manifest, ep.clone())
            .await
            .map(|_| ())
    }
    .await;

    let active = crate::vision::session::get_active_ep_name();

    match reload_result {
        Ok(_) => (
            StatusCode::OK,
            Json(VisionSetEpResponse {
                ok: true,
                execution_provider: ep.clone(),
                active,
                message: "设备已切换并重新加载模型".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::OK,
            Json(VisionSetEpResponse {
                ok: true,
                execution_provider: ep.clone(),
                active,
                message: format!("配置已保存，但重新加载失败: {}", e),
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod shape_tests {
    use super::{
        VisionInstallModelResponse, VisionInstallTaskResponse, VisionSearchResponse,
        VisionSearchResult,
    };
    use crate::state::AppState;
    use crate::test_support::{
        json_request, read_json, seed_event_and_product, shape_of, test_state,
    };
    use crate::vision::download::model_root_dir;
    use crate::vision::state::VisionStatusSnapshot;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    /// Vision registry / runtime config 已 bootstrap、且带两个商品的展会。
    ///
    /// 返回 `AppState` 是为了能直接摆弄 `vision_runtime.state`——搜索与重建的有
    /// 状态分支（未就绪 / 重建中 / 已就绪）只能这样构造，没有别的注入点。
    async fn seeded() -> (Router, tempfile::TempDir, AppState) {
        let (state, dir) = test_state().await;
        state
            .vision_runtime
            .bootstrap(state.db.clone())
            .await
            .expect("bootstrap vision runtime");
        seed_event_and_product(&state.db).await;
        let router = Router::new()
            .nest("/api", crate::api::router().split_for_parts().0)
            .with_state(state.clone());
        (router, dir, state)
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

    fn multipart_request(uri: &str, fields: &[(&str, &str)]) -> Request<Body> {
        let boundary = "X-BOUNDARY";
        let mut body = String::new();
        for (name, value) in fields {
            body.push_str(&format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            ));
        }
        body.push_str(&format!("--{boundary}--\r\n"));
        Request::builder()
            .method("POST")
            .uri(uri)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .expect("build multipart request")
    }

    async fn multipart_call(
        router: &Router,
        uri: &str,
        fields: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(multipart_request(uri, fields))
            .await
            .unwrap();
        let status = res.status();
        (status, read_json(res).await)
    }

    fn ready_state(model_id: &str, model_version: &str) -> VisionStatusSnapshot {
        VisionStatusSnapshot {
            model_id: model_id.to_string(),
            model_version: model_version.to_string(),
            index_version: 1,
            index_size: 3,
            is_ready: true,
            is_rebuilding: false,
            last_rebuild_at: Some("2026-10-01 12:00:00".to_string()),
            reason: None,
            rebuild_processed: 0,
            rebuild_total: 0,
            last_rebuild_error: None,
        }
    }

    /// 造一个模型文件，让 activate/delete 走到成功分支（内容无所谓，不会真的加载）。
    fn write_fake_model(state: &AppState, model_id: &str) {
        let path = model_root_dir(&state.app_data_dir).join(format!("{model_id}/model.onnx"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"not-a-real-onnx").unwrap();
    }

    #[tokio::test]
    async fn shape_get_vision_status() {
        let (router, _dir, _state) = seeded().await;
        let (s, body) = call(&router, "GET", "/api/vision/status", None, json!(null)).await;
        println!(
            "shape_get_vision_status status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "model_id": "string",
                "model_version": "string",
                "index_version": "int",
                "index_size": "int",
                "last_rebuild_at": "string",
                "is_ready": "bool",
                "is_rebuilding": "bool",
                "reason": "string",
                "rebuild_processed": "int",
                "rebuild_total": "int",
                "execution_provider": "string",
                "last_rebuild_error": "null",
                "runtime_error": "null",
            })
        );
    }

    #[tokio::test]
    async fn shape_search_by_image() {
        let (router, _dir, state) = seeded().await;

        // bootstrap 后 index_size=0：未就绪
        let (s, body) = multipart_call(&router, "/api/vision/search", &[("image", "x")]).await;
        println!(
            "shape_search_by_image not_ready status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            shape_of(&body),
            json!({"error": "string", "reason": "string"})
        );

        // 重建中优先于未就绪
        let mut snap = state.vision_runtime.snapshot().await;
        snap.is_rebuilding = true;
        state.vision_runtime.state.set(snap).await;
        let (s, body) = multipart_call(&router, "/api/vision/search", &[("image", "x")]).await;
        println!(
            "shape_search_by_image rebuilding status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            shape_of(&body),
            json!({"error": "string", "reason": "string"})
        );

        // 已就绪但缺 image
        state
            .vision_runtime
            .state
            .set(ready_state(
                "convnextv2_pico_fp16",
                "convnextv2_pico_fp16-v1",
            ))
            .await;
        let (s, body) = multipart_call(&router, "/api/vision/search", &[("top_k", "5")]).await;
        println!(
            "shape_search_by_image no_image status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(shape_of(&body), json!({"error": "string"}));

        // 已就绪 + image，但没有模型文件：解析通过，推理失败成 500
        let (s, body) = multipart_call(
            &router,
            "/api/vision/search",
            &[("image", "x"), ("top_k", "5")],
        )
        .await;
        println!(
            "shape_search_by_image embed_fail status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(shape_of(&body), json!({"error": "string"}));
        assert!(
            !state.upload_dir.join("vision/query").exists(),
            "查询图不能落盘"
        );

        // 成功分支需要真 ONNX 模型文件（测试环境没有），所以 HTTP 只能钉到 500。
        // 这里直接对 handler 构造的同一个响应结构做序列化快照，防止字段改名漏网。
        let success = VisionSearchResponse {
            model_id: "m".to_string(),
            model_version: "v".to_string(),
            index_version: 1,
            is_uncertain: false,
            results: vec![VisionSearchResult {
                master_product_id: 1,
                product_code: "A".to_string(),
                name: "本子A".to_string(),
                score: 0.5,
                thumb_url: Some("/uploads/x.jpg".to_string()),
            }],
        };
        assert_eq!(
            shape_of(&serde_json::to_value(&success).unwrap()),
            json!({
                "model_id": "string",
                "model_version": "string",
                "index_version": "int",
                "is_uncertain": "bool",
                "results": [{
                    "master_product_id": "int",
                    "product_code": "string",
                    "name": "string",
                    "score": "float",
                    "thumb_url": "string",
                }],
            })
        );
    }

    #[tokio::test]
    async fn shape_rebuild_index() {
        let (router, _dir, _state) = seeded().await;
        let (s, body) = call(
            &router,
            "POST",
            "/api/vision/rebuild",
            None,
            json!({"force_full": true}),
        )
        .await;
        println!(
            "shape_rebuild_index full status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::ACCEPTED);
        assert_eq!(
            shape_of(&body),
            json!({"ok": "bool", "message": "string", "force_full": "bool"})
        );

        let (router, _dir, state) = seeded().await;
        let mut snap = state.vision_runtime.snapshot().await;
        snap.is_rebuilding = true;
        state.vision_runtime.state.set(snap).await;
        let (s, body) = call(
            &router,
            "POST",
            "/api/vision/rebuild",
            None,
            json!({"force_full": false}),
        )
        .await;
        println!(
            "shape_rebuild_index conflict status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::CONFLICT);
        assert_eq!(shape_of(&body), json!({"error": "string"}));
    }

    #[tokio::test]
    async fn feedback_route_is_gone() {
        // 免登录、扩展名随客户端定、还把 URL 回给调用方——前端也没用它，删掉。
        let (router, _dir, _state) = seeded().await;
        let (s, _body) = multipart_call(
            &router,
            "/api/vision/feedback",
            &[("image", "x"), ("chosen_master_product_id", "1")],
        )
        .await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn shape_list_models() {
        let (router, _dir, _state) = seeded().await;
        let (s, body) = call(&router, "GET", "/api/vision/models", None, json!(null)).await;
        println!(
            "shape_list_models status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "active_model_id": "string",
                "models": [{
                    "model_id": "string",
                    "model_version": "string",
                    "description": "string",
                    "tier": "string",
                    "dim": "int",
                    "input_size": "int",
                    "size_mb": "float",
                    "installed": "bool",
                    "is_active": "bool",
                }],
            })
        );
    }

    #[tokio::test]
    async fn shape_install_model() {
        let (router, _dir, _state) = seeded().await;
        let (s, body) = call(
            &router,
            "POST",
            "/api/vision/models/install",
            None,
            json!({"model_id": "convnextv2_pico_fp16", "source": "bogus"}),
        )
        .await;
        println!(
            "shape_install_model bad_source status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(shape_of(&body), json!({"error": "string"}));

        let (s, body) = call(
            &router,
            "POST",
            "/api/vision/models/install",
            None,
            json!({"model_id": "nope"}),
        )
        .await;
        println!(
            "shape_install_model unknown status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(shape_of(&body), json!({"ok": "bool", "error": "string"}));

        // 202 成功分支会真的起一个下载任务（测试不打网络），所以直接对 handler
        // 构造的同一响应结构做序列化快照；字段改名会在这里变红。
        let success = VisionInstallModelResponse {
            ok: true,
            task_id: "t".to_string(),
            model_id: "m".to_string(),
            source: "auto".to_string(),
        };
        assert_eq!(
            shape_of(&serde_json::to_value(&success).unwrap()),
            json!({"ok": "bool", "task_id": "string", "model_id": "string", "source": "string"})
        );
    }

    #[tokio::test]
    async fn shape_get_install_task() {
        let (router, _dir, _state) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            "/api/vision/models/tasks/nope",
            None,
            json!(null),
        )
        .await;
        println!(
            "shape_get_install_task not_found status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert_eq!(shape_of(&body), json!({"error": "string"}));

        // 成功分支同样要有一个真实安装任务，直接对具名结构做序列化快照。
        let success = VisionInstallTaskResponse {
            task_id: "t".to_string(),
            model_id: "m".to_string(),
            status: "downloading".to_string(),
            progress: 1,
            message: Some("Task created".to_string()),
            error: None,
        };
        assert_eq!(
            shape_of(&serde_json::to_value(&success).unwrap()),
            json!({
                "task_id": "string",
                "model_id": "string",
                "status": "string",
                "progress": "int",
                "message": "string",
                "error": "null",
            })
        );
    }

    #[tokio::test]
    async fn shape_activate_model() {
        let (router, _dir, state) = seeded().await;
        write_fake_model(&state, "dinov2_small_fp16");
        let (s, body) = call(
            &router,
            "POST",
            "/api/vision/models/activate",
            None,
            json!({"model_id": "dinov2_small_fp16"}),
        )
        .await;
        println!(
            "shape_activate_model ok status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::ACCEPTED);
        assert_eq!(
            shape_of(&body),
            json!({"ok": "bool", "active_model_id": "string", "rebuild_started": "bool"})
        );

        let (s, body) = call(
            &router,
            "POST",
            "/api/vision/models/activate",
            None,
            json!({"model_id": "nope"}),
        )
        .await;
        println!(
            "shape_activate_model unknown status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(shape_of(&body), json!({"ok": "bool", "error": "string"}));
    }

    #[tokio::test]
    async fn shape_delete_model() {
        let (router, _dir, state) = seeded().await;
        write_fake_model(&state, "dinov2_small_fp16");
        let (s, body) = call(
            &router,
            "DELETE",
            "/api/vision/models/dinov2_small_fp16",
            None,
            json!(null),
        )
        .await;
        println!(
            "shape_delete_model ok status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({"ok": "bool", "deleted_model_id": "string"})
        );

        let (s, body) = call(
            &router,
            "DELETE",
            "/api/vision/models/convnextv2_pico_fp16",
            None,
            json!(null),
        )
        .await;
        println!(
            "shape_delete_model active status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(shape_of(&body), json!({"ok": "bool", "error": "string"}));
    }

    #[tokio::test]
    async fn shape_get_ep_setting() {
        let (router, _dir, _state) = seeded().await;
        let (s, body) = call(&router, "GET", "/api/vision/settings/ep", None, json!(null)).await;
        println!(
            "shape_get_ep_setting status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "configured": "string",
                "active": "string",
                "platform": "string",
                "gpu_devices": ["empty"],
            })
        );
    }

    #[tokio::test]
    async fn shape_set_ep_setting() {
        let (router, _dir, _state) = seeded().await;
        let (s, body) = call(
            &router,
            "PUT",
            "/api/vision/settings/ep",
            None,
            json!({"execution_provider": "cpu"}),
        )
        .await;
        println!(
            "shape_set_ep_setting ok status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "ok": "bool",
                "execution_provider": "string",
                "active": "string",
                "message": "string",
            })
        );

        let (s, body) = call(
            &router,
            "PUT",
            "/api/vision/settings/ep",
            None,
            json!({"execution_provider": "bogus"}),
        )
        .await;
        println!(
            "shape_set_ep_setting invalid status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(shape_of(&body), json!({"error": "string"}));

        let (router, dir, _state) = seeded().await;
        std::fs::remove_dir_all(dir.path().join("models/vision")).unwrap();
        let (s, body) = call(
            &router,
            "PUT",
            "/api/vision/settings/ep",
            None,
            json!({"execution_provider": "cpu"}),
        )
        .await;
        println!(
            "shape_set_ep_setting load_fail status={s} shape={}",
            serde_json::to_string(&shape_of(&body)).unwrap()
        );
        assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(shape_of(&body), json!({"error": "string"}));
    }

    #[tokio::test]
    async fn ep_setting_is_visible_immediately_and_survives_model_activation() {
        let (router, dir, state) = seeded().await;
        let (s, _) = call(
            &router,
            "PUT",
            "/api/vision/settings/ep",
            None,
            json!({"execution_provider": "cpu"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        let (_, body) = call(&router, "GET", "/api/vision/settings/ep", None, json!(null)).await;
        assert_eq!(body["configured"], "cpu", "GET 必须立刻读到新值");

        // 激活会把内存配置整份写回文件，不能把 EP 冲回 auto
        write_fake_model(&state, "dinov2_small_fp16");
        let (s, _) = call(
            &router,
            "POST",
            "/api/vision/models/activate",
            None,
            json!({"model_id": "dinov2_small_fp16"}),
        )
        .await;
        assert_eq!(s, StatusCode::ACCEPTED);
        let raw =
            std::fs::read_to_string(dir.path().join("models/vision/vision_model.json")).unwrap();
        let cfg: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(cfg["runtime"]["execution_provider"], "cpu");
        assert_eq!(cfg["active_model_id"], "dinov2_small_fp16");
    }

    #[tokio::test]
    async fn rebuild_failure_is_reported_in_status() {
        // bootstrap 激活了模型但磁盘上没有模型文件：重建一定失败，状态里要看得到原因。
        let (router, _dir, _state) = seeded().await;
        let (s, _) = call(
            &router,
            "POST",
            "/api/vision/rebuild",
            None,
            json!({"force_full": false}),
        )
        .await;
        assert_eq!(s, StatusCode::ACCEPTED);

        let mut body = json!(null);
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            body = call(&router, "GET", "/api/vision/status", None, json!(null))
                .await
                .1;
            if body["reason"] == "VISION_REBUILD_FAILED" {
                break;
            }
        }
        assert_eq!(body["is_rebuilding"], false);
        assert_eq!(body["reason"], "VISION_REBUILD_FAILED", "{body}");
        assert!(body["last_rebuild_error"].is_string(), "{body}");
    }
}
