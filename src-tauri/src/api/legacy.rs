// src/api/legacy.rs
//
// v1 历史数据的唯一出口。
//
// Task 2 在迁移前把老库永久备份到 `sale_system.db.v1-backup`（知情清零），
// 但一份躺在 app data 目录里、没有任何 UI/端点能看到的数据等于不存在。
// 这里提供状态查询和 Excel 导出两条**只读**通路。

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use std::path::{Path, PathBuf};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::{guard::AdminOnly, openapi::ApiErrorBody},
    error::ApiResult,
    state::AppState,
};

/// 本模块的路由：v1 备份状态查询与 xlsx 导出，全部挂在 `/api/legacy` 下。
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_legacy_status))
        .routes(routes!(export_legacy_xlsx))
}

/// v1 备份文件的路径。
///
/// 推导方式必须和 `db::init_db` / `db::backup_v1_once` 保持一致
/// （`sale_system.db` 的扩展名替换为 `db.v1-backup`），而不是另写一个字面量：
/// 备份文件名的唯一真相在 `db/mod.rs`，端点只是它的读者。
fn v1_backup_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir
        .join("sale_system.db")
        .with_extension("db.v1-backup")
}

/// 以只读方式打开 v1 备份。
///
/// `mode=ro` 不是洁癖——这份文件是用户数据的最后一份副本，任何写入都不可接受。
/// 另外 `mode=ro` 对不存在的文件会连接失败，而不是像默认模式那样顺手建一个新库，
/// 所以「连接失败」正好可以作为「没有备份」的判据。
async fn open_v1_backup_readonly(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    SqlitePool::connect(&format!("sqlite://{}?mode=ro", path.display())).await
}

/// `/status` 的响应。计数是给前端「有没有值得打扰用户的历史数据」判断用的。
#[derive(Serialize, ToSchema)]
struct LegacyStatus {
    has_backup: bool,
    event_count: i64,
    order_count: i64,
    /// `order_items` 的**明细行数**，不是 `SUM(quantity)` 的件数——别拿它写
    /// 「你有 N 件历史商品」这类文案。
    item_count: i64,
}

impl LegacyStatus {
    /// 备份不存在 / 打不开时的统一答复。
    ///
    /// 这不是错误：升级前就是全新安装的用户本来就没有 v1 库。
    fn none() -> Self {
        Self {
            has_backup: false,
            event_count: 0,
            order_count: 0,
            item_count: 0,
        }
    }
}

/// `SELECT COUNT(*)` 三连。老 schema 的表名（events / orders / order_items）。
async fn count_v1_rows(pool: &SqlitePool) -> Option<(i64, i64, i64)> {
    let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(pool)
        .await
        .ok()?;
    let order_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
        .fetch_one(pool)
        .await
        .ok()?;
    let item_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM order_items")
        .fetch_one(pool)
        .await
        .ok()?;
    Some((event_count, order_count, item_count))
}

/// v1 备份的状态与行数。需要管理员。
#[utoipa::path(
    get,
    path = "/status",
    tag = "legacy",
    security(("bearer" = [])),
    responses(
        (status = 200, body = LegacyStatus, description = "备份是否存在，以及老库里的展会 / 订单 / 明细行数"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
    ),
)]
async fn get_legacy_status(
    State(state): State<AppState>,
    _: AdminOnly,
) -> ApiResult<Json<LegacyStatus>> {
    let path = v1_backup_path(&state.app_data_dir);

    let Ok(pool) = open_v1_backup_readonly(&path).await else {
        return Ok(Json(LegacyStatus::none()));
    };

    let counts = count_v1_rows(&pool).await;
    pool.close().await;

    match counts {
        Some((event_count, order_count, item_count)) => Ok(Json(LegacyStatus {
            has_backup: true,
            event_count,
            order_count,
            item_count,
        })),
        // 文件在但表/数据读不出来（损坏、不是 v1 库）：同样按「没有可用历史数据」处理。
        None => Ok(Json(LegacyStatus::none())),
    }
}

// ==========================================
// 导出：备份库是 v1 的旧 schema，别按新表名查
// ==========================================

/// v1 `events`。只取导出需要的列；`location` 可空，用 COALESCE 收成 String。
#[derive(FromRow)]
struct LegacyEvent {
    id: i64,
    name: String,
    event_date: String,
    location: String,
    status: String,
}

/// v1 `orders`。`total_amount` 是 REAL，**单位元**。
#[derive(FromRow)]
struct LegacyOrder {
    id: i64,
    event_id: i64,
    total_amount: f64,
    status: String,
    created_at: String,
}

/// v1 `order_items`。`product_price` 是 REAL，**单位元**。
#[derive(FromRow)]
struct LegacyOrderItem {
    id: i64,
    order_id: i64,
    product_id: i64,
    product_name: String,
    product_price: f64,
    quantity: i64,
}

/// 从只读备份库里读出三张表。
///
/// `created_at` 在老库里是 DATETIME，可能以不同亲和类型存着；`CAST(... AS TEXT)`
/// 保证取出来一定是文本，不会因为某份老库把它存成别的类型而整条导出失败。
async fn read_v1_data(
    pool: &SqlitePool,
) -> Result<(Vec<LegacyEvent>, Vec<LegacyOrder>, Vec<LegacyOrderItem>), sqlx::Error> {
    let events = sqlx::query_as::<_, LegacyEvent>(
        "SELECT id, name, event_date, COALESCE(location, '') AS location, status
         FROM events ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    let orders = sqlx::query_as::<_, LegacyOrder>(
        "SELECT id, event_id, total_amount, status,
                COALESCE(CAST(created_at AS TEXT), '') AS created_at
         FROM orders ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    let items = sqlx::query_as::<_, LegacyOrderItem>(
        "SELECT id, order_id, product_id, product_name, product_price, quantity
         FROM order_items ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    Ok((events, orders, items))
}

/// 一个样式集合。三张表共用同一套，避免逐表重复构造。
struct SheetFormats {
    header: Format,
    text: Format,
    center: Format,
    integer: Format,
    money: Format,
}

impl SheetFormats {
    fn new() -> Self {
        Self {
            header: Format::new()
                .set_bold()
                .set_background_color(rust_xlsxwriter::Color::RGB(0xDDEBF7))
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Center)
                .set_align(FormatAlign::VerticalCenter),
            text: Format::new()
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Left),
            center: Format::new()
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Center),
            integer: Format::new()
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Right),
            money: Format::new()
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Right)
                .set_num_format("#,##0.00"),
        }
    }
}

/// 写表头。三张 sheet 的列数不同，但写法一致。
fn write_headers(worksheet: &mut rust_xlsxwriter::Worksheet, headers: &[&str], format: &Format) {
    for (col, text) in headers.iter().enumerate() {
        let _ = worksheet.write_string_with_format(0, col as u16, *text, format);
    }
}

/// 把备份库里的展会 / 订单 / 明细导成三张 sheet 的 xlsx。需要管理员。
///
/// 金额**按老库的元原样写进单元格**，不换算成分——老库那一侧就是元，
/// 中间多一次 ×100 / ÷100 只会引入浮点误差。
///
/// 错误体是原样的 `text/plain`（不是 `{"error": ...}`），保持既有形状不变。
#[utoipa::path(
    get,
    path = "/export.xlsx",
    tag = "legacy",
    security(("bearer" = [])),
    responses(
        (status = 200, content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", body = Vec<u8>, description = "三张 sheet 的 xlsx 二进制流"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "需要管理员"),
        (status = 404, content_type = "text/plain", body = String, description = "没有可导出的历史数据"),
        (status = 500, content_type = "text/plain", body = String, description = "生成或读回 xlsx 失败"),
    ),
)]
async fn export_legacy_xlsx(State(state): State<AppState>, _: AdminOnly) -> Response {
    let path = v1_backup_path(&state.app_data_dir);

    if !path.exists() {
        return (StatusCode::NOT_FOUND, "没有可导出的历史数据").into_response();
    }

    let Ok(pool) = open_v1_backup_readonly(&path).await else {
        return (StatusCode::NOT_FOUND, "没有可导出的历史数据").into_response();
    };

    let data = read_v1_data(&pool).await;
    pool.close().await;

    let Ok((events, orders, items)) = data else {
        // 文件在但读不出 v1 三张表，对用户而言同样是「没有可用历史数据」。
        return (StatusCode::NOT_FOUND, "没有可导出的历史数据").into_response();
    };

    let formats = SheetFormats::new();

    let mut workbook = Workbook::new();

    // --- Sheet 1: 展会 ---
    {
        let worksheet = workbook.add_worksheet();
        let _ = worksheet.set_name("展会");
        write_headers(
            worksheet,
            &["展会ID", "展会名称", "日期", "地点", "状态"],
            &formats.header,
        );
        let _ = worksheet.set_column_width(0, 10);
        let _ = worksheet.set_column_width(1, 24);
        let _ = worksheet.set_column_width(2, 14);
        let _ = worksheet.set_column_width(3, 18);
        let _ = worksheet.set_column_width(4, 10);

        for (row, event) in events.iter().enumerate() {
            let row = row as u32 + 1;
            let _ = worksheet.write_number_with_format(row, 0, event.id as f64, &formats.integer);
            let _ = worksheet.write_string_with_format(row, 1, &event.name, &formats.text);
            let _ = worksheet.write_string_with_format(row, 2, &event.event_date, &formats.center);
            let _ = worksheet.write_string_with_format(row, 3, &event.location, &formats.text);
            let _ = worksheet.write_string_with_format(row, 4, &event.status, &formats.center);
        }
    }

    // --- Sheet 2: 订单 ---
    {
        let worksheet = workbook.add_worksheet();
        let _ = worksheet.set_name("订单");
        write_headers(
            worksheet,
            &["订单ID", "展会ID", "金额(元)", "状态", "创建时间"],
            &formats.header,
        );
        let _ = worksheet.set_column_width(0, 10);
        let _ = worksheet.set_column_width(1, 10);
        let _ = worksheet.set_column_width(2, 14);
        let _ = worksheet.set_column_width(3, 12);
        let _ = worksheet.set_column_width(4, 22);

        for (row, order) in orders.iter().enumerate() {
            let row = row as u32 + 1;
            let _ = worksheet.write_number_with_format(row, 0, order.id as f64, &formats.integer);
            let _ =
                worksheet.write_number_with_format(row, 1, order.event_id as f64, &formats.integer);
            // 元，原样写入。
            let _ = worksheet.write_number_with_format(row, 2, order.total_amount, &formats.money);
            let _ = worksheet.write_string_with_format(row, 3, &order.status, &formats.center);
            let _ = worksheet.write_string_with_format(row, 4, &order.created_at, &formats.center);
        }
    }

    // --- Sheet 3: 订单明细 ---
    {
        let worksheet = workbook.add_worksheet();
        let _ = worksheet.set_name("订单明细");
        write_headers(
            worksheet,
            &["明细ID", "订单ID", "商品ID", "商品名称", "单价(元)", "数量"],
            &formats.header,
        );
        let _ = worksheet.set_column_width(0, 10);
        let _ = worksheet.set_column_width(1, 10);
        let _ = worksheet.set_column_width(2, 10);
        let _ = worksheet.set_column_width(3, 24);
        let _ = worksheet.set_column_width(4, 14);
        let _ = worksheet.set_column_width(5, 10);

        for (row, item) in items.iter().enumerate() {
            let row = row as u32 + 1;
            let _ = worksheet.write_number_with_format(row, 0, item.id as f64, &formats.integer);
            let _ =
                worksheet.write_number_with_format(row, 1, item.order_id as f64, &formats.integer);
            let _ = worksheet.write_number_with_format(
                row,
                2,
                item.product_id as f64,
                &formats.integer,
            );
            let _ = worksheet.write_string_with_format(row, 3, &item.product_name, &formats.text);
            // 元，原样写入。
            let _ = worksheet.write_number_with_format(row, 4, item.product_price, &formats.money);
            let _ =
                worksheet.write_number_with_format(row, 5, item.quantity as f64, &formats.integer);
        }
    }

    // --- 落盘、读回、走和 stats.rs 一样的响应头 ---
    use axum::http::header::{HeaderMap, HeaderValue};
    use std::fs;

    let temp_dir = std::env::temp_dir();
    let download_filename = "legacy_v1_export.xlsx";
    // pid + 纳秒时间戳：同一进程内并发导出（管理员连点两下）不能共用一个路径，
    // 否则 A 的 remove_file 可能赶在 B 的 read 之前，用户拿到半截损坏的 xlsx。
    let temp_file = temp_dir.join(format!(
        "legacy_v1_export_{}_{}.xlsx",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    match workbook.save(&temp_file) {
        Ok(_) => match fs::read(&temp_file) {
            Ok(buf) => {
                let _ = fs::remove_file(&temp_file);
                let mut headers = HeaderMap::new();
                headers.insert(
                    axum::http::header::CONTENT_TYPE,
                    HeaderValue::from_static(
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    ),
                );
                headers.insert(
                    axum::http::header::CONTENT_DISPOSITION,
                    HeaderValue::from_str(&format!(
                        "attachment; filename=\"{}\"",
                        download_filename
                    ))
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
                );
                (headers, buf).into_response()
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read excel").into_response(),
        },
        Err(e) => {
            log::warn!("Legacy excel generation error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate excel",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{admin_token, json_request, read_json, test_router};
    use axum::http::StatusCode;
    use serde_json::json;
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;
    use tower::ServiceExt;

    /// 造一份带 v1 老 schema 和几行数据的备份文件。
    ///
    /// 用 `create_if_missing` 的写连接现建现写，模拟 Task 2 落下的那份备份；
    /// 建表语句抄的是 v1 的 `202601020001_init_schema.sql`（只留导出会读的列）。
    pub(super) async fn write_v1_backup(dir: &Path) -> PathBuf {
        let path = dir.join("sale_system.db.v1-backup");
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
            .unwrap()
            .create_if_missing(true);
        let pool = sqlx::SqlitePool::connect_with(opts)
            .await
            .expect("create v1 backup db");

        for ddl in [
            "CREATE TABLE events (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                event_date TEXT NOT NULL,
                location TEXT,
                status TEXT NOT NULL
             )",
            "CREATE TABLE orders (
                id INTEGER PRIMARY KEY,
                event_id INTEGER NOT NULL,
                total_amount REAL NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
             )",
            "CREATE TABLE order_items (
                id INTEGER PRIMARY KEY,
                order_id INTEGER NOT NULL,
                product_id INTEGER NOT NULL,
                product_name TEXT NOT NULL,
                product_price REAL NOT NULL,
                quantity INTEGER NOT NULL
             )",
        ] {
            sqlx::query(ddl)
                .execute(&pool)
                .await
                .expect("create v1 table");
        }

        sqlx::query(
            "INSERT INTO events (id, name, event_date, location, status) VALUES
                (1, '漫展A', '2026-01-01', '上海', '已结束'),
                (2, '漫展B', '2026-02-01', NULL, '已结束')",
        )
        .execute(&pool)
        .await
        .expect("seed v1 events");

        sqlx::query(
            "INSERT INTO orders (id, event_id, total_amount, status) VALUES
                (1, 1, 88.5, 'completed'),
                (2, 1, 12.0, 'cancelled'),
                (3, 2, 30.0, 'completed')",
        )
        .execute(&pool)
        .await
        .expect("seed v1 orders");

        sqlx::query(
            "INSERT INTO order_items (id, order_id, product_id, product_name, product_price, quantity) VALUES
                (1, 1, 1, '本子', 30.0, 2),
                (2, 1, 2, '挂件', 28.5, 1),
                (3, 2, 1, '本子', 12.0, 1)",
        )
        .execute(&pool)
        .await
        .expect("seed v1 order items");

        pool.close().await;
        path
    }

    /// 全新安装没有 v1 备份是正常状态，不能返回 500。
    #[tokio::test]
    async fn status_without_backup_is_ok_and_reports_none() {
        let (router, _dir) = test_router().await;

        let res = router
            .oneshot(json_request(
                "GET",
                "/api/legacy/status",
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();

        assert_eq!(
            res.status(),
            StatusCode::OK,
            "没有备份是正常状态，不能是 500"
        );
        let body = read_json(res).await;
        assert_eq!(body["has_backup"], false);
        assert_eq!(body["event_count"], 0);
        assert_eq!(body["order_count"], 0);
        assert_eq!(body["item_count"], 0);
    }

    /// 备份存在时三个计数要对应老库里的真实行数。
    #[tokio::test]
    async fn status_counts_rows_in_an_existing_v1_backup() {
        let (router, dir) = test_router().await;
        write_v1_backup(dir.path()).await;

        let res = router
            .oneshot(json_request(
                "GET",
                "/api/legacy/status",
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;
        assert_eq!(body["has_backup"], true);
        assert_eq!(body["event_count"], 2);
        assert_eq!(body["order_count"], 3);
        assert_eq!(body["item_count"], 3);
    }

    /// 没有备份时导出是 404（「没有可导出的历史数据」），不能是 500。
    #[tokio::test]
    async fn export_without_backup_is_not_found() {
        let (router, _dir) = test_router().await;

        let res = router
            .oneshot(json_request(
                "GET",
                "/api/legacy/export.xlsx",
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();

        assert_eq!(
            res.status(),
            StatusCode::NOT_FOUND,
            "没有备份时导出应是 404，不能是 500"
        );
    }

    /// 导出这条路径单独持有 v1 老 schema 的 SQL，且返回的是二进制流。
    /// 这里让 `read_v1_data` 真的跑过一份带数据的老库，守住「还认识老表名列名」：
    /// 列名/类型写错会在这里以 5xx 或空流暴露，而不是等用户点了按钮才发现。
    #[tokio::test]
    async fn export_with_backup_returns_an_xlsx() {
        let (router, dir) = test_router().await;
        write_v1_backup(dir.path()).await;

        let res = router
            .oneshot(json_request(
                "GET",
                "/api/legacy/export.xlsx",
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        );

        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read export body");
        assert!(!bytes.is_empty(), "xlsx 导出不能返回空流");
        assert_eq!(
            &bytes[0..2],
            &b"PK"[..],
            "xlsx 是 zip 格式，前两个字节应是 PK"
        );
    }
}

/// ③b 形状快照：钉住每个路由的 JSON / 二进制形状，类型化前后必须一行不改照样绿。
#[cfg(test)]
mod shape_tests {
    use super::tests::write_v1_backup;
    use crate::test_support::{
        admin_token, json_request, read_json, shape_of, test_router_with, vendor_token,
    };
    use axum::http::StatusCode;
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    /// 一份含展会 / 订单 / 明细的 v1 备份，让 status 的计数和 export 都有内容可读。
    async fn seeded() -> (Router, tempfile::TempDir) {
        let (router, dir, _pool) = test_router_with().await;
        write_v1_backup(dir.path()).await;
        (router, dir)
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

    #[tokio::test]
    async fn shape_get_legacy_status() {
        let expected = json!({
            "has_backup": "bool",
            "event_count": "int",
            "order_count": "int",
            "item_count": "int",
        });

        // 没有备份：计数全 0，仍是 200 而不是 5xx。
        let (router, _dir, _pool) = test_router_with().await;
        let (s, body) = call(
            &router,
            "GET",
            "/api/legacy/status",
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::OK, expected.clone()));

        // 有备份：计数来自老库，形状一个键都不变。
        let (router, _dir) = seeded().await;
        let (s, body) = call(
            &router,
            "GET",
            "/api/legacy/status",
            Some(&admin_token()),
            json!(null),
        )
        .await;
        assert_eq!((s, shape_of(&body)), (StatusCode::OK, expected));

        // AdminOnly：无令牌 401、非管理员 403，错误体都是 {"error": "..."}。
        let (s, body) = call(&router, "GET", "/api/legacy/status", None, json!(null)).await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );
        let (s, body) = call(
            &router,
            "GET",
            "/api/legacy/status",
            Some(&vendor_token(1)),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::FORBIDDEN, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_export_legacy_xlsx() {
        // 有备份：200 + xlsx content-type，且是 zip（PK 头）。
        let (router, _dir) = seeded().await;
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                "/api/legacy/export.xlsx",
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            &bytes[0..2],
            &b"PK"[..],
            "xlsx 是 zip 格式，前两个字节应是 PK"
        );

        // 没有备份：404 + 原样的纯文本错误体（不是 {"error": ...} 形状）。
        let (router, _dir, _pool) = test_router_with().await;
        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                "/api/legacy/export.xlsx",
                Some(&admin_token()),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8(bytes.to_vec()).unwrap(),
            "没有可导出的历史数据"
        );

        // AdminOnly：无令牌 401、非管理员 403，错误体都是 {"error": "..."}。
        let (s, body) = call(&router, "GET", "/api/legacy/export.xlsx", None, json!(null)).await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );
        let (s, body) = call(
            &router,
            "GET",
            "/api/legacy/export.xlsx",
            Some(&vendor_token(1)),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::FORBIDDEN, json!({"error": "string"}))
        );
    }
}
