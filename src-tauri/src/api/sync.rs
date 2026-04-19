use axum::{
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, FromRow};
use std::io::{Cursor, Read, Write};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::{api::guard::AdminOnly, db::models::MasterProduct, state::AppState};

use axum::extract::DefaultBodyLimit;

const SYNC_IMPORT_LIMIT_BYTES: usize = 1000 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync/export-products", get(export_products))
        .route("/sync/import-products", post(import_products))
        .layer(DefaultBodyLimit::max(SYNC_IMPORT_LIMIT_BYTES))
}

/// 识别用图片的导出/导入结构（用 product_code 关联，而非 id，确保跨设备导入正确）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct ProductImageExport {
    product_code: String,
    image_url: String,
    kind: String,
}

/// 完整的导出数据结构
#[derive(Debug, Serialize, Deserialize)]
struct CatalogExport {
    products: Vec<MasterProduct>,
    /// AI 识别用图片列表（v1.1+ 新增，导入时向后兼容缺失的情况）
    #[serde(default)]
    product_images: Vec<ProductImageExport>,
}

// ==========================================
// 1. 导出制品包 (Export)
// ==========================================
// ZIP 结构:
// - catalog.json       (商品数据 + 识别用图片元数据)
// - products/xxx.jpg   (商品缩略图)
// - vision/xxx.jpg     (AI 识别用图片)
async fn export_products(
    State(state): State<AppState>,
    _: AdminOnly,
) -> impl IntoResponse {
    // 1. 获取所有商品
    let products = query_as::<_, MasterProduct>("SELECT * FROM master_products")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    // 2. 获取所有识别用图片（排除 legacy_main 和 feedback_incorrect）
    let product_images = query_as::<_, ProductImageExport>(
        r#"
        SELECT mp.product_code, mpi.image_url, mpi.kind
        FROM master_product_images mpi
        JOIN master_products mp ON mp.id = mpi.master_product_id
        WHERE mpi.kind NOT IN ('legacy_main', 'feedback_incorrect')
        ORDER BY mp.product_code, mpi.id
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // 3. 构建 catalog
    let catalog = CatalogExport {
        products,
        product_images,
    };

    // 4. 创建 ZIP
    let buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(buf));
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // 5. 写入 catalog.json
    match serde_json::to_string_pretty(&catalog) {
        Ok(json_str) => {
            if let Err(e) = zip.start_file("catalog.json", options) {
                eprintln!("ZIP start file error: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create zip entry")
                    .into_response();
            }
            if let Err(e) = zip.write_all(json_str.as_bytes()) {
                eprintln!("ZIP write json error: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write json to zip")
                    .into_response();
            }
        }
        Err(e) => {
            eprintln!("JSON serialization error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize data")
                .into_response();
        }
    }

    // 6. 收集所有需要打包的图片路径
    let mut image_paths: Vec<String> = Vec::new();

    for prod in &catalog.products {
        if let Some(url) = &prod.image_url {
            image_paths.push(url.clone());
        }
    }
    for img in &catalog.product_images {
        image_paths.push(img.image_url.clone());
    }

    // 7. 写入所有图片文件
    for image_url in &image_paths {
        let relative_path = image_url
            .trim_start_matches("/uploads/")
            .trim_start_matches("uploads/");

        let physical_path = state.upload_dir.join(relative_path);

        if physical_path.exists() && physical_path.is_file() {
            match std::fs::read(&physical_path) {
                Ok(file_bytes) => {
                    let zip_path = relative_path.replace('\\', "/");
                    if let Err(e) = zip.start_file(&zip_path, options) {
                        eprintln!("Failed to add {} to zip: {}", zip_path, e);
                        continue;
                    }
                    if let Err(e) = zip.write_all(&file_bytes) {
                        eprintln!("Failed to write {} to zip: {}", zip_path, e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read image file {:?}: {}", physical_path, e);
                }
            }
        }
    }

    // 8. 完成 ZIP
    let cursor = match zip.finish() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ZIP finish error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to finalize zip").into_response();
        }
    };

    let buf = cursor.into_inner();
    let filename = format!(
        "booth_catalog_{}.boothpack",
        Local::now().format("%Y%m%d_%H%M")
    );
    let disposition = format!("attachment; filename=\"{}\"", filename);

    (
        [
            (header::CONTENT_TYPE, "application/zip"),
            (header::CONTENT_DISPOSITION, disposition.as_str()),
        ],
        buf,
    )
        .into_response()
}

// ==========================================
// 2. 导入制品包 (Import)
// ==========================================
async fn import_products(
    State(state): State<AppState>,
    _: AdminOnly,
    mut multipart: Multipart,
) -> impl IntoResponse {
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name() == Some("file") {
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("Upload error: {}", e))
                        .into_response()
                }
            };

            // ── 1. 解析 ZIP + catalog（纯内存操作，放在 spawn_blocking 里避免阻塞 tokio） ──
            let upload_dir = state.upload_dir.clone();
            let parsed = tokio::task::spawn_blocking(move || {
                let reader = Cursor::new(data);
                let mut archive = match ZipArchive::new(reader) {
                    Ok(a) => a,
                    Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid ZIP/Boothpack file".to_string())),
                };

                // 读取 catalog.json
                let mut json_content = String::new();
                match archive.by_name("catalog.json") {
                    Ok(mut file) => {
                        if file.read_to_string(&mut json_content).is_err() {
                            return Err((StatusCode::BAD_REQUEST, "Failed to read catalog.json".to_string()));
                        }
                    }
                    Err(_) => return Err((StatusCode::BAD_REQUEST, "Missing catalog.json in package".to_string())),
                };

                // 向后兼容：尝试解析新格式（CatalogExport），回退到旧格式（Vec<MasterProduct>）
                let catalog: CatalogExport =
                    if let Ok(c) = serde_json::from_str::<CatalogExport>(&json_content) {
                        c
                    } else if let Ok(products) =
                        serde_json::from_str::<Vec<MasterProduct>>(&json_content)
                    {
                        CatalogExport {
                            products,
                            product_images: vec![],
                        }
                    } else {
                        return Err((StatusCode::BAD_REQUEST, "JSON Parse Error in catalog.json".to_string()));
                    };

                // ── 2. 解压所有图片文件（磁盘 I/O，在 blocking 线程池里执行） ──
                for i in 0..archive.len() {
                    let mut file = match archive.by_index(i) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Failed to read zip entry {}: {}", i, e);
                            continue;
                        }
                    };
                    let file_path_str = file.name().to_string();

                    if file_path_str == "catalog.json" || file_path_str.ends_with('/') {
                        continue;
                    }
                    if file_path_str.contains("..") {
                        eprintln!("Security: rejected path with ..: {}", file_path_str);
                        continue;
                    }

                    let target_path = upload_dir.join(&file_path_str);
                    if let Some(parent) = target_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!("Failed to create dir {:?}: {}", parent, e);
                            continue;
                        }
                    }
                    match std::fs::File::create(&target_path) {
                        Ok(mut outfile) => {
                            if let Err(e) = std::io::copy(&mut file, &mut outfile) {
                                eprintln!("Failed to extract file {}: {}", file_path_str, e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to create file {:?}: {}", target_path, e);
                        }
                    }
                }

                Ok(catalog)
            })
            .await;

            // 处理 spawn_blocking 的结果
            let catalog = match parsed {
                Ok(Ok(c)) => c,
                Ok(Err((status, msg))) => return (status, msg).into_response(),
                Err(e) => {
                    eprintln!("Import task panic: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Internal error during import".to_string()).into_response();
                }
            };

            // ── 3. 数据库写入（事务尽量短，只做 DB 操作） ──
            let mut tx = match state.db.begin().await {
                Ok(tx) => tx,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("DB Error: {}", e),
                    )
                        .into_response()
                }
            };

            // Upsert 商品数据
            let products_count = catalog.products.len();
            for prod in &catalog.products {
                let res = query(
                    r#"
                    INSERT INTO master_products (product_code, name, default_price, category, image_url, is_active, tags)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(product_code) DO UPDATE SET
                        name = excluded.name,
                        default_price = excluded.default_price,
                        category = excluded.category,
                        image_url = excluded.image_url,
                        is_active = excluded.is_active,
                        tags = excluded.tags
                    "#,
                )
                .bind(&prod.product_code)
                .bind(&prod.name)
                .bind(&prod.default_price)
                .bind(&prod.category)
                .bind(&prod.image_url)
                .bind(&prod.is_active)
                .bind(&prod.tags)
                .execute(&mut *tx)
                .await;

                if let Err(e) = res {
                    eprintln!("Import DB Error for {}: {}", prod.product_code, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("DB Write Failed: {}", e),
                    )
                        .into_response();
                }
            }

            // Upsert AI 识别用图片（通过 product_code 关联到本地商品 id）
            let images_count = catalog.product_images.len();
            for img in &catalog.product_images {
                let master_id: Option<(i64,)> = query_as(
                    "SELECT id FROM master_products WHERE product_code = ?",
                )
                .bind(&img.product_code)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);

                let Some((mid,)) = master_id else { continue };

                if let Err(e) = query(
                    r#"
                    INSERT INTO master_product_images (master_product_id, image_url, kind)
                    SELECT ?, ?, ?
                    WHERE NOT EXISTS (
                        SELECT 1 FROM master_product_images
                        WHERE master_product_id = ? AND image_url = ?
                    )
                    "#,
                )
                .bind(mid)
                .bind(&img.image_url)
                .bind(&img.kind)
                .bind(mid)
                .bind(&img.image_url)
                .execute(&mut *tx)
                .await
                {
                    eprintln!("Import image DB Error for {}: {}", img.image_url, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("DB Write Failed (image): {}", e),
                    )
                        .into_response();
                }
            }

            // 提交事务
            if let Err(e) = tx.commit().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Commit Failed: {}", e),
                )
                    .into_response();
            }

            return (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Import successful",
                    "products_count": products_count,
                    "images_count": images_count
                })),
            )
                .into_response();
        }
    }

    (StatusCode::BAD_REQUEST, "No file found in request").into_response()
}
