use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use tokio::time::{timeout, Duration};

use crate::{
    state::AppState,
    utils::file::{save_upload_bytes, save_upload_file},
    vision::{index, store::VisionStore},
};

const VISION_UPLOAD_LIMIT_BYTES: usize = 10 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_vision_status))
        .route("/search", post(search_by_image))
        .route("/rebuild", post(rebuild_index))
        .route("/feedback", post(save_feedback))
        .route("/models", get(list_models))
        .route("/models/install", post(install_model))
        .route("/models/tasks/:task_id", get(get_install_task))
        .route("/models/activate", post(activate_model))
        .route("/models/:model_id", delete(delete_model))
        .route("/settings/ep", get(get_ep_setting))
        .route("/settings/ep", put(set_ep_setting))
        .layer(DefaultBodyLimit::max(VISION_UPLOAD_LIMIT_BYTES))
}

#[derive(Debug, Serialize)]
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
}

async fn get_vision_status(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot = state.vision_runtime.snapshot().await;

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
    })
}

#[derive(Debug, Serialize)]
struct VisionSearchResult {
    master_product_id: i64,
    product_code: String,
    name: String,
    score: f32,
    thumb_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct VisionSearchResponse {
    model_id: String,
    model_version: String,
    index_version: i64,
    is_uncertain: bool,
    results: Vec<VisionSearchResult>,
}

#[derive(Debug, Serialize)]
struct VisionModelItem {
    model_id: String,
    model_version: String,
    description: Option<String>,
    tier: Option<String>,
    dim: usize,
    input_size: usize,
    installed: bool,
    is_active: bool,
}

#[derive(Debug, Serialize)]
struct VisionModelsResponse {
    active_model_id: String,
    models: Vec<VisionModelItem>,
}

#[derive(Debug, Deserialize)]
struct ActivateModelRequest {
    model_id: String,
}

#[derive(Debug, Deserialize)]
struct InstallModelRequest {
    model_id: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RebuildRequest {
    force_full: Option<bool>,
}

#[derive(Debug)]
struct SearchRequestContext {
    image_url: String,
    image_bytes: Vec<u8>,
    top_k: i64,
    mode: Option<String>,
    master_product_ids: Option<Vec<i64>>,
    event_id: Option<i64>,
    roi: Option<String>,
}

#[derive(Debug)]
struct FeedbackRequestContext {
    image_url: String,
    chosen_master_product_id: i64,
    is_correct: bool,
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
    state: &AppState,
    mut multipart: Multipart,
) -> Result<SearchRequestContext, (StatusCode, String)> {
    let mut image_url: Option<String> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut top_k = 5_i64;
    let mut mode: Option<String> = None;
    let mut master_product_ids: Option<Vec<i64>> = None;
    let mut event_id: Option<i64> = None;
    let mut roi: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "image" {
            let file_name = field.file_name().map(|value| value.to_string());
            let bytes = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let path = save_upload_bytes(
                &state.upload_dir,
                &bytes,
                file_name.as_deref(),
                Some("vision/query"),
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

            image_url = Some(path);
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
            "roi" => {
                if !value.is_empty() {
                    roi = Some(value);
                }
            }
            _ => {}
        }
    }

    let image_url = image_url.ok_or((
        StatusCode::BAD_REQUEST,
        "image field is required".to_string(),
    ))?;
    let image_bytes = image_bytes.ok_or((
        StatusCode::BAD_REQUEST,
        "image field is required".to_string(),
    ))?;

    validate_mode(&mode, event_id)?;
    validate_roi(&roi)?;

    Ok(SearchRequestContext {
        image_url,
        image_bytes,
        top_k,
        mode,
        master_product_ids,
        event_id,
        roi,
    })
}

async fn parse_feedback_request(
    state: &AppState,
    mut multipart: Multipart,
) -> Result<FeedbackRequestContext, (StatusCode, String)> {
    let mut image_url: Option<String> = None;
    let mut chosen_master_product_id: Option<i64> = None;
    let mut is_correct = true;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "image" {
            let path = save_upload_file(&state.upload_dir, field, Some("vision/feedback"))
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            image_url = Some(path);
            continue;
        }

        let value = field.text().await.unwrap_or_default();
        match field_name.as_str() {
            "chosen_master_product_id" => {
                chosen_master_product_id = value.parse::<i64>().ok();
            }
            "is_correct" => {
                is_correct = matches!(value.as_str(), "1" | "true" | "TRUE" | "True");
            }
            _ => {}
        }
    }

    Ok(FeedbackRequestContext {
        image_url: image_url.ok_or((
            StatusCode::BAD_REQUEST,
            "image field is required".to_string(),
        ))?,
        chosen_master_product_id: chosen_master_product_id.ok_or((
            StatusCode::BAD_REQUEST,
            "chosen_master_product_id is required".to_string(),
        ))?,
        is_correct,
    })
}

async fn search_by_image(State(state): State<AppState>, multipart: Multipart) -> impl IntoResponse {
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

    let req = match parse_search_request(&state, multipart).await {
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

async fn save_feedback(State(state): State<AppState>, multipart: Multipart) -> impl IntoResponse {
    let req = match parse_feedback_request(&state, multipart).await {
        Ok(req) => req,
        Err((code, msg)) => return (code, Json(json!({ "error": msg }))).into_response(),
    };

    let store = VisionStore::new(state.db.clone());
    let exists = match store
        .master_product_exists(req.chosen_master_product_id)
        .await
    {
        Ok(value) => value,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("DB Error: {}", e) })),
            )
                .into_response();
        }
    };

    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "master product not found" })),
        )
            .into_response();
    }

    let kind = if req.is_correct {
        "feedback"
    } else {
        "feedback_incorrect"
    };

    match store
        .insert_master_product_image(req.chosen_master_product_id, &req.image_url, kind)
        .await
    {
        Ok(image_id) => {
            state.vision_runtime.clone().start_incremental_for_images(
                state.db.clone(),
                state.upload_dir.clone(),
                vec![image_id],
            );

            (
                StatusCode::CREATED,
                Json(json!({
                    "ok": true,
                    "image_id": image_id,
                    "image_url": req.image_url,
                    "kind": kind
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("DB Error: {}", e) })),
        )
            .into_response(),
    }
}

async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
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

async fn install_model(
    State(state): State<AppState>,
    Json(payload): Json<InstallModelRequest>,
) -> impl IntoResponse {
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
            Json(json!({
                "ok": true,
                "task_id": task_id,
                "model_id": payload.model_id,
                "source": payload.source.unwrap_or_else(|| "auto".to_string())
            })),
        )
            .into_response(),
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": msg })),
        )
            .into_response(),
    }
}

async fn get_install_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    match state.vision_runtime.get_install_task(&task_id).await {
        Some(task) => (
            StatusCode::OK,
            Json(json!({
                "task_id": task.task_id,
                "model_id": task.model_id,
                "status": task.status,
                "progress": task.progress,
                "message": task.message,
                "error": task.error
            })),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "task not found" })),
        )
            .into_response(),
    }
}

async fn activate_model(
    State(state): State<AppState>,
    Json(payload): Json<ActivateModelRequest>,
) -> impl IntoResponse {
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
                Json(json!({
                    "ok": true,
                    "active_model_id": payload.model_id,
                    "rebuild_started": true
                })),
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

async fn rebuild_index(
    State(state): State<AppState>,
    payload: Option<Json<RebuildRequest>>,
) -> impl IntoResponse {
    let snapshot = state.vision_runtime.snapshot().await;
    if snapshot.is_rebuilding {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "VISION_REBUILDING" })),
        )
            .into_response();
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

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "ok": true,
            "message": if force_full { "full rebuild started" } else { "incremental rebuild started" },
            "force_full": force_full
        })),
    )
        .into_response()
}

async fn delete_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    match state.vision_runtime.delete_model(&model_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "ok": true, "deleted_model_id": model_id })),
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

async fn get_ep_setting(State(state): State<AppState>) -> impl IntoResponse {
    let ep = state
        .vision_runtime
        .model_manager
        .get_runtime_config()
        .await
        .map(|c| c.execution_provider)
        .unwrap_or_else(|| "auto".to_string());

    let current = crate::vision::session::get_active_ep_name();
    let gpu_devices = crate::vision::session::probe_gpu_devices();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "other"
    };

    Json(json!({
        "configured": ep,
        "active": current,
        "platform": platform,
        "gpu_devices": gpu_devices
    }))
}

#[derive(Debug, Deserialize)]
struct SetEpRequest {
    execution_provider: String,
}

async fn set_ep_setting(
    State(state): State<AppState>,
    Json(payload): Json<SetEpRequest>,
) -> impl IntoResponse {
    let ep = &payload.execution_provider;
    let valid = ep == "auto" || ep == "cpu" || ep == "nnapi" || ep.starts_with("gpu:");
    if !valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid EP. Use: auto, cpu, gpu:0, gpu:1, ..." })),
        )
            .into_response();
    }

    let app_data_dir = state.vision_runtime.model_manager.app_data_dir();
    match crate::vision::download::load_runtime_config(app_data_dir).await {
        Ok(mut cfg) => {
            cfg.runtime.execution_provider = ep.clone();
            if let Err(e) = crate::vision::download::save_runtime_config(app_data_dir, &cfg).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": format!("Failed to save: {}", e) })),
                )
                    .into_response();
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to load config: {}", e) })),
            )
                .into_response();
        }
    }

    // 清除缓存并立即重新加载模型，使新 EP 立即生效
    state.vision_runtime.session_cache.clear().await;

    // 触发模型重新加载（用新的 EP 配置）
    let reload_result = async {
        let snapshot = state.vision_runtime.snapshot().await;
        let manifest = state.vision_runtime.model_manager
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
        state.vision_runtime.session_cache
            .get_or_load_with_check(
                &model_id,
                &model_version,
                &model_path,
                manifest,
                ep.clone(),
            )
            .await
            .map(|_| ())
    }.await;

    let active = crate::vision::session::get_active_ep_name();

    match reload_result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "execution_provider": ep,
                "active": active,
                "message": "设备已切换并重新加载模型"
            })),
        ).into_response(),
        Err(e) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "execution_provider": ep,
                "active": active,
                "message": format!("配置已保存，但重新加载失败: {}", e)
            })),
        ).into_response(),
    }
}
