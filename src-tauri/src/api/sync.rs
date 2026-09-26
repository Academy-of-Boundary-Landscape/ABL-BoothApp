use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, FromRow};
use std::io::{Cursor, Read, Write};
use std::time::Instant;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::{
    api::guard::AdminOnly, api::openapi::ApiErrorBody, db::models::MasterProduct, state::AppState,
    utils::barcode::normalize_barcode,
};

// 日志前缀，方便从大堆 stdout/stderr 里 grep 出来。
// 用 eprintln! 是为了和现有错误日志风格一致；dev 与 release 都会写到 stderr。
// 用户朋友实际运行的是 release 构建，所以诊断日志对 release 也开启 —— 排查这次崩溃需要看到。
// 导入/导出是用户主动触发的低频操作，每次产生几十行日志可接受。
const TAG: &str = "[sync]";

macro_rules! dev_log {
    ($($arg:tt)*) => {
        log::warn!("{} {}", TAG, format_args!($($arg)*));
    };
}

use axum::extract::DefaultBodyLimit;

const SYNC_IMPORT_LIMIT_BYTES: usize = 1000 * 1024 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(export_products))
        // 多部分（multipart/form-data）端点 —— 老路径，LAN 客户端 / 浏览器 fallback 走这里，保留向后兼容
        .routes(routes!(import_products))
        // 原始字节端点 —— Tauri webview 走这里，避开 plugin-http 的 IPC 序列化阻塞
        // body 直接是 zip 字节流，Content-Type 期望 application/zip 或 application/octet-stream
        .routes(routes!(import_products_raw))
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
    /// 社团名单（v1.2+）。导入时按**名字**匹配/新建，不能用 id——
    /// 不同设备上同一个社团的 id 必然不同。
    #[serde(default)]
    societies: Vec<String>,
    /// product_code → 归属社团名（v1.2+）。导入时按名字解析成本地 id，
    /// 缺失或找不到时回落到本社团。
    #[serde(default)]
    product_owners: Vec<(String, String)>,
}

// ==========================================
// 1. 导出制品包 (Export)
// ==========================================
// ZIP 结构:
// - catalog.json       (商品数据 + 识别用图片元数据)
// - products/xxx.jpg   (商品缩略图)
// - vision/xxx.jpg     (AI 识别用图片)
/// 导出商品库与识别图片为 `.boothpack`（zip）压缩包。需要管理员。
#[utoipa::path(
    get,
    path = "/sync/export-products",
    tag = "sync",
    security(("bearer" = [])),
    responses(
        (status = 200, content_type = "application/zip", body = Vec<u8>),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, content_type = "text/plain", body = String, description = "读取数据或打包失败"),
    ),
)]
async fn export_products(State(state): State<AppState>, _: AdminOnly) -> Response {
    let t0 = Instant::now();
    log::warn!("{} export: start", TAG);

    // 1. 获取所有商品
    let products = match query_as::<_, MasterProduct>("SELECT * FROM master_products")
        .fetch_all(&state.db)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            log::warn!("{} export: fetch master_products failed: {}", TAG, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load products").into_response();
        }
    };
    dev_log!(
        "export: loaded {} master_products in {:?}",
        products.len(),
        t0.elapsed()
    );

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
            log::warn!("{} export: fetch product_images failed: {}", TAG, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load product images",
            )
                .into_response();
        }
    };
    dev_log!("export: loaded {} product_images", product_images.len());

    // 2b. 社团名单 + 每个商品的归属社团名。
    // 归属用**名字**传递而不是 id：不同设备上同一个社团的 id 必然不同。
    let societies: Vec<String> = match sqlx::query_scalar("SELECT name FROM societies ORDER BY id")
        .fetch_all(&state.db)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!("{} export: fetch societies failed: {}", TAG, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load societies",
            )
                .into_response();
        }
    };
    let product_owners: Vec<(String, String)> = match sqlx::query_as(
        r#"
        SELECT mp.product_code, s.name
        FROM master_products mp
        JOIN societies s ON s.id = mp.owner_society_id
        ORDER BY mp.id
        "#,
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!("{} export: fetch product owners failed: {}", TAG, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load product owners",
            )
                .into_response();
        }
    };
    dev_log!(
        "export: loaded {} societies, {} product_owners",
        societies.len(),
        product_owners.len()
    );

    // 3. 构建 catalog
    let catalog = CatalogExport {
        products,
        product_images,
        societies,
        product_owners,
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
                log::warn!("ZIP start file error: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to create zip entry",
                )
                    .into_response();
            }
            if let Err(e) = zip.write_all(json_str.as_bytes()) {
                log::warn!("ZIP write json error: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to write json to zip",
                )
                    .into_response();
            }
        }
        Err(e) => {
            log::warn!("JSON serialization error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to serialize data",
            )
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
                        log::warn!("{} export: add {} to zip failed: {}", TAG, zip_path, e);
                        continue;
                    }
                    if let Err(e) = zip.write_all(&file_bytes) {
                        log::warn!("{} export: write {} to zip failed: {}", TAG, zip_path, e);
                    } else {
                        total_image_bytes += file_bytes.len() as u64;
                        images_written += 1;
                    }
                }
                Err(e) => {
                    log::warn!("{} export: read {:?} failed: {}", TAG, physical_path, e);
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
        images_written,
        images_missing,
        total_image_bytes,
        t0.elapsed()
    );

    // 8. 完成 ZIP
    let cursor = match zip.finish() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("ZIP finish error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to finalize zip").into_response();
        }
    };

    let buf = cursor.into_inner();
    log::warn!(
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
//
// 提供两个端点共享同一套 ZIP 解析 + DB 写入逻辑：
//
//   POST /sync/import-products       —— multipart/form-data，向后兼容
//                                       (LAN 浏览器 / 任何用 FormData 的客户端)
//   POST /sync/import-products-raw   —— body 直接是 zip 字节流
//                                       Tauri webview 走这条，避开 plugin-http 的 IPC 序列化阻塞主线程
//
// 共享部分抽到 process_import_bytes() 里，两个 handler 只负责把字节拿出来。

/// 导入成功后的响应体。
#[derive(Serialize, ToSchema)]
#[schema(as = SyncImportResponse)]
struct SyncImportResponse {
    message: String,
    products_count: usize,
    images_count: usize,
}

/// 共享导入处理：拿到完整 zip 字节后做解压 + DB 写入，返回 axum Response。
/// `t0` 是请求入口时间戳，用于打印总耗时。
async fn process_import_bytes(state: AppState, data: Bytes, t0: Instant) -> Response {
    // ── 1. 解析 ZIP + catalog（纯内存操作，放在 spawn_blocking 里避免阻塞 tokio） ──
    let upload_dir = state.upload_dir.clone();
    let received_bytes = data.len();
    let parsed = tokio::task::spawn_blocking(move || {
        let t_blk = Instant::now();
        dev_log!(
            "import.blk: spawn_blocking entered, {} bytes",
            received_bytes
        );

        let reader = Cursor::new(data);
        let mut archive = match ZipArchive::new(reader) {
            Ok(a) => a,
            Err(e) => {
                log::warn!("{} import.blk: ZipArchive::new failed: {}", TAG, e);
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
                    log::warn!("{} import.blk: read catalog.json failed: {}", TAG, e);
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Failed to read catalog.json".to_string(),
                    ));
                }
            }
            Err(e) => {
                log::warn!("{} import.blk: catalog.json missing: {}", TAG, e);
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Missing catalog.json in package".to_string(),
                ));
            }
        };
        dev_log!(
            "import.blk: catalog.json read, {} bytes",
            json_content.len()
        );

        // 向后兼容：尝试解析新格式（CatalogExport），回退到旧格式（Vec<MasterProduct>）
        let catalog: CatalogExport =
            if let Ok(c) = serde_json::from_str::<CatalogExport>(&json_content) {
                dev_log!(
                    "import.blk: parsed as CatalogExport — products={} images={}",
                    c.products.len(),
                    c.product_images.len()
                );
                c
            } else if let Ok(products) = serde_json::from_str::<Vec<MasterProduct>>(&json_content) {
                dev_log!(
                    "import.blk: parsed as legacy Vec<MasterProduct> — products={}",
                    products.len()
                );
                CatalogExport {
                    products,
                    product_images: vec![],
                    societies: vec![],
                    product_owners: vec![],
                }
            } else {
                // 解析失败时打印 JSON 头一段帮助定位（截断到 200 字节避免刷屏）
                let preview: String = json_content.chars().take(200).collect();
                log::warn!(
                    "{} import.blk: JSON parse failed; preview: {:?}",
                    TAG,
                    preview
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
                    log::warn!("{} import.blk: read zip entry {} failed: {}", TAG, i, e);
                    errored += 1;
                    continue;
                }
            };
            let file_path_str = file.name().to_string();

            if file_path_str == "catalog.json" || file_path_str.ends_with('/') {
                skipped += 1;
                continue;
            }
            // 条目名来自别人发来的包，不可信。enclosed_name 拒绝绝对路径、盘符和 `..`：
            // 只拦 `..` 不够——`Path::join` 遇到绝对路径会整段替换，
            // 一个名为 `C:/…/Startup/x.bat` 的条目就能写进开机启动目录。
            let rel_path = file
                .enclosed_name()
                .filter(|_| !file_path_str.contains(".."))
                .map(|p| p.to_path_buf());
            let Some(rel_path) = rel_path else {
                log::warn!(
                    "{} import.blk: rejected path-traversal entry: {}",
                    TAG,
                    file_path_str
                );
                errored += 1;
                continue;
            };

            let target_path = upload_dir.join(&rel_path);
            if let Some(parent) = target_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    log::warn!(
                        "{} import.blk: create_dir_all {:?} failed: {}",
                        TAG,
                        parent,
                        e
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
                        if extracted.is_multiple_of(50) {
                            dev_log!(
                                "import.blk: extracted {}/{} entries (t={:?})",
                                extracted,
                                entry_count,
                                t_extract.elapsed()
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "{} import.blk: extract {} -> {:?} failed: {}",
                            TAG,
                            file_path_str,
                            target_path,
                            e
                        );
                        errored += 1;
                    }
                },
                Err(e) => {
                    log::warn!(
                        "{} import.blk: create file {:?} failed: {}",
                        TAG,
                        target_path,
                        e
                    );
                    errored += 1;
                }
            }
        }
        log::warn!(
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
            log::warn!(
                "{} import: spawn_blocking returned OK, products={} images={}",
                TAG,
                c.products.len(),
                c.product_images.len()
            );
            c
        }
        Ok(Err((status, msg))) => {
            log::warn!(
                "{} import: spawn_blocking returned error: {} ({})",
                TAG,
                msg,
                status
            );
            return (status, msg).into_response();
        }
        Err(e) => {
            log::warn!("{} import: spawn_blocking PANICKED: {}", TAG, e);
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
            log::warn!("{} import.db: tx begin failed: {}", TAG, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB Error: {}", e),
            )
                .into_response();
        }
    };
    dev_log!("import.db: tx begun");

    // ── 社团归属：按名字解析成本机 id，解析不到回落到本社团 ──
    // 包里 MasterProduct 自带的 owner_society_id 是**源设备的 id**，绝不能用：
    // 直接 upsert 会把商品挂到本机不相干的社团上，甚至挂到不存在的 id 上。
    // 所以下面每个商品都用 product_owners 里的社团名重新解析并**无条件覆盖**。
    let home_society_id: i64 =
        match sqlx::query_scalar("SELECT id FROM societies WHERE is_home = 1 LIMIT 1")
            .fetch_one(&mut *tx)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                log::warn!("{} import.db: resolve home society failed: {}", TAG, e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("DB Write Failed (society): {}", e),
                )
                    .into_response();
            }
        };

    // 先把包里的社团按名字补齐（已存在就复用；本社团恒为 id 1，不会被覆盖）。
    for name in &catalog.societies {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        if let Err(e) = query(
            "INSERT INTO societies (name, is_home) VALUES (?, 0)
             ON CONFLICT(name) DO NOTHING",
        )
        .bind(name)
        .execute(&mut *tx)
        .await
        {
            log::warn!("{} import.db: create society {:?} failed: {}", TAG, name, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB Write Failed (society): {}", e),
            )
                .into_response();
        }
    }

    // name -> 本机 id 的映射（补齐之后查一次即可，顺带覆盖本就存在的同名社团）。
    let mut society_ids: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    match sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM societies")
        .fetch_all(&mut *tx)
        .await
    {
        Ok(rows) => {
            for (id, name) in rows {
                society_ids.insert(name, id);
            }
        }
        Err(e) => {
            log::warn!("{} import.db: load societies failed: {}", TAG, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB Write Failed (society): {}", e),
            )
                .into_response();
        }
    }

    // product_code -> 归属社团名。
    let owner_by_code: std::collections::HashMap<&str, &str> = catalog
        .product_owners
        .iter()
        .map(|(code, name)| (code.as_str(), name.as_str()))
        .collect();

    // Upsert 商品数据
    let products_count = catalog.products.len();
    for (idx, prod) in catalog.products.iter().enumerate() {
        // 无条件用解析结果覆盖 prod.owner_society_id（外来 id）。
        let owner_society_id = owner_by_code
            .get(prod.product_code.as_str())
            .and_then(|name| society_ids.get(*name).copied())
            .unwrap_or(home_society_id);

        let res = query(
                    r#"
                    INSERT INTO master_products (product_code, name, default_price, category, image_url, is_active, tags, barcode, owner_society_id)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(product_code) DO UPDATE SET
                        name = excluded.name,
                        default_price = excluded.default_price,
                        category = excluded.category,
                        image_url = excluded.image_url,
                        is_active = excluded.is_active,
                        tags = excluded.tags,
                        barcode = COALESCE(excluded.barcode, master_products.barcode),
                        owner_society_id = excluded.owner_society_id
                    "#,
                )
                .bind(&prod.product_code)
                .bind(&prod.name)
                .bind(prod.default_price)
                .bind(&prod.category)
                .bind(&prod.image_url)
                .bind(prod.is_active)
                .bind(&prod.tags)
                // 统一走 normalize_barcode：空串 / 纯分隔符 → NULL（COALESCE 保留本机值），
                // `978-4-…` 去掉连字符后再入库。
                .bind(prod.barcode.as_deref().and_then(normalize_barcode))
                .bind(owner_society_id)
                .execute(&mut *tx)
                .await;

        if let Err(e) = res {
            log::warn!(
                "{} import.db: upsert product {} failed: {}",
                TAG,
                prod.product_code,
                e
            );
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
        let master_id: Option<(i64,)> =
            query_as("SELECT id FROM master_products WHERE product_code = ?")
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
            log::warn!(
                "{} import.db: upsert image {} failed: {}",
                TAG,
                img.image_url,
                e
            );
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
        log::warn!("{} import.db: commit failed: {}", TAG, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Commit Failed: {}", e),
        )
            .into_response();
    }
    // 新导入的识别图还没有 embedding，补一轮增量，否则拍照识别认不出这些商品。
    // 重建已在跑时会挂起、跑完再补，不会丢。
    #[cfg(feature = "vision")]
    if images_count > 0 {
        state.vision_runtime.clone().start_rebuild_task(
            state.db.clone(),
            state.upload_dir.clone(),
            false,
            None,
        );
    }

    log::warn!(
        "{} import: done — products={} images={} (commit took {:?}, total {:?})",
        TAG,
        products_count,
        images_count,
        t_commit.elapsed(),
        t0.elapsed()
    );

    (
        StatusCode::OK,
        Json(SyncImportResponse {
            message: "Import successful".to_string(),
            products_count,
            images_count,
        }),
    )
        .into_response()
}

/// 仅用于 OpenAPI 文档：multipart 表单的字段。
#[derive(ToSchema)]
#[allow(dead_code)]
struct ImportProductsForm {
    /// `.boothpack` / `.zip` 文件内容。
    #[schema(value_type = String, format = Binary)]
    file: Vec<u8>,
}

// ─── 端点 1: multipart/form-data（向后兼容）────────────────────────────────
// LAN 浏览器、curl、任何用标准 FormData 上传的客户端走这条。
/// 以 `multipart/form-data` 上传 `.boothpack` 并导入商品库（向后兼容的老路径）。需要管理员。
#[utoipa::path(
    post,
    path = "/sync/import-products",
    tag = "sync",
    request_body(content = ImportProductsForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 200, body = SyncImportResponse, description = "导入成功"),
        (status = 400, content_type = "text/plain", body = String, description = "没有 file 字段或读取上传字节失败"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
    ),
)]
async fn import_products(
    State(state): State<AppState>,
    _: AdminOnly,
    mut multipart: Multipart,
) -> Response {
    // 不需要展会守卫：只同步全局商品库和社团名单，不写任何展会的账
    let t0 = Instant::now();
    log::warn!("{} import (multipart): start", TAG);

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name() == Some("file") {
            let t_recv = Instant::now();
            let data = match field.bytes().await {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("{} import (multipart): receive bytes failed: {}", TAG, e);
                    return (StatusCode::BAD_REQUEST, format!("Upload error: {}", e))
                        .into_response();
                }
            };
            log::warn!(
                "{} import (multipart): received {} bytes in {:?}",
                TAG,
                data.len(),
                t_recv.elapsed()
            );
            return process_import_bytes(state, data, t0).await;
        }
    }

    log::warn!(
        "{} import (multipart): no 'file' field found (t={:?})",
        TAG,
        t0.elapsed()
    );
    (StatusCode::BAD_REQUEST, "No file found in request").into_response()
}

// ─── 端点 2: 原始字节（Tauri webview 主用）────────────────────────────────
// body 直接是 zip 字节流，无需 multipart 解析。客户端把 Uint8Array 直接当 body 发即可，
// 这条路径不走 plugin-http 的 multipart 编码 / IPC 字符串化，主线程不会被冻住。
/// 以 `application/octet-stream` 原始字节上传 `.boothpack` 并导入（Tauri webview 主用）。需要管理员。
#[utoipa::path(
    post,
    path = "/sync/import-products-raw",
    tag = "sync",
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    security(("bearer" = [])),
    responses(
        (status = 200, body = SyncImportResponse, description = "导入成功"),
        (status = 400, content_type = "text/plain", body = String, description = "zip 或 catalog.json 无效"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 500, content_type = "text/plain", body = String, description = "数据库写入或 zip 处理失败"),
    ),
)]
async fn import_products_raw(State(state): State<AppState>, _: AdminOnly, body: Bytes) -> Response {
    // 不需要展会守卫：只同步全局商品库和社团名单，不写任何展会的账
    let t0 = Instant::now();
    log::warn!("{} import (raw): received {} bytes", TAG, body.len());
    process_import_bytes(state, body, t0).await
}

#[cfg(test)]
mod tests {
    use crate::test_support::{admin_token, read_json, test_router_with};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use std::io::Write;
    use tower::ServiceExt;

    /// 把一个 catalog.json 打成 .boothpack（zip）字节流。
    fn make_pack(catalog: serde_json::Value) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            zip.start_file("catalog.json", zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(catalog.to_string().as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[tokio::test]
    async fn import_resolves_owner_by_society_name_and_ignores_the_id_in_the_pack() {
        // 跨设备最脏的一个坑：包里 MasterProduct 自带的 owner_society_id 是**源设备的 id**。
        // 直接信它，商品会挂到本机一个毫不相干、甚至根本不存在的社团上。
        // 正确做法是按 product_owners 里的**社团名**重新解析。
        let (router, _dir, pool) = test_router_with().await;

        // 本机已有一个叫「黄昏堂」的社团，但它的 id 是 2——和包里写的 77 完全不同
        sqlx::query("INSERT INTO societies (id, name, is_home) VALUES (2, '黄昏堂', 0)")
            .execute(&pool)
            .await
            .unwrap();

        let pack = make_pack(json!({
            "products": [
                {
                    "id": 999, "product_code": "A", "name": "本子A",
                    "default_price": 30.0, "image_url": null, "category": null,
                    "is_active": true, "tags": "", "image_count": null,
                    "owner_society_id": 77          // ← 源设备的 id，必须被忽略
                },
                {
                    "id": 998, "product_code": "B", "name": "本子B",
                    "default_price": 20.0, "image_url": null, "category": null,
                    "is_active": true, "tags": "", "image_count": null,
                    "owner_society_id": 77
                }
            ],
            "societies": ["黄昏堂", "星见社"],
            "product_owners": [
                ["A", "黄昏堂"],        // 本机已有 → 应解析到 id 2
                ["B", "从未听说的社团"]  // 本机没有、也不在 societies 里 → 应回落到本社团 1
            ]
        }));

        let res = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/import-products-raw")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .header("content-type", "application/zip")
                    .body(Body::from(pack))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let body = read_json(res).await;
        assert_eq!(status, StatusCode::OK, "导入应当成功: {body:?}");

        let a: i64 = sqlx::query_scalar(
            "SELECT owner_society_id FROM master_products WHERE product_code = 'A'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(a, 2, "应按社团名解析到本机的黄昏堂(2)，而不是包里写的 77");

        let b: i64 = sqlx::query_scalar(
            "SELECT owner_society_id FROM master_products WHERE product_code = 'B'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(b, 1, "社团名在本机解析不到时必须回落到本社团(1)，不能留 77");

        // 导入不能造出第二个本社团
        let homes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM societies WHERE is_home = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(homes, 1, "导入新建社团时 is_home 必须恒为 0");
    }

    /// 旧包没有 `barcode` 字段：serde 默认成 None，`COALESCE` 不能拿 NULL 清掉本机已有值。
    #[tokio::test]
    async fn import_keeps_local_barcode_when_the_pack_omits_it() {
        let (router, _dir, pool) = test_router_with().await;
        sqlx::query(
            "INSERT INTO master_products (product_code, name, default_price, tags, barcode)
             VALUES ('A', '本子A', 30.0, '', '4901234567894')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let pack = make_pack(json!({
            "products": [{
                "id": 1, "product_code": "A", "name": "旧包改名", "default_price": 30.0,
                "image_url": null, "category": null, "is_active": true, "tags": "",
                "image_count": null, "owner_society_id": 1
            }],
            "societies": [],
            "product_owners": []
        }));
        let res = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/import-products-raw")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .header("content-type", "application/zip")
                    .body(Body::from(pack))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let (name, barcode): (String, Option<String>) = sqlx::query_as(
            "SELECT name, barcode FROM master_products WHERE product_code = 'A'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(name, "旧包改名", "其他字段照常被包覆盖");
        assert_eq!(
            barcode.as_deref(),
            Some("4901234567894"),
            "包里没有 barcode 不能清空本机已有值"
        );
    }

    /// 新包带 `barcode` → 覆盖本机值。
    #[tokio::test]
    async fn import_updates_barcode_when_the_pack_has_one() {
        let (router, _dir, pool) = test_router_with().await;
        sqlx::query(
            "INSERT INTO master_products (product_code, name, default_price, tags, barcode)
             VALUES ('A', '本子A', 30.0, '', '1111111111111')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let pack = make_pack(json!({
            "products": [{
                "id": 1, "product_code": "A", "name": "本子A", "default_price": 30.0,
                "image_url": null, "category": null, "is_active": true, "tags": "",
                "barcode": "2222222222222", "image_count": null, "owner_society_id": 1
            }],
            "societies": [],
            "product_owners": []
        }));
        let res = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/import-products-raw")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .header("content-type", "application/zip")
                    .body(Body::from(pack))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let barcode: Option<String> =
            sqlx::query_scalar("SELECT barcode FROM master_products WHERE product_code = 'A'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(barcode.as_deref(), Some("2222222222222"));
    }

    /// 包里 `"barcode": ""` 规范化后是 NULL，`COALESCE` 不能拿它清掉本机已有值。
    #[tokio::test]
    async fn import_keeps_local_barcode_when_the_pack_sends_an_empty_string() {
        let (router, _dir, pool) = test_router_with().await;
        sqlx::query(
            "INSERT INTO master_products (product_code, name, default_price, tags, barcode)
             VALUES ('A', '本子A', 30.0, '', '4901234567894')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let pack = make_pack(json!({
            "products": [{
                "id": 1, "product_code": "A", "name": "本子A", "default_price": 30.0,
                "image_url": null, "category": null, "is_active": true, "tags": "",
                "barcode": "", "image_count": null, "owner_society_id": 1
            }],
            "societies": [],
            "product_owners": []
        }));
        let res = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/import-products-raw")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .header("content-type", "application/zip")
                    .body(Body::from(pack))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let barcode: Option<String> =
            sqlx::query_scalar("SELECT barcode FROM master_products WHERE product_code = 'A'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            barcode.as_deref(),
            Some("4901234567894"),
            "空串规范化成 NULL 后必须由 COALESCE 保留本机已有值"
        );
    }

    /// 带连字符的 ISBN 在入库前会被 normalize_barcode 去掉连字符。
    #[tokio::test]
    async fn import_normalizes_hyphenated_barcode_before_persisting() {
        let (router, _dir, pool) = test_router_with().await;

        let pack = make_pack(json!({
            "products": [{
                "id": 1, "product_code": "A", "name": "本子A", "default_price": 30.0,
                "image_url": null, "category": null, "is_active": true, "tags": "",
                "barcode": "978-4-06-123456-7", "image_count": null, "owner_society_id": 1
            }],
            "societies": [],
            "product_owners": []
        }));
        let res = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/import-products-raw")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .header("content-type", "application/zip")
                    .body(Body::from(pack))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let barcode: Option<String> =
            sqlx::query_scalar("SELECT barcode FROM master_products WHERE product_code = 'A'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(barcode.as_deref(), Some("9784061234567"));
    }

    #[tokio::test]
    async fn import_refuses_entries_that_escape_the_upload_dir() {
        // 包来自第三方。条目名是绝对路径时 `Path::join` 会整段替换，
        // 只拦 `..` 的旧检查放得过去，文件会被写到磁盘上任意位置。
        let (router, _dir, _pool) = test_router_with().await;
        let outside = tempfile::tempdir().unwrap();
        let abs = outside.path().join("pwned.txt");
        let abs_name = abs.to_string_lossy().replace('\\', "/");

        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::FileOptions::default();
            zip.start_file("catalog.json", opts).unwrap();
            zip.write_all(json!({"products": []}).to_string().as_bytes())
                .unwrap();
            zip.start_file(abs_name.as_str(), opts).unwrap();
            zip.write_all(b"x").unwrap();
            zip.start_file("products/../../escaped.txt", opts).unwrap();
            zip.write_all(b"x").unwrap();
            zip.finish().unwrap();
        }

        let res = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sync/import-products-raw")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .header("content-type", "application/zip")
                    .body(Body::from(buf.into_inner()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let body = read_json(res).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "坏条目只跳过，不让整包失败: {body:?}"
        );
        assert!(
            !abs.exists(),
            "绝对路径条目被写到了 upload 目录之外: {abs:?}"
        );
    }
}

#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        admin_token, read_json, seed_event_and_product, shape_of, test_router_with, vendor_token,
    };
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use serde_json::{json, Value};
    use std::io::Write;
    use tower::ServiceExt;

    /// 把一个 catalog.json 打成 .boothpack（zip）字节流。和 `tests::make_pack` 同形；
    /// 两边同处一个文件，但分属不同的 `mod`，私有 helper 不能跨模块引用。
    fn make_pack(catalog: Value) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            zip.start_file("catalog.json", zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(catalog.to_string().as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    /// 一个带两张商品的商品库，让导入响应里的计数不是 0。
    fn sample_catalog() -> Value {
        json!({
            "products": [
                {
                    "id": 1, "product_code": "A", "name": "本子A",
                    "default_price": 30.0, "image_url": null, "category": null,
                    "is_active": true, "tags": "", "image_count": null,
                    "owner_society_id": 1
                },
                {
                    "id": 2, "product_code": "B", "name": "本子B",
                    "default_price": 20.0, "image_url": "/uploads/b.jpg", "category": "周边",
                    "is_active": true, "tags": "红色", "image_count": 1,
                    "owner_society_id": 2
                }
            ],
            "product_images": [
                {"product_code": "B", "image_url": "/uploads/b.jpg", "kind": "main"}
            ],
            "societies": ["黄昏堂"],
            "product_owners": [["A", "本子A"], ["B", "黄昏堂"]]
        })
    }

    async fn seeded() -> (Router, tempfile::TempDir) {
        let (router, dir, pool) = test_router_with().await;
        seed_event_and_product(&pool).await;
        (router, dir)
    }

    /// 读非 JSON 响应体（本模块的错误分支是 text/plain）。
    async fn read_text(res: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    /// 手写 multipart body：只放一个名为 `file` 的文件域，内容是 zip 字节。
    fn import_products_request(pack: &[u8]) -> Request<Body> {
        let boundary = "X-BOUNDARY";
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; \
                 filename=\"pack.boothpack\"\r\nContent-Type: application/zip\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(pack);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        Request::builder()
            .method("POST")
            .uri("/api/sync/import-products")
            .header("authorization", format!("Bearer {}", admin_token()))
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap()
    }

    fn raw_request(pack: &[u8]) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/sync/import-products-raw")
            .header("authorization", format!("Bearer {}", admin_token()))
            .header("content-type", "application/octet-stream")
            .body(Body::from(pack.to_vec()))
            .unwrap()
    }

    fn import_response_shape() -> Value {
        json!({"images_count": "int", "message": "string", "products_count": "int"})
    }

    #[tokio::test]
    async fn shape_export_products() {
        let (router, _dir) = seeded().await;
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/sync/export-products")
                    .header("authorization", format!("Bearer {}", admin_token()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "application/zip"
        );
        assert!(res
            .headers()
            .get("content-disposition")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("attachment; filename=\"booth_catalog_"));

        // 未登录 → 401；非管理员 → 403。两者都是 ApiErrorBody 形状。
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/sync/export-products")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(shape_of(&read_json(res).await), json!({"error": "string"}));

        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/sync/export-products")
                    .header("authorization", format!("Bearer {}", vendor_token(1)))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert_eq!(shape_of(&read_json(res).await), json!({"error": "string"}));
    }

    #[tokio::test]
    async fn shape_import_products() {
        let (router, _dir) = seeded().await;
        let pack = make_pack(sample_catalog());

        let res = router
            .clone()
            .oneshot(import_products_request(&pack))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(shape_of(&read_json(res).await), import_response_shape());

        // 没有 file 字段 → 400，非标准错误体（纯文本）。
        let boundary = "X-BOUNDARY";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"other\"\r\n\r\nx\r\n--{boundary}--\r\n"
        );
        let req = Request::builder()
            .method("POST")
            .uri("/api/sync/import-products")
            .header("authorization", format!("Bearer {}", admin_token()))
            .header(
                "content-type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(body))
            .unwrap();
        let res = router.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(read_text(res).await, "No file found in request");
    }

    #[tokio::test]
    async fn shape_import_products_raw() {
        let (router, _dir) = seeded().await;
        let pack = make_pack(sample_catalog());

        let res = router.clone().oneshot(raw_request(&pack)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(shape_of(&read_json(res).await), import_response_shape());

        // 无效 zip → 400，非标准错误体（纯文本）。
        let res = router
            .clone()
            .oneshot(raw_request(b"not a zip"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert_eq!(read_text(res).await, "Invalid ZIP/Boothpack file");
    }
}
