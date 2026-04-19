use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, put},
    Router,
};
use serde::Deserialize;
#[cfg(feature = "vision")]
use serde::Serialize;
use serde_json::json;
use sqlx::{query, query_as};

#[cfg(feature = "vision")]
use crate::vision::store::VisionStore;
use crate::{
    api::guard::AdminOnly,
    db::models::MasterProduct,
    state::AppState,
    utils::file::{delete_file, save_upload_file},
};

const PRODUCT_UPLOAD_LIMIT_BYTES: usize = 10 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    let router = Router::new()
        .route("/", get(list_products))
        .route("/", post(create_product))
        .route("/:id", post(update_product).put(update_product))
        .route("/:id/status", put(update_status));

    #[cfg(feature = "vision")]
    let router = router
        .route("/:id/images", get(list_product_images))
        .route("/:id/images", post(add_product_image))
        .route(
            "/:id/images/:image_id",
            post(update_product_image).put(update_product_image),
        )
        .route(
            "/:id/images/:image_id",
            axum::routing::delete(delete_product_image),
        );

    // layer 放在所有路由之后，确保覆盖全部路由（包括 vision images）
    router.layer(DefaultBodyLimit::max(PRODUCT_UPLOAD_LIMIT_BYTES))
}

#[cfg(feature = "vision")]
#[derive(Debug, Serialize)]
struct ProductImageDto {
    id: i64,
    master_product_id: i64,
    image_url: String,
    kind: String,
    created_at: String,
    has_embedding: bool,
}

#[cfg(feature = "vision")]
async fn list_product_images(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let store = VisionStore::new(state.db.clone());
    let active_model_version = {
        let snapshot = state.vision_runtime.snapshot().await;
        snapshot.model_version
    };

    match store.list_master_product_images(id).await {
        Ok(items) => {
            let mut dtos = Vec::with_capacity(items.len());
            for item in items {
                let has_embedding = store
                    .has_embedding_for_image(item.id, &active_model_version)
                    .await
                    .unwrap_or(false);
                let url = if item.image_url.starts_with("/uploads/") {
                    item.image_url
                } else {
                    format!("/uploads/{}", item.image_url)
                };
                dtos.push(ProductImageDto {
                    id: item.id,
                    master_product_id: item.master_product_id,
                    image_url: url,
                    kind: item.kind,
                    created_at: item.created_at,
                    has_embedding,
                });
            }
            Json(dtos).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response(),
    }
}

#[derive(Deserialize)]
struct ListQuery {
    all: Option<bool>,
}

async fn list_products(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> impl IntoResponse {
    let show_all = params.all.unwrap_or(false);

    let sql = if show_all {
        "SELECT * FROM master_products ORDER BY product_code ASC"
    } else {
        "SELECT * FROM master_products WHERE is_active = 1 ORDER BY product_code ASC"
    };

    let products: Vec<MasterProduct> = query_as::<_, MasterProduct>(sql)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    Json(products)
}

async fn create_product(
    State(state): State<AppState>,
    _: AdminOnly,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut product_code = String::new();
    let mut name = String::new();
    let mut default_price: f64 = 0.0;
    let mut category: Option<String> = None;
    let mut tags = String::new();
    let mut image_path: Option<String> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "image" {
            match save_upload_file(&state.upload_dir, field, Some("products")).await {
                Ok(path) => image_path = Some(path),
                Err(e) => {
                    eprintln!("Upload error: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "File upload failed")
                        .into_response();
                }
            }
        } else {
            let value = field.text().await.unwrap_or_default();
            match field_name.as_str() {
                "product_code" => product_code = value,
                "name" => name = value,
                "default_price" => default_price = value.parse().unwrap_or(0.0),
                "category" => category = if value.is_empty() { None } else { Some(value) },
                "tags" => tags = value,
                _ => {}
            }
        }
    }

    if product_code.is_empty() || name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Code and Name are required"})),
        )
            .into_response();
    }

    let result = query_as::<_, MasterProduct>(
        r#"
        INSERT INTO master_products (product_code, name, default_price, category, image_url, tags)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(product_code)
    .bind(name)
    .bind(default_price)
    .bind(category)
    .bind(image_path)
    .bind(tags)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(product) => {

            (StatusCode::CREATED, Json(product)).into_response()
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") {
                (
                    StatusCode::CONFLICT,
                    Json(json!({"error": "Product code already exists"})),
                )
                    .into_response()
            } else {
                eprintln!("DB Error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
            }
        }
    }
}

async fn update_product(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let old_product: MasterProduct = match query_as("SELECT * FROM master_products WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "Product not found").into_response(),
    };

    let mut product_code = old_product.product_code;
    let mut name = old_product.name;
    let mut default_price = old_product.default_price;
    let mut category = old_product.category;
    let mut tags = old_product.tags;
    let mut image_path = old_product.image_url;
    let mut should_remove_image = false;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "image" {
            match save_upload_file(&state.upload_dir, field, Some("products")).await {
                Ok(new_path) => {
                    if let Some(old_path) = &image_path {
                        let _ = delete_file(&state.upload_dir, old_path).await;
                    }
                    image_path = Some(new_path);
                }
                Err(e) => eprintln!("Update upload error: {}", e),
            }
        } else {
            let value = field.text().await.unwrap_or_default();
            match field_name.as_str() {
                "product_code" => product_code = value,
                "name" => name = value,
                "default_price" => {
                    if !value.is_empty() {
                        default_price = value.parse().unwrap_or(default_price);
                    }
                }
                "category" => category = if value.is_empty() { None } else { Some(value) },
                "tags" => tags = value,
                "remove_image" => {
                    if value == "true" {
                        should_remove_image = true;
                    }
                }
                _ => {}
            }
        }
    }

    if should_remove_image {
        if let Some(old_path) = &image_path {
            let _ = delete_file(&state.upload_dir, old_path).await;
        }
        image_path = None;
    }

    let result = query_as::<_, MasterProduct>(
        r#"
        UPDATE master_products
        SET product_code = ?, name = ?, default_price = ?, category = ?, image_url = ?, tags = ?
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(product_code)
    .bind(name)
    .bind(default_price)
    .bind(category)
    .bind(image_path)
    .bind(tags)
    .bind(id)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(product) => {

            (StatusCode::OK, Json(product)).into_response()
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") {
                (
                    StatusCode::CONFLICT,
                    Json(json!({"error": "Product code already exists"})),
                )
                    .into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response()
            }
        }
    }
}

#[derive(Deserialize)]
struct UpdateStatusRequest {
    is_active: bool,
}

async fn update_status(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    let result = query_as::<_, MasterProduct>(
        r#"
        UPDATE master_products
        SET is_active = ?
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(payload.is_active)
    .bind(id)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(product) => (StatusCode::OK, Json(product)).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response(),
    }
}

#[cfg(feature = "vision")]
async fn add_product_image(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(master_product_id): Path<i64>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut image_url: Option<String> = None;
    let mut kind = "gallery".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "image" {
            match save_upload_file(&state.upload_dir, field, Some("products")).await {
                Ok(path) => image_url = Some(path),
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e})))
                        .into_response();
                }
            }
        } else if field_name == "kind" {
            let value = field.text().await.unwrap_or_default();
            if !value.is_empty() {
                kind = value;
            }
        }
    }

    let image_url = match image_url {
        Some(value) => value,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "image is required"})),
            )
                .into_response();
        }
    };

    let store = VisionStore::new(state.db.clone());
    match store
        .insert_master_product_image(master_product_id, &image_url, &kind)
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
                    "id": image_id,
                    "master_product_id": master_product_id,
                    "image_url": image_url,
                    "kind": kind
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database Error: {}", e)})),
        )
            .into_response(),
    }
}

#[cfg(feature = "vision")]
async fn update_product_image(
    State(state): State<AppState>,
    _: AdminOnly,
    Path((_master_product_id, image_id)): Path<(i64, i64)>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let store = VisionStore::new(state.db.clone());
    let old = match store.get_master_product_image(image_id).await {
        Ok(Some(item)) => item,
        Ok(None) => return (StatusCode::NOT_FOUND, "Image not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response(),
    };

    let mut new_image_url = old.image_url.clone();
    let mut new_kind = old.kind.clone();
    let mut image_replaced = false;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "image" {
            match save_upload_file(&state.upload_dir, field, Some("products")).await {
                Ok(path) => {
                    new_image_url = path;
                    image_replaced = true;
                }
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e})))
                        .into_response();
                }
            }
        } else if field_name == "kind" {
            let value = field.text().await.unwrap_or_default();
            if !value.is_empty() {
                new_kind = value;
            }
        }
    }

    if let Err(e) = store
        .update_master_product_image(image_id, &new_image_url, &new_kind)
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database Error: {}", e)})),
        )
            .into_response();
    }

    if image_replaced {
        let _ = delete_file(&state.upload_dir, &old.image_url).await;
    }

    let _ = store.delete_embeddings_by_image_id(image_id).await;
    state.vision_runtime.clone().start_incremental_for_images(
        state.db.clone(),
        state.upload_dir.clone(),
        vec![image_id],
    );

    (
        StatusCode::OK,
        Json(json!({
            "id": image_id,
            "master_product_id": old.master_product_id,
            "image_url": new_image_url,
            "kind": new_kind
        })),
    )
        .into_response()
}

#[cfg(feature = "vision")]
async fn delete_product_image(
    State(state): State<AppState>,
    _: AdminOnly,
    Path((_master_product_id, image_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let store = VisionStore::new(state.db.clone());
    let old = match store.get_master_product_image(image_id).await {
        Ok(Some(item)) => item,
        Ok(None) => return (StatusCode::NOT_FOUND, "Image not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response(),
    };

    if let Err(e) = store.delete_embeddings_by_image_id(image_id).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to delete embedding: {}", e)})),
        )
            .into_response();
    }

    if let Err(e) = store.delete_master_product_image(image_id).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database Error: {}", e)})),
        )
            .into_response();
    }

    let _ = delete_file(&state.upload_dir, &old.image_url).await;

    (
        StatusCode::OK,
        Json(json!({"ok": true, "deleted_image_id": image_id})),
    )
        .into_response()
}
