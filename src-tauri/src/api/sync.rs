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
use std::time::Instant;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::{api::guard::AdminOnly, db::models::MasterProduct, state::AppState};

// 日志前缀，方便从大堆 stdout/stderr 里 grep 出来。
// 用 eprintln! 是为了和现有错误日志风格一致；dev 与 release 都会写到 stderr。
// 用户朋友实际运行的是 release 构建，所以诊断日志对 release 也开启 —— 排查这次崩溃需要看到。
// 导入/导出是用户主动触发的低频操作，每次产生几十行日志可接受。
const TAG: &str = "[sync]";

macro_rules! dev_log {
    ($($arg:tt)*) => {
        eprintln!("{} {}", TAG, format_args!($($arg)*));
    };
}

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
    let t0 = Instant::now();
    eprintln!("{} export: start", TAG);

    // 1. 获取所有商品
    let products = match query_as::<_, MasterProduct>("SELECT * FROM master_products")
        .fetch_all(&state.db)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{} export: fetch master_products failed: {}", TAG, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load products").into_response();
        }
    };
    dev_log!("export: loaded {} master_products in {:?}", products.len(), t0.elapsed());

    // 2. 获取所有识别用图片（排除 legacy_main 和 feedback_incorrect）
    let product_images = match query_as::<_, ProductImageExport>(
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
    {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("{} export: fetch product_images failed: {}", TAG, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load product images")
                .into_response();
        }
    };
    dev_log!("export: loaded {} product_images", product_images.len());

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
    let mut images_written = 0usize;
    let mut images_missing = 0usize;
    let mut total_image_bytes: u64 = 0;
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
                        eprintln!("{} export: add {} to zip failed: {}", TAG, zip_path, e);
                        continue;
                    }
                    if let Err(e) = zip.write_all(&file_bytes) {
                        eprintln!("{} export: write {} to zip failed: {}", TAG, zip_path, e);
                    } else {
                        total_image_bytes += file_bytes.len() as u64;
                        images_written += 1;
                    }
                }
                Err(e) => {
                    eprintln!("{} export: read {:?} failed: {}", TAG, physical_path, e);
                    images_missing += 1;
                }
            }
        } else {
            dev_log!("export: skip missing image {:?}", physical_path);
            images_missing += 1;
        }
    }
    dev_log!(
        "export: zip body — images_written={} missing={} total_image_bytes={} elapsed={:?}",
        images_written, images_missing, total_image_bytes, t0.elapsed()
    );

    // 8. 完成 ZIP
    let cursor = match zip.finish() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ZIP finish error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to finalize zip").into_response();
        }
    };

    let buf = cursor.into_inner();
    eprintln!(
        "{} export: done — zip {} bytes, total elapsed {:?}",
        TAG,
        buf.len(),
        t0.elapsed()
    );
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
    let t0 = Instant::now();
    eprintln!("{} import: start", TAG);

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name() == Some("file") {
            let t_recv = Instant::now();
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{} import: receive bytes failed: {}", TAG, e);
                    return (StatusCode::BAD_REQUEST, format!("Upload error: {}", e))
                        .into_response();
                }
            };
            eprintln!(
                "{} import: received {} bytes in {:?}",
                TAG,
                data.len(),
                t_recv.elapsed()
            );

            // ── 1. 解析 ZIP + catalog（纯内存操作，放在 spawn_blocking 里避免阻塞 tokio） ──
            let upload_dir = state.upload_dir.clone();
            let received_bytes = data.len();
            let parsed = tokio::task::spawn_blocking(move || {
                let t_blk = Instant::now();
                dev_log!("import.blk: spawn_blocking entered, {} bytes", received_bytes);

                let reader = Cursor::new(data);
                let mut archive = match ZipArchive::new(reader) {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("{} import.blk: ZipArchive::new failed: {}", TAG, e);
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "Invalid ZIP/Boothpack file".to_string(),
                        ));
                    }
                };
                let entry_count = archive.len();
                dev_log!(
                    "import.blk: zip opened, {} entries, t={:?}",
                    entry_count,
                    t_blk.elapsed()
                );

                // 读取 catalog.json
                let mut json_content = String::new();
                match archive.by_name("catalog.json") {
                    Ok(mut file) => {
                        if let Err(e) = file.read_to_string(&mut json_content) {
                            eprintln!("{} import.blk: read catalog.json failed: {}", TAG, e);
                            return Err((
                                StatusCode::BAD_REQUEST,
                                "Failed to read catalog.json".to_string(),
                            ));
                        }
                    }
                    Err(e) => {
                        eprintln!("{} import.blk: catalog.json missing: {}", TAG, e);
                        return Err((
                            StatusCode::BAD_REQUEST,
                            "Missing catalog.json in package".to_string(),
                        ));
                    }
                };
                dev_log!("import.blk: catalog.json read, {} bytes", json_content.len());

                // 向后兼容：尝试解析新格式（CatalogExport），回退到旧格式（Vec<MasterProduct>）
                let catalog: CatalogExport = if let Ok(c) =
                    serde_json::from_str::<CatalogExport>(&json_content)
                {
                    dev_log!(
                        "import.blk: parsed as CatalogExport — products={} images={}",
                        c.products.len(),
                        c.product_images.len()
                    );
                    c
                } else if let Ok(products) =
                    serde_json::from_str::<Vec<MasterProduct>>(&json_content)
                {
                    dev_log!(
                        "import.blk: parsed as legacy Vec<MasterProduct> — products={}",
                        products.len()
                    );
                    CatalogExport {
                        products,
                        product_images: vec![],
                    }
                } else {
                    // 解析失败时打印 JSON 头一段帮助定位（截断到 200 字节避免刷屏）
                    let preview: String = json_content.chars().take(200).collect();
                    eprintln!(
                        "{} import.blk: JSON parse failed; preview: {:?}",
                        TAG, preview
                    );
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "JSON Parse Error in catalog.json".to_string(),
                    ));
                };

                // ── 2. 解压所有图片文件（磁盘 I/O，在 blocking 线程池里执行） ──
                let t_extract = Instant::now();
                let mut extracted = 0usize;
                let mut skipped = 0usize;
                let mut errored = 0usize;
                for i in 0..entry_count {
                    let mut file = match archive.by_index(i) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("{} import.blk: read zip entry {} failed: {}", TAG, i, e);
                            errored += 1;
                            continue;
                        }
                    };
                    let file_path_str = file.name().to_string();

                    if file_path_str == "catalog.json" || file_path_str.ends_with('/') {
                        skipped += 1;
                        continue;
                    }
                    if file_path_str.contains("..") {
                        eprintln!(
                            "{} import.blk: rejected path-traversal entry: {}",
                            TAG, file_path_str
                        );
                        errored += 1;
                        continue;
                    }

                    let target_path = upload_dir.join(&file_path_str);
                    if let Some(parent) = target_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!(
                                "{} import.blk: create_dir_all {:?} failed: {}",
                                TAG, parent, e
                            );
                            errored += 1;
                            continue;
                        }
                    }
                    match std::fs::File::create(&target_path) {
                        Ok(mut outfile) => match std::io::copy(&mut file, &mut outfile) {
                            Ok(_) => {
                                extracted += 1;
                                // 每 50 个 entry 打一行进度，免得 200 张图刷屏
                                if extracted % 50 == 0 {
                                    dev_log!(
                                        "import.blk: extracted {}/{} entries (t={:?})",
                                        extracted,
                                        entry_count,
                                        t_extract.elapsed()
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "{} import.blk: extract {} -> {:?} failed: {}",
                                    TAG, file_path_str, target_path, e
                                );
                                errored += 1;
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "{} import.blk: create file {:?} failed: {}",
                                TAG, target_path, e
                            );
                            errored += 1;
                        }
                    }
                }
                eprintln!(
                    "{} import.blk: extraction done — extracted={} skipped={} errored={} elapsed={:?}",
                    TAG,
                    extracted,
                    skipped,
                    errored,
                    t_extract.elapsed()
                );

                Ok(catalog)
            })
            .await;

            // 处理 spawn_blocking 的结果
            let catalog = match parsed {
                Ok(Ok(c)) => {
                    eprintln!(
                        "{} import: spawn_blocking returned OK, products={} images={}",
                        TAG,
                        c.products.len(),
                        c.product_images.len()
                    );
                    c
                }
                Ok(Err((status, msg))) => {
                    eprintln!("{} import: spawn_blocking returned error: {} ({})", TAG, msg, status);
                    return (status, msg).into_response();
                }
                Err(e) => {
                    eprintln!("{} import: spawn_blocking PANICKED: {}", TAG, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal error during import".to_string(),
                    )
                        .into_response();
                }
            };

            // ── 3. 数据库写入（事务尽量短，只做 DB 操作） ──
            let t_db = Instant::now();
            let mut tx = match state.db.begin().await {
                Ok(tx) => tx,
                Err(e) => {
                    eprintln!("{} import.db: tx begin failed: {}", TAG, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("DB Error: {}", e),
                    )
                        .into_response();
                }
            };
            dev_log!("import.db: tx begun");

            // Upsert 商品数据
            let products_count = catalog.products.len();
            for (idx, prod) in catalog.products.iter().enumerate() {
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
                    eprintln!("{} import.db: upsert product {} failed: {}", TAG, prod.product_code, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("DB Write Failed: {}", e),
                    )
                        .into_response();
                }
                if (idx + 1) % 50 == 0 {
                    dev_log!(
                        "import.db: upserted {}/{} products (t={:?})",
                        idx + 1,
                        products_count,
                        t_db.elapsed()
                    );
                }
            }
            dev_log!(
                "import.db: products done — {} upserts, t={:?}",
                products_count,
                t_db.elapsed()
            );

            // Upsert AI 识别用图片（通过 product_code 关联到本地商品 id）
            let t_img = Instant::now();
            let images_count = catalog.product_images.len();
            let mut images_skipped_no_master = 0usize;
            for (idx, img) in catalog.product_images.iter().enumerate() {
                let master_id: Option<(i64,)> = query_as(
                    "SELECT id FROM master_products WHERE product_code = ?",
                )
                .bind(&img.product_code)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);

                let Some((mid,)) = master_id else {
                    images_skipped_no_master += 1;
                    continue;
                };

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
                    eprintln!("{} import.db: upsert image {} failed: {}", TAG, img.image_url, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("DB Write Failed (image): {}", e),
                    )
                        .into_response();
                }
                if (idx + 1) % 100 == 0 {
                    dev_log!(
                        "import.db: upserted {}/{} images (t={:?})",
                        idx + 1,
                        images_count,
                        t_img.elapsed()
                    );
                }
            }
            dev_log!(
                "import.db: images done — {} processed ({} skipped: no matching product_code), t={:?}",
                images_count,
                images_skipped_no_master,
                t_img.elapsed()
            );

            // 提交事务
            let t_commit = Instant::now();
            if let Err(e) = tx.commit().await {
                eprintln!("{} import.db: commit failed: {}", TAG, e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Commit Failed: {}", e),
                )
                    .into_response();
            }
            eprintln!(
                "{} import: done — products={} images={} (commit took {:?}, total {:?})",
                TAG,
                products_count,
                images_count,
                t_commit.elapsed(),
                t0.elapsed()
            );

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

    eprintln!(
        "{} import: no 'file' field found in multipart body (t={:?})",
        TAG,
        t0.elapsed()
    );
    (StatusCode::BAD_REQUEST, "No file found in request").into_response()
}
