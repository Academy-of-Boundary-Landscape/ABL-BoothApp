use axum::{
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Local;
use sqlx::{query, query_as};
use std::io::{Cursor, Read, Write};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

// 引入你的项目模块
use crate::{api::guard::AdminOnly, db::models::MasterProduct, state::AppState};

use axum::extract::DefaultBodyLimit;


pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync/export-products", get(export_products))
        .route("/sync/import-products", post(import_products))
        .layer(DefaultBodyLimit::max(1000 * 1024 * 1024))
}

// ==========================================
// 1. 导出制品包 (Export)
// ==========================================
// 生成的 ZIP 结构:
// - catalog.json (数据库数据)
// - products/xxx.jpg (图片资源，相对于 uploads 的路径)
async fn export_products(
    State(state): State<AppState>,
    _: AdminOnly, // 仅管理员可操作
) -> impl IntoResponse {
    // 1. 从数据库获取所有制品信息
    let products = query_as::<_, MasterProduct>("SELECT * FROM master_products")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    // 2. 在内存中创建 ZIP
    let buf = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(buf));

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // 3. 写入 catalog.json
    // 使用 serde_json 将结构体转为漂亮的 JSON 字符串
    match serde_json::to_string_pretty(&products) {
        Ok(json_str) => {
            if let Err(e) = zip.start_file("catalog.json", options) {
                eprintln!("ZIP start file error: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to create zip entry",
                )
                    .into_response();
            }
            if let Err(e) = zip.write_all(json_str.as_bytes()) {
                eprintln!("ZIP write json error: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to write json to zip",
                )
                    .into_response();
            }
        }
        Err(e) => {
            eprintln!("JSON serialization error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to serialize data",
            )
                .into_response();
        }
    }

    // 4. 遍历制品，写入关联的图片文件
    // 数据库存的是: "/uploads/products/uuid.jpg"
    // state.upload_dir 是 uploads 文件夹的物理路径
    for prod in &products {
        if let Some(image_url) = &prod.image_url {
            // 去掉前导的 "/uploads/" 来获取相对路径
            let relative_path = image_url
                .trim_start_matches("/uploads/")
                .trim_start_matches("uploads/");

            // 构建物理路径
            let physical_path = state.upload_dir.join(relative_path);

            if physical_path.exists() && physical_path.is_file() {
                // 读取物理文件（zip 库是同步的）
                match std::fs::read(&physical_path) {
                    Ok(file_bytes) => {
                        // ZIP 内使用简洁路径: products/xxx.jpg
                        // 确保使用正斜杠（ZIP 标准）
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
            } else {
                eprintln!("Image file not found: {:?}", physical_path);
            }
        }
    }

    // 5. 完成 ZIP 构建
    let cursor = match zip.finish() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ZIP finish error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to finalize zip").into_response();
        }
    };

    // 从 Cursor 中提取 Vec<u8>
    let buf = cursor.into_inner();

    // 6. 返回文件流
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
    // 1. 获取上传的文件
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        // 前端 FormData append 的 key 建议叫 'file'
        if field.name() == Some("file") {
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("Upload error: {}", e))
                        .into_response()
                }
            };

            // 2. 解析 ZIP
            let reader = Cursor::new(data);
            let mut archive = match ZipArchive::new(reader) {
                Ok(a) => a,
                Err(_) => {
                    return (StatusCode::BAD_REQUEST, "Invalid ZIP/Boothpack file").into_response()
                }
            };

            // 3. 开启数据库事务 (原子性：要么全成功，要么全失败)
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

            // 4. 读取 catalog.json 数据
            let mut json_content = String::new();
            match archive.by_name("catalog.json") {
                Ok(mut file) => {
                    if let Err(_) = file.read_to_string(&mut json_content) {
                        return (StatusCode::BAD_REQUEST, "Failed to read catalog.json")
                            .into_response();
                    }
                }
                Err(_) => {
                    return (StatusCode::BAD_REQUEST, "Missing catalog.json in package")
                        .into_response()
                }
            };

            let imported_products: Vec<MasterProduct> = match serde_json::from_str(&json_content) {
                Ok(p) => p,
                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("JSON Parse Error: {}", e))
                        .into_response()
                }
            };

            // 5. 解压图片文件到物理硬盘
            // 遍历 ZIP 中的所有文件
            for i in 0..archive.len() {
                let mut file = match archive.by_index(i) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Failed to read zip entry {}: {}", i, e);
                        continue;
                    }
                };
                let file_path_str = file.name().to_string();

                // 跳过 json 和文件夹
                if file_path_str == "catalog.json" || file_path_str.ends_with('/') {
                    continue;
                }

                // 安全检查：防止 Zip Slip 漏洞
                if file_path_str.contains("..") {
                    eprintln!("Security: rejected path with ..: {}", file_path_str);
                    continue;
                }

                // ZIP 内的路径是: products/xxx.jpg
                // 目标物理路径: state.upload_dir/products/xxx.jpg
                let target_path = state.upload_dir.join(&file_path_str);

                // 确保父目录存在
                if let Some(parent) = target_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!("Failed to create dir {:?}: {}", parent, e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to create directory: {}", e),
                        )
                            .into_response();
                    }
                }

                // 写入文件（使用 std::fs 因为 zip 库是同步的）
                match std::fs::File::create(&target_path) {
                    Ok(mut outfile) => {
                        if let Err(e) = std::io::copy(&mut file, &mut outfile) {
                            eprintln!("Failed to extract file {}: {}", file_path_str, e);
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                format!("Failed to extract file: {}", e),
                            )
                                .into_response();
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to create file {:?}: {}", target_path, e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to create file: {}", e),
                        )
                            .into_response();
                    }
                }
            }

            // 6. 数据库 Upsert (插入或更新)
            let products_count = imported_products.len();
            for prod in imported_products {
                // SQLite Upsert 语法
                // 如果 product_code 冲突，则覆盖更新
                let sql = r#"
                    INSERT INTO master_products (product_code, name, default_price, category, image_url, is_active)
                    VALUES (?, ?, ?, ?, ?, ?)
                    ON CONFLICT(product_code) DO UPDATE SET
                        name = excluded.name,
                        default_price = excluded.default_price,
                        category = excluded.category,
                        image_url = excluded.image_url,
                        is_active = excluded.is_active
                "#;

                // 确保 image_url 使用正确的格式（/uploads/...）
                // 导入的数据应该保持原有的 /uploads/ 前缀
                let res = query(sql)
                    .bind(&prod.product_code)
                    .bind(&prod.name)
                    .bind(&prod.default_price)
                    .bind(&prod.category)
                    .bind(&prod.image_url)
                    .bind(&prod.is_active)
                    .execute(&mut *tx)
                    .await;

                if let Err(e) = res {
                    // 数据库写入失败，事务会自动回滚
                    eprintln!("Import DB Error for {}: {}", prod.product_code, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("DB Write Failed: {}", e),
                    )
                        .into_response();
                }
            }

            // 7. 提交事务
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
                    "count": products_count
                })),
            )
                .into_response();
        }
    }

    (StatusCode::BAD_REQUEST, "No file found in request").into_response()
}
