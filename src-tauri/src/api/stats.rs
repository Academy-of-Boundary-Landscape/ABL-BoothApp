use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use chrono::{NaiveDateTime, Timelike};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

use crate::{db::models::Event, state::AppState, utils::security::Claims};

use chrono::Local;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook}; // 用于在Excel中显示生成时间（可选）
use sqlx::AssertSqlSafe;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/:event_id/stats", get(get_event_stats))
        .route("/:event_id/sales_summary", get(get_sales_summary))
        .route(
            "/:event_id/sales_summary/download",
            get(download_sales_summary),
        )
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct SummaryQuery {
    product_code: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    interval_minutes: Option<i64>,
}

#[derive(Serialize, FromRow)]
struct ProductSalesItem {
    product_id: i64,
    product_code: String,
    product_name: String,
    /// 单位：分。
    unit_price: i64,
    /// 「累计进货」= 从外部进到现场仓的总件数（不是当前余额）。
    initial_stock: i64,
    total_quantity: i64,
    /// 单位：分。
    total_revenue_per_item: i64,
}

// ==========================================
// 1. 获取仪表盘统计 (Dashboard Stats)
// ==========================================
async fn get_event_stats(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> impl IntoResponse {
    if let Err(e) = check_read_permission(&claims, event_id) {
        return e.into_response();
    }

    let event = match sqlx::query_as::<_, Event>("SELECT * FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Event not found").into_response(),
    };

    #[derive(Serialize, FromRow)]
    struct SummaryStats {
        /// 单位：分。
        total_revenue: i64,
        completed_orders_count: i64,
        total_items_sold: i64,
    }

    let summary: SummaryStats = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(o.final_amount), 0) as total_revenue,
            COUNT(DISTINCT o.id) as completed_orders_count,
            COALESCE(SUM(ol.qty), 0) as total_items_sold
        FROM orders o
        LEFT JOIN order_lines ol ON o.id = ol.order_id
        WHERE o.event_id = ? AND o.status != 'cancelled'
        "#,
    )
    .bind(event_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(SummaryStats {
        total_revenue: 0,
        completed_orders_count: 0,
        total_items_sold: 0,
    });

    let product_details = sqlx::query_as::<_, ProductSalesItem>(
        r#"
        SELECT 
            ol.event_product_id as product_id,
            COALESCE(ep.product_code, '') as product_code,
            ep.name as product_name,
            ol.unit_price,
            COALESCE((
                SELECT SUM(sm.qty) FROM stock_movements sm
                WHERE sm.event_product_id = ol.event_product_id
                  AND sm.from_location = '外部' AND sm.to_location = '现场仓'
            ), 0) as initial_stock,
            SUM(ol.qty) as total_quantity,
            SUM(ol.unit_price * ol.qty) as total_revenue_per_item
        FROM order_lines ol
        JOIN orders o ON ol.order_id = o.id
        JOIN event_products ep ON ol.event_product_id = ep.id
        WHERE o.event_id = ? AND o.status != 'cancelled'
        GROUP BY ol.event_product_id, ep.product_code, ep.name, ol.unit_price
        ORDER BY total_revenue_per_item DESC
        "#,
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    #[derive(Serialize)]
    struct StatsResponse {
        event_info: Event,
        summary: SummaryStats,
        product_details: Vec<ProductSalesItem>,
    }

    Json(StatsResponse {
        event_info: event,
        summary,
        product_details,
    })
    .into_response()
}

// ==========================================
// 2. 获取销售趋势图数据 (Sales Summary Chart)
// ==========================================
async fn get_sales_summary(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Query(params): Query<SummaryQuery>,
) -> impl IntoResponse {
    if let Err(e) = check_read_permission(&claims, event_id) {
        return e.into_response();
    }

    // 检查展会存在性
    let event = match sqlx::query_as::<_, Event>("SELECT * FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Event not found").into_response(),
    };

    // 1. 获取商品销售详情（支持 product_code、start_date、end_date 筛选）
    //
    // 统计口径不变：仍是「非 cancelled 的订单」。这里故意不用账本（journals /
    // stock_movements / money_movements）聚合——销售统计看的是订单视图（谁买了什么），
    // 账本看的是货和钱的流向，两者都对但回答的问题不同；结算单（②-3）才是账本视图。
    let mut summary_query = String::from(
        r#"
        SELECT 
            ol.event_product_id as product_id,
            COALESCE(ep.product_code, '') as product_code,
            ep.name as product_name,
            ol.unit_price,
            COALESCE((
                SELECT SUM(sm.qty) FROM stock_movements sm
                WHERE sm.event_product_id = ol.event_product_id
                  AND sm.from_location = '外部' AND sm.to_location = '现场仓'
            ), 0) as initial_stock,
            SUM(ol.qty) as total_quantity,
            SUM(ol.unit_price * ol.qty) as total_revenue_per_item
        FROM order_lines ol
        JOIN orders o ON ol.order_id = o.id
        JOIN event_products ep ON ol.event_product_id = ep.id
        WHERE o.event_id = ? AND o.status != 'cancelled'
        "#,
    );

    let mut sql_params: Vec<String> = vec![event_id.to_string()];

    // 条件筛选
    if let Some(ref code) = params.product_code {
        summary_query.push_str(" AND ep.product_code = ?");
        sql_params.push(code.clone());
    }
    if let Some(ref start) = params.start_date {
        summary_query.push_str(" AND DATE(o.created_at) >= ?");
        sql_params.push(start.clone());
    }
    if let Some(ref end) = params.end_date {
        summary_query.push_str(" AND DATE(o.created_at) <= ?");
        sql_params.push(end.clone());
    }

    summary_query.push_str(
        r#"
        GROUP BY ol.event_product_id, ep.product_code, ep.name, ol.unit_price
        ORDER BY total_revenue_per_item DESC
        "#,
    );

    // 执行动态 SQL 查询
    let summary = {
        // 已审计：push_str 追加的全是固定片段（" AND p.product_code = ?" 之类），
        // 用户传来的 product_code / start_date / end_date 一律走 bind。
        let mut q = sqlx::query_as::<_, ProductSalesItem>(AssertSqlSafe(summary_query));
        // 绑定所有参数（第一个是 event_id）
        q = q.bind(&sql_params[0]);
        for param in &sql_params[1..] {
            q = q.bind(param);
        }
        q.fetch_all(&state.db).await.unwrap_or_default()
    };

    // 2. 获取总销售额（单位：分）
    let total_revenue: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(o.final_amount), 0) FROM orders o WHERE o.event_id = ? AND o.status != 'cancelled'"
    )
    .bind(event_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or((0,));

    // 3. 获取时间序列数据（用于图表）
    #[derive(FromRow)]
    struct TimePoint {
        created_at: NaiveDateTime,
        /// 单位：分。
        final_amount: i64,
    }

    let mut ts_query = String::from(
        r#"
        SELECT o.created_at, o.final_amount
        FROM orders o
        WHERE o.event_id = ? AND o.status != 'cancelled'
        "#,
    );

    let mut ts_params: Vec<String> = vec![event_id.to_string()];

    // 如果指定了 product_code，需要在时间序列查询中也进行过滤
    if let Some(ref code) = params.product_code {
        ts_query.push_str(
            r#"
            AND EXISTS (
                SELECT 1 FROM order_lines ol2
                JOIN event_products ep2 ON ol2.event_product_id = ep2.id
                WHERE ol2.order_id = o.id AND ep2.product_code = ?
            )
            "#,
        );
        ts_params.push(code.clone());
    }
    if let Some(ref start) = params.start_date {
        ts_query.push_str(" AND DATE(o.created_at) >= ?");
        ts_params.push(start.clone());
    }
    if let Some(ref end) = params.end_date {
        ts_query.push_str(" AND DATE(o.created_at) <= ?");
        ts_params.push(end.clone());
    }

    ts_query.push_str(" ORDER BY o.created_at ASC");

    let time_points = {
        // 已审计：同上，追加的是固定片段，用户输入只经 bind 进入。
        let mut q = sqlx::query_as::<_, TimePoint>(AssertSqlSafe(ts_query));
        q = q.bind(&ts_params[0]);
        for param in &ts_params[1..] {
            q = q.bind(param);
        }
        q.fetch_all(&state.db).await.unwrap_or_default()
    };

    // 4. 按时间粒度分组（默认 60 分钟）
    let interval_minutes = params.interval_minutes.unwrap_or(60);
    let interval_val = if interval_minutes == 30 { 30 } else { 60 };

    fn floor_time(t: NaiveDateTime, interval: u32) -> Option<NaiveDateTime> {
        let floored = (t.minute() / interval) * interval;
        t.with_minute(floored).and_then(|t| t.with_second(0))
    }

    let mut bucketed: HashMap<String, i64> = HashMap::new();
    for point in &time_points {
        if let Some(bucket_time) = floor_time(point.created_at, interval_val as u32) {
            let key = bucket_time.format("%Y-%m-%d %H:%M").to_string();
            *bucketed.entry(key).or_insert(0) += point.final_amount;
        }
    }

    // 5. 填充空白时间桶（让图表显示连续的时间轴，无销售的时段为 0）
    #[derive(Serialize)]
    struct TimeseriesItem {
        date: String,
        /// 单位：分。
        revenue: i64,
    }

    let mut timeseries: Vec<TimeseriesItem> = Vec::new();

    if time_points.len() >= 2 {
        let first_time = time_points
            .first()
            .and_then(|p| floor_time(p.created_at, interval_val as u32));
        let last_time = time_points
            .last()
            .and_then(|p| floor_time(p.created_at, interval_val as u32));

        if let (Some(start), Some(end)) = (first_time, last_time) {
            let mut cursor = start;
            while cursor <= end {
                let key = cursor.format("%Y-%m-%d %H:%M").to_string();
                let revenue = bucketed.get(&key).copied().unwrap_or(0);
                timeseries.push(TimeseriesItem { date: key, revenue });
                cursor += chrono::Duration::minutes(interval_val);
            }
        }
    } else {
        // 0 或 1 个数据点：直接输出，不需要填充
        timeseries = bucketed
            .into_iter()
            .map(|(date, revenue)| TimeseriesItem { date, revenue })
            .collect();
        timeseries.sort_by(|a, b| a.date.cmp(&b.date));
    }

    #[derive(Serialize)]
    struct SalesResponse {
        event_name: String,
        /// 单位：分。
        total_revenue: i64,
        summary: Vec<ProductSalesItem>,
        timeseries: Vec<TimeseriesItem>,
    }

    Json(SalesResponse {
        event_name: event.name,
        total_revenue: total_revenue.0,
        summary,
        timeseries,
    })
    .into_response()
}
// ==========================================
// 3. 导出 Excel (Excel Download) - 美化版
// ==========================================
async fn download_sales_summary(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> Response {
    if let Err(e) = check_read_permission(&claims, event_id) {
        return e.into_response();
    }

    // 获取展会名称
    let event_name = sqlx::query_as::<_, Event>("SELECT * FROM events WHERE id = ?")
        .bind(event_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
        .map(|e| e.name)
        .unwrap_or_else(|| format!("Event {}", event_id));

    // 获取数据
    let details = sqlx::query_as::<_, ProductSalesItem>(
        r#"
        SELECT 
            ol.event_product_id as product_id,
            COALESCE(ep.product_code, '') as product_code,
            ep.name as product_name,
            ol.unit_price,
            COALESCE((
                SELECT SUM(sm.qty) FROM stock_movements sm
                WHERE sm.event_product_id = ol.event_product_id
                  AND sm.from_location = '外部' AND sm.to_location = '现场仓'
            ), 0) as initial_stock,
            SUM(ol.qty) as total_quantity,
            SUM(ol.unit_price * ol.qty) as total_revenue_per_item
        FROM order_lines ol
        JOIN orders o ON ol.order_id = o.id
        JOIN event_products ep ON ol.event_product_id = ep.id
        WHERE o.event_id = ? AND o.status != 'cancelled'
        GROUP BY ol.event_product_id, ep.product_code, ep.name, ol.unit_price
        ORDER BY total_revenue_per_item DESC
        "#,
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    use axum::http::header::{HeaderMap, HeaderValue};
    use std::fs;

    let temp_dir = std::env::temp_dir();
    let filename = format!("sales_report_{}.xlsx", event_id);
    let temp_file = temp_dir.join(&filename);

    // --- Excel 生成逻辑 ---
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // 1. 样式定义
    let title_format = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xDDEBF7))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    let text_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Left);

    let center_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    let currency_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right)
        .set_num_format("#,##0.00");

    let total_row_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF2F2F2))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    let total_currency_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF2F2F2))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Right)
        .set_num_format("#,##0.00");

    // 2. 设置列宽
    let _ = worksheet.set_column_width(0, 15);
    let _ = worksheet.set_column_width(1, 25);
    let _ = worksheet.set_column_width(2, 12);
    let _ = worksheet.set_column_width(3, 12);
    let _ = worksheet.set_column_width(4, 15);
    let _ = worksheet.set_column_width(5, 12);
    let _ = worksheet.set_column_width(6, 15);

    // 3. 写入标题 (merge_range 保持不变，它支持带格式)
    let title_text = format!("{} 展会销售记录表", event_name);
    let _ = worksheet.merge_range(0, 0, 0, 6, &title_text, &title_format);

    let time_str = format!("生成时间: {}", Local::now().format("%Y-%m-%d %H:%M"));
    let _ = worksheet.merge_range(
        1,
        0,
        1,
        6,
        &time_str,
        &Format::new().set_align(FormatAlign::Right),
    );

    // 4. 写入表头
    let headers = [
        "制品编号",
        "制品名",
        "初始数量",
        "结束数量",
        "单价",
        "销售量",
        "销售额",
    ];
    let header_row_idx = 2;
    for (col, text) in headers.iter().enumerate() {
        // 修改点：使用 write_string_with_format
        let _ =
            worksheet.write_string_with_format(header_row_idx, col as u16, *text, &header_format);
    }

    // 冻结窗格
    let _ = worksheet.set_freeze_panes(header_row_idx + 1, 0);

    // 5. 写入数据
    let mut start_row = header_row_idx + 1;
    let mut sum_quantity: i64 = 0;
    // 注意：这里的金额是**元**（f64），和上面的 API 响应（分，i64）不一样是有意的——
    // Excel 单元格是给人看的，所以单价/销售额/总计都除以 100 换算成元；API 响应保持分。
    let mut sum_revenue: f64 = 0.0;

    for item in details.iter() {
        // 修改点：所有带 format 的都加上 _with_format
        let _ =
            worksheet.write_string_with_format(start_row, 0, &item.product_code, &center_format);
        let _ = worksheet.write_string_with_format(start_row, 1, &item.product_name, &text_format);
        let _ = worksheet.write_number_with_format(
            start_row,
            2,
            item.initial_stock as f64,
            &center_format,
        );
        let _ = worksheet.write_number_with_format(
            start_row,
            4,
            item.unit_price as f64 / 100.0,
            &currency_format,
        );
        let _ = worksheet.write_number_with_format(
            start_row,
            5,
            item.total_quantity as f64,
            &center_format,
        );
        let _ = worksheet.write_number_with_format(
            start_row,
            6,
            item.total_revenue_per_item as f64 / 100.0,
            &currency_format,
        );

        // 第三列空着，用于现场填写结束数量进行盘点
        let _ = worksheet.write_blank(start_row, 3, &text_format);

        sum_quantity += item.total_quantity;
        sum_revenue += item.total_revenue_per_item as f64 / 100.0;
        start_row += 1;
    }

    // 6. 写入总计
    let _ = worksheet.write_string_with_format(start_row, 0, "总计", &total_row_format);

    // 注意：write_blank 不需要 _with_format 就能应用背景色
    let _ = worksheet.write_blank(start_row, 1, &total_row_format);
    let _ = worksheet.write_blank(start_row, 2, &total_row_format);
    let _ = worksheet.write_blank(start_row, 3, &total_row_format);
    let _ = worksheet.write_blank(start_row, 4, &total_row_format);

    let _ =
        worksheet.write_number_with_format(start_row, 5, sum_quantity as f64, &total_row_format);
    let _ = worksheet.write_number_with_format(start_row, 6, sum_revenue, &total_currency_format);

    start_row += 1;
    // 7. 写入备注和签名窗格

    // 空一行
    start_row += 1;

    // 备注框样式
    let note_format = Format::new()
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    // 备注行 - 设置行高为 40 点
    let _ = worksheet.set_row_height(start_row, 40.0);
    let _ = worksheet.write_string_with_format(start_row, 0, "备注信息:", &note_format);
    let _ = worksheet.merge_range(start_row, 1, start_row, 6, "", &note_format);

    start_row += 1;

    // 签名行 - 设置行高为 25 点
    let _ = worksheet.set_row_height(start_row, 30.0);
    let _ = worksheet.write_string_with_format(start_row, 0, "出摊人:", &note_format);
    let _ = worksheet.merge_range(start_row, 1, start_row, 6, "", &note_format);

    start_row += 1;
    // 写一行用于说明
    let instruction_format = Format::new().set_italic().set_align(FormatAlign::Left);
    let instruction_text = "说明：请在“结束数量”栏填写展会结束时你清点出来的数量，和销售情况做对比，以盘点可能的货物丢失。";
    let _ = worksheet.set_row_height(start_row, 30.0);
    let _ = worksheet.merge_range(
        start_row,
        0,
        start_row,
        6,
        instruction_text,
        &instruction_format,
    );

    // 保存
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
                let download_filename = format!("sales_report_event_{}.xlsx", event_id);
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
            eprintln!("Excel generation error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to generate excel",
            )
                .into_response()
        }
    }
}

fn check_read_permission(claims: &Claims, event_id: i64) -> Result<(), (StatusCode, &'static str)> {
    // 管理员拥有所有权限
    if claims.role == "admin" {
        println!("Admin access granted");
        return Ok(());
    }

    // 摊主需要检查 access 权限
    if claims.role == "vendor" {
        if claims.access == "all" || claims.event_id == Some(event_id) {
            println!("Vendor access granted");
            return Ok(());
        }
        // 摊主权限不足
        return Err((StatusCode::FORBIDDEN, "Access denied"));
    }

    // 未知角色
    Err((StatusCode::FORBIDDEN, "Access denied"))
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, test_router_with,
    };
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::ServiceExt;

    /// 这条守的不是统计正确性，而是「查询还认识新 schema」。
    /// 全仓是运行时 SQL，schema 改了编译器一个都抓不到，只会在运行时炸——
    /// 而 stats 是没人写过测试的路径，正是最容易烂掉的地方。
    ///
    /// 必须让查询真的跑起来：不种事件的话 handler 会在「Event not found」提前返回，
    /// 坏 SQL 根本不会被 prepare，这条测试就白守了。种一场展会并下一单，让
    /// summary / time series / 进货子查询都吃进真实数据。
    #[tokio::test]
    async fn sales_summary_runs_against_the_new_schema() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, _ep_b) = seed_event_and_product(&pool).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 2}]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = router
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/sales_summary"),
                Some(&admin_token()),
                json!({}),
            ))
            .await
            .unwrap();
        assert_ne!(
            res.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "SQL 引用了不存在的表/列"
        );
        assert_eq!(res.status(), StatusCode::OK);

        let body = read_json(res).await;
        assert_eq!(
            body["total_revenue"], 6000,
            "2 件 × 3000 分，API 响应保持分"
        );
    }
}
