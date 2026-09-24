use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;
#[cfg(feature = "vision")]
use serde::Serialize;
#[cfg(feature = "vision")]
use serde_json::json;
use sqlx::query_as;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

#[cfg(feature = "vision")]
use crate::vision::store::VisionStore;
use crate::{
    api::guard::AdminOnly,
    api::openapi::ApiErrorBody,
    db::models::MasterProduct,
    error::{ApiError, ApiResult},
    state::AppState,
    utils::file::{delete_file, save_upload_file},
};

const PRODUCT_UPLOAD_LIMIT_BYTES: usize = 10 * 1024 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    let router = OpenApiRouter::new()
        .routes(routes!(list_products, create_product))
        .routes(routes!(update_product))
        .routes(routes!(update_status));

    #[cfg(feature = "vision")]
    let router = router
        .routes(routes!(list_product_images, add_product_image))
        .routes(routes!(update_product_image, delete_product_image));

    // layer 放在所有路由之后，确保覆盖全部路由（包括 vision images）
    router.layer(DefaultBodyLimit::max(PRODUCT_UPLOAD_LIMIT_BYTES))
}

#[cfg(feature = "vision")]
#[derive(Debug, Serialize, ToSchema)]
#[schema(as = MasterProductImageDto)]
struct ProductImageDto {
    id: i64,
    master_product_id: i64,
    image_url: String,
    kind: String,
    created_at: String,
    has_embedding: bool,
}

/// 列出某个全局商品的全部识别图。
#[cfg(feature = "vision")]
#[utoipa::path(
    get,
    path = "/{id}/images",
    tag = "master_product",
    params(("id" = i64, Path, description = "商品 id")),
    responses(
        (status = 200, body = Vec<ProductImageDto>),
        (status = 500, content_type = "text/plain", body = String, description = "数据库错误（纯文本）"),
    ),
)]
async fn list_product_images(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
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

#[derive(Deserialize, IntoParams)]
struct ListQuery {
    all: Option<bool>,
}

/// 仅用于 OpenAPI 文档：`create_product` 的 multipart 表单字段。
#[derive(ToSchema)]
#[allow(dead_code)]
struct CreateMasterProductForm {
    product_code: String,
    name: String,
    /// 元，不是分（老字段，`f64`）。
    default_price: String,
    category: Option<String>,
    tags: String,
    #[schema(value_type = Option<String>, format = Binary)]
    image: Option<Vec<u8>>,
}

/// 仅用于 OpenAPI 文档：`update_product` 的 multipart 表单字段。
#[derive(ToSchema)]
#[allow(dead_code)]
struct UpdateMasterProductForm {
    product_code: Option<String>,
    name: Option<String>,
    /// 元，不是分（老字段，`f64`）。
    default_price: Option<String>,
    category: Option<String>,
    tags: Option<String>,
    remove_image: Option<bool>,
    #[schema(value_type = Option<String>, format = Binary)]
    image: Option<Vec<u8>>,
}

/// 全局商品库列表。默认只列上架商品，`?all=true` 连停用的一起返回。
#[utoipa::path(
    get,
    path = "/",
    tag = "master_product",
    params(ListQuery),
    responses((status = 200, body = Vec<MasterProduct>)),
)]
async fn list_products(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Json<Vec<MasterProduct>> {
    let show_all = params.all.unwrap_or(false);

    // LEFT JOIN master_product_images 得到每个商品的识别图数量（image_count）。
    // 用 idx_mp_images_product_id 索引保证性能；mp.* 必须放在 SELECT 前列，
    // 否则 sqlx 的 FromRow 可能按位置错位赋值。
    let sql = if show_all {
        r#"
        SELECT mp.*, COUNT(mpi.id) AS image_count
        FROM master_products mp
        LEFT JOIN master_product_images mpi ON mpi.master_product_id = mp.id
        GROUP BY mp.id
        ORDER BY mp.product_code ASC
        "#
    } else {
        r#"
        SELECT mp.*, COUNT(mpi.id) AS image_count
        FROM master_products mp
        LEFT JOIN master_product_images mpi ON mpi.master_product_id = mp.id
        WHERE mp.is_active = 1
        GROUP BY mp.id
        ORDER BY mp.product_code ASC
        "#
    };

    let products: Vec<MasterProduct> = query_as::<_, MasterProduct>(sql)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    Json(products)
}

/// 新建一个全局商品（multipart 表单，`image` 为可选文件字段）。需要管理员。
#[utoipa::path(
    post,
    path = "/",
    tag = "master_product",
    request_body(content = CreateMasterProductForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 201, body = MasterProduct, description = "创建成功"),
        (status = 400, body = ApiErrorBody, description = "缺少 product_code 或 name"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 409, body = ApiErrorBody, description = "product_code 已存在"),
        (status = 500, content_type = "text/plain", body = String, description = "上传或数据库错误（纯文本）"),
    ),
)]
async fn create_product(
    State(state): State<AppState>,
    _: AdminOnly,
    mut multipart: Multipart,
) -> ApiResult<Response> {
    // 不需要展会守卫：全局商品库，不属于任何展会；选的货到选品时才写 event_products
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
                    return Ok(
                        (StatusCode::INTERNAL_SERVER_ERROR, "File upload failed").into_response()
                    );
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
        return Err(ApiError::BadRequest("Code and Name are required".into()));
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
        Ok(product) => Ok((StatusCode::CREATED, Json(product)).into_response()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") {
                Err(ApiError::Conflict("Product code already exists".into()))
            } else {
                eprintln!("DB Error: {:?}", e);
                Ok((StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response())
            }
        }
    }
}

/// 修改一个全局商品（multipart 表单；为兼容旧前端同时挂 POST 与 PUT）。需要管理员。
#[utoipa::path(
    method(post, put),
    path = "/{id}",
    tag = "master_product",
    params(("id" = i64, Path, description = "商品 id")),
    request_body(content = UpdateMasterProductForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 200, body = MasterProduct, description = "更新成功"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, content_type = "text/plain", body = String, description = "商品不存在（纯文本）"),
        (status = 409, body = ApiErrorBody, description = "product_code 已存在"),
        (status = 500, content_type = "text/plain", body = String, description = "数据库错误（纯文本）"),
    ),
)]
async fn update_product(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> ApiResult<Response> {
    // 不需要展会守卫：全局商品库，不属于任何展会；已有展会的账靠 event_products 快照
    let old_product: MasterProduct = match query_as("SELECT * FROM master_products WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    {
        Some(p) => p,
        None => return Ok((StatusCode::NOT_FOUND, "Product not found").into_response()),
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
                "remove_image" if value == "true" => {
                    should_remove_image = true;
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
        Ok(product) => Ok((StatusCode::OK, Json(product)).into_response()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("UNIQUE constraint failed") {
                Err(ApiError::Conflict("Product code already exists".into()))
            } else {
                Ok((StatusCode::INTERNAL_SERVER_ERROR, "Database Error").into_response())
            }
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[schema(as = MasterProductUpdateStatusRequest)]
struct UpdateStatusRequest {
    is_active: bool,
}

/// 上架 / 下架一个全局商品。需要管理员。
#[utoipa::path(
    put,
    path = "/{id}/status",
    tag = "master_product",
    params(("id" = i64, Path, description = "商品 id")),
    request_body = UpdateStatusRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, body = MasterProduct, description = "更新成功"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, content_type = "text/plain", body = String, description = "数据库错误（纯文本）"),
    ),
)]
async fn update_status(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateStatusRequest>,
) -> Response {
    // 不需要展会守卫：全局商品库的上下架，不属于任何展会
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

/// 仅用于 OpenAPI 文档：`add_product_image` 的 multipart 表单字段。
#[cfg(feature = "vision")]
#[derive(ToSchema)]
#[allow(dead_code)]
struct AddProductImageForm {
    #[schema(value_type = Option<String>, format = Binary)]
    image: Option<Vec<u8>>,
    kind: Option<String>,
}

/// 仅用于 OpenAPI 文档：`update_product_image` 的 multipart 表单字段。
#[cfg(feature = "vision")]
#[derive(ToSchema)]
#[allow(dead_code)]
struct UpdateProductImageForm {
    #[schema(value_type = Option<String>, format = Binary)]
    image: Option<Vec<u8>>,
    kind: Option<String>,
}

/// 新增 / 修改识别图后返回的字段。
#[cfg(feature = "vision")]
#[derive(Serialize, ToSchema)]
#[schema(as = MasterProductImageResponse)]
struct ProductImageResponse {
    id: i64,
    master_product_id: i64,
    image_url: String,
    kind: String,
}

/// 删除识别图的响应。
#[cfg(feature = "vision")]
#[derive(Serialize, ToSchema)]
#[schema(as = DeleteMasterProductImageResponse)]
struct DeleteProductImageResponse {
    ok: bool,
    deleted_image_id: i64,
}

/// 给全局商品加一张识别图（multipart，`image` 为必填文件字段）。需要管理员。
#[cfg(feature = "vision")]
#[utoipa::path(
    post,
    path = "/{id}/images",
    tag = "master_product",
    params(("id" = i64, Path, description = "商品 id")),
    request_body(content = AddProductImageForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 201, body = ProductImageResponse, description = "添加成功"),
        (status = 400, body = ApiErrorBody, description = "缺少 image 字段"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, body = ApiErrorBody, description = "上传或数据库错误"),
    ),
)]
async fn add_product_image(
    State(state): State<AppState>,
    _: AdminOnly,
    Path(master_product_id): Path<i64>,
    mut multipart: Multipart,
) -> ApiResult<Response> {
    // 不需要展会守卫：全局商品库的识别图片，不属于任何展会
    let mut image_url: Option<String> = None;
    let mut kind = "gallery".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "image" {
            match save_upload_file(&state.upload_dir, field, Some("products")).await {
                Ok(path) => image_url = Some(path),
                Err(e) => {
                    return Ok(
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e})))
                            .into_response(),
                    );
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
        None => return Err(ApiError::BadRequest("image is required".into())),
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

            Ok((
                StatusCode::CREATED,
                Json(ProductImageResponse {
                    id: image_id,
                    master_product_id,
                    image_url,
                    kind,
                }),
            )
                .into_response())
        }
        Err(e) => Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Database Error: {}", e)})),
        )
            .into_response()),
    }
}

/// 修改一张识别图（multipart；为兼容旧前端同时挂 POST 与 PUT）。需要管理员。
#[cfg(feature = "vision")]
#[utoipa::path(
    method(post, put),
    path = "/{id}/images/{image_id}",
    tag = "master_product",
    params(
        ("id" = i64, Path, description = "商品 id"),
        ("image_id" = i64, Path, description = "识别图 id"),
    ),
    request_body(content = UpdateProductImageForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 200, body = ProductImageResponse, description = "更新成功"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, content_type = "text/plain", body = String, description = "识别图不存在（纯文本）"),
        (status = 500, body = ApiErrorBody, description = "上传或数据库错误"),
    ),
)]
async fn update_product_image(
    State(state): State<AppState>,
    _: AdminOnly,
    Path((_master_product_id, image_id)): Path<(i64, i64)>,
    mut multipart: Multipart,
) -> Response {
    // 不需要展会守卫：全局商品库的识别图片，不属于任何展会
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
        Json(ProductImageResponse {
            id: image_id,
            master_product_id: old.master_product_id,
            image_url: new_image_url,
            kind: new_kind,
        }),
    )
        .into_response()
}

/// 删除一张识别图。需要管理员。
#[cfg(feature = "vision")]
#[utoipa::path(
    delete,
    path = "/{id}/images/{image_id}",
    tag = "master_product",
    params(
        ("id" = i64, Path, description = "商品 id"),
        ("image_id" = i64, Path, description = "识别图 id"),
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, body = DeleteProductImageResponse, description = "删除成功"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, content_type = "text/plain", body = String, description = "识别图不存在（纯文本）"),
        (status = 500, body = ApiErrorBody, description = "数据库错误"),
    ),
)]
async fn delete_product_image(
    State(state): State<AppState>,
    _: AdminOnly,
    Path((_master_product_id, image_id)): Path<(i64, i64)>,
) -> Response {
    // 不需要展会守卫：全局商品库的识别图片，不属于任何展会
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
        Json(DeleteProductImageResponse {
            ok: true,
            deleted_image_id: image_id,
        }),
    )
        .into_response()
}

/// ③b 形状快照：钉住每个路由的 JSON 形状（键 + 类型），类型化前后必须一行不改照样绿。
///
/// 本模块是 v1.1 时代的老代码：`json!` 现拼响应、`unwrap_or_default()` 吞错误多，
/// 所以这里逐条断言，连错误分支的非标准错误体（纯文本）也一并钉住。
#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        admin_token, json_request, read_json, shape_of, test_router_with, vendor_token,
    };
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    const BOUNDARY: &str = "X-BOUNDARY";

    /// 造两个全局商品：P1 字段齐全且带一张识别图，P2 只有必填项。
    /// Option 字段因此能同时出现 null 与非 null，`image_count` 也能出现非零。
    async fn seeded() -> (Router, tempfile::TempDir, i64, i64) {
        let (router, dir, _pool) = test_router_with().await;
        let token = admin_token();

        let (s, p1) = multipart_call(
            &router,
            "POST",
            "/api/master-products",
            Some(&token),
            &[
                ("product_code", "P1", None),
                ("name", "本子甲", None),
                ("default_price", "12.5", None),
                ("category", "本子", None),
                ("tags", "红,蓝", None),
                ("image", "fake-png", Some("a.png")),
            ],
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        let product_id = p1["id"].as_i64().unwrap();

        let (s, _p2) = multipart_call(
            &router,
            "POST",
            "/api/master-products",
            Some(&token),
            &[("product_code", "P2", None), ("name", "本子乙", None)],
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);

        #[cfg(feature = "vision")]
        let image_id = {
            let (s, img) = multipart_call(
                &router,
                "POST",
                &format!("/api/master-products/{product_id}/images"),
                Some(&token),
                &[
                    ("image", "fake-img", Some("g.png")),
                    ("kind", "gallery", None),
                ],
            )
            .await;
            assert_eq!(s, StatusCode::CREATED);
            img["id"].as_i64().unwrap()
        };
        #[cfg(not(feature = "vision"))]
        let image_id = 0;

        (router, dir, product_id, image_id)
    }

    fn multipart_request(
        method: &str,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str, Option<&str>)],
    ) -> Request<Body> {
        let mut body = String::new();
        for (name, value, filename) in fields {
            body.push_str(&format!("--{BOUNDARY}\r\n"));
            match filename {
                Some(f) => body.push_str(&format!(
                    "Content-Disposition: form-data; name=\"{name}\"; filename=\"{f}\"\r\n\r\n{value}\r\n"
                )),
                None => body.push_str(&format!(
                    "Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                )),
            }
        }
        body.push_str(&format!("--{BOUNDARY}--\r\n"));
        let mut b = Request::builder().method(method).uri(uri).header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );
        if let Some(t) = token {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        b.body(Body::from(body)).unwrap()
    }

    async fn multipart_call(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str, Option<&str>)],
    ) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(multipart_request(method, uri, token, fields))
            .await
            .unwrap();
        let status = res.status();
        (status, read_json(res).await)
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

    /// 非 JSON（纯文本 / 原样）错误体：直接读字节，`read_json` 会炸。
    async fn call_text(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, Vec<u8>) {
        let res = router
            .clone()
            .oneshot(json_request(method, uri, token, body))
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, bytes.to_vec())
    }

    async fn multipart_call_text(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str, Option<&str>)],
    ) -> (StatusCode, Vec<u8>) {
        let res = router
            .clone()
            .oneshot(multipart_request(method, uri, token, fields))
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, bytes.to_vec())
    }

    /// `list_products` 里的商品：`image_count` 是 COUNT，恒为整数。
    fn product_shape() -> Value {
        json!({
            "id": "int",
            "product_code": "string",
            "name": "string",
            "default_price": "float",
            "image_url": "null|string",
            "category": "null|string",
            "is_active": "bool",
            "tags": "string",
            "image_count": "int",
            "owner_society_id": "int",
        })
    }

    /// 单个 `RETURNING *` 返回的商品：表里没有 `image_count`，默认 None ⇒ null。
    fn returned_product_shape(image_url: &str, category: &str, tags: &str) -> Value {
        json!({
            "id": "int",
            "product_code": "string",
            "name": "string",
            "default_price": "float",
            "image_url": image_url,
            "category": category,
            "is_active": "bool",
            "tags": tags,
            "image_count": "null",
            "owner_society_id": "int",
        })
    }

    #[tokio::test]
    async fn shape_list_products() {
        let (router, _dir, _, _) = seeded().await;
        let (s, body) = call(&router, "GET", "/api/master-products", None, json!(null)).await;
        println!("shape_list_products = {}", shape_of(&body));
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!([product_shape()]));
        // `?all=true` 走另一条 SQL 分支，形状必须一致
        let (s, body) = call(
            &router,
            "GET",
            "/api/master-products?all=true",
            None,
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(shape_of(&body), json!([product_shape()]));
    }

    #[tokio::test]
    async fn shape_create_product() {
        let (router, _dir, _, _) = seeded().await;
        let t = admin_token();

        let (s, body) = multipart_call(
            &router,
            "POST",
            "/api/master-products",
            Some(&t),
            &[
                ("product_code", "P3", None),
                ("name", "本子丙", None),
                ("default_price", "9.9", None),
                ("category", "挂件", None),
                ("tags", "丙", None),
                ("image", "fake-png", Some("c.png")),
            ],
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(
            shape_of(&body),
            returned_product_shape("string", "string", "string")
        );

        // 缺 product_code / name ⇒ 400 {"error": "..."}
        let (s, body) = multipart_call(
            &router,
            "POST",
            "/api/master-products",
            Some(&t),
            &[("name", "只有名字", None)],
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );

        // 重复的 product_code ⇒ 409
        let (s, body) = multipart_call(
            &router,
            "POST",
            "/api/master-products",
            Some(&t),
            &[("product_code", "P1", None), ("name", "撞码", None)],
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::CONFLICT, json!({"error": "string"}))
        );

        // 未登录 ⇒ 401；非管理员 ⇒ 403
        let (s, body) = multipart_call(
            &router,
            "POST",
            "/api/master-products",
            None,
            &[("product_code", "P9", None), ("name", "x", None)],
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );
        let (s, body) = multipart_call(
            &router,
            "POST",
            "/api/master-products",
            Some(&vendor_token(1)),
            &[("product_code", "P9", None), ("name", "x", None)],
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::FORBIDDEN, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_update_product() {
        let (router, _dir, product_id, _) = seeded().await;
        let t = admin_token();

        // POST 与 PUT 都挂到同一个 handler 上
        for method in ["POST", "PUT"] {
            let (s, body) = multipart_call(
                &router,
                method,
                &format!("/api/master-products/{product_id}"),
                Some(&t),
                &[("name", "本子甲（改）", None)],
            )
            .await;
            assert_eq!(s, StatusCode::OK, "{method}");
            assert_eq!(
                shape_of(&body),
                returned_product_shape("string", "string", "string"),
                "{method}"
            );
        }

        // 撞到别人的 product_code ⇒ 409
        let (s, body) = multipart_call(
            &router,
            "POST",
            &format!("/api/master-products/{product_id}"),
            Some(&t),
            &[("product_code", "P2", None)],
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::CONFLICT, json!({"error": "string"}))
        );

        // 不存在的商品 ⇒ 404 纯文本
        let (s, bytes) = multipart_call_text(
            &router,
            "POST",
            "/api/master-products/99999",
            Some(&t),
            &[("name", "x", None)],
        )
        .await;
        assert_eq!(
            (s, bytes.as_slice()),
            (StatusCode::NOT_FOUND, b"Product not found".as_slice())
        );
    }

    #[tokio::test]
    async fn shape_update_status() {
        let (router, _dir, product_id, _) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "PUT",
            &format!("/api/master-products/{product_id}/status"),
            Some(&t),
            json!({"is_active": false}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            returned_product_shape("string", "string", "string")
        );

        // 不存在的 id：fetch_one 报 RowNotFound，吞成 500 纯文本
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                "/api/master-products/99999/status",
                Some(&t),
                json!({"is_active": false}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(bytes.as_ref(), b"Database Error");
    }

    #[cfg(feature = "vision")]
    #[tokio::test]
    async fn shape_list_product_images() {
        let (router, _dir, product_id, _) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            &format!("/api/master-products/{product_id}/images"),
            Some(&admin_token()),
            json!(null),
        )
        .await;
        println!("shape_list_product_images = {}", shape_of(&body));
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!([{
                "id": "int",
                "master_product_id": "int",
                "image_url": "string",
                "kind": "string",
                "created_at": "string",
                "has_embedding": "bool",
            }])
        );
    }

    #[cfg(feature = "vision")]
    #[tokio::test]
    async fn shape_add_product_image() {
        let (router, _dir, product_id, _) = seeded().await;
        let t = admin_token();

        let (s, body) = multipart_call(
            &router,
            "POST",
            &format!("/api/master-products/{product_id}/images"),
            Some(&t),
            &[
                ("image", "fake-img", Some("x.png")),
                ("kind", "reference", None),
            ],
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        assert_eq!(
            shape_of(&body),
            json!({
                "id": "int",
                "master_product_id": "int",
                "image_url": "string",
                "kind": "string",
            })
        );

        // 没有 image 字段 ⇒ 400
        let (s, body) = multipart_call(
            &router,
            "POST",
            &format!("/api/master-products/{product_id}/images"),
            Some(&t),
            &[("kind", "gallery", None)],
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::BAD_REQUEST, json!({"error": "string"}))
        );
    }

    #[cfg(feature = "vision")]
    #[tokio::test]
    async fn shape_update_product_image() {
        let (router, _dir, product_id, image_id) = seeded().await;
        let t = admin_token();

        for method in ["POST", "PUT"] {
            let (s, body) = multipart_call(
                &router,
                method,
                &format!("/api/master-products/{product_id}/images/{image_id}"),
                Some(&t),
                &[("kind", "reference", None)],
            )
            .await;
            assert_eq!(s, StatusCode::OK, "{method}");
            assert_eq!(
                shape_of(&body),
                json!({
                    "id": "int",
                    "master_product_id": "int",
                    "image_url": "string",
                    "kind": "string",
                }),
                "{method}"
            );
        }

        // 不存在的图片 ⇒ 404 纯文本
        let (s, bytes) = multipart_call_text(
            &router,
            "POST",
            &format!("/api/master-products/{product_id}/images/99999"),
            Some(&t),
            &[("kind", "gallery", None)],
        )
        .await;
        assert_eq!(
            (s, bytes.as_slice()),
            (StatusCode::NOT_FOUND, b"Image not found".as_slice())
        );
    }

    #[cfg(feature = "vision")]
    #[tokio::test]
    async fn shape_delete_product_image() {
        let (router, _dir, product_id, image_id) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "DELETE",
            &format!("/api/master-products/{product_id}/images/{image_id}"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({"ok": "bool", "deleted_image_id": "int"})
        );

        // 再删一次 ⇒ 404 纯文本
        let (s, bytes) = call_text(
            &router,
            "DELETE",
            &format!("/api/master-products/{product_id}/images/{image_id}"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, bytes.as_slice()),
            (StatusCode::NOT_FOUND, b"Image not found".as_slice())
        );
    }
}
