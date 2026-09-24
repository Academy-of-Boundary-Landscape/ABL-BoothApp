use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use chrono::{NaiveDateTime, Timelike};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    api::openapi::ApiErrorBody, db::models::Event, domain::money::Money, state::AppState,
    utils::security::Claims,
};

use chrono::Local;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook}; // 用于在Excel中显示生成时间（可选）
use sqlx::AssertSqlSafe;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_event_stats))
        .routes(routes!(get_sales_summary))
        .routes(routes!(download_sales_summary))
}

/// 本模块的错误体是历史遗留的纯文本（`Access denied` / `Event not found` /
/// `Failed to generate excel`，见 REPORT 的「形状清理」），所以三个 handler 都
/// 原样返回 `Response`；成功体已经收成具名 `#[derive(ToSchema)]` 结构体。
/// `ApiError` 会把错误体变成 `{"error": ...}`，不能用。

#[derive(Deserialize, IntoParams)]
#[allow(dead_code)]
struct SummaryQuery {
    product_code: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    interval_minutes: Option<i64>,
}

#[derive(Serialize, FromRow, ToSchema)]
#[schema(as = StatsProductSalesItem)]
struct ProductSalesItem {
    product_id: i64,
    product_code: String,
    product_name: String,
    /// 单位：分。
    #[schema(value_type = Money)]
    unit_price: i64,
    /// 「累计进货」= 从外部进到现场仓的总件数（不是当前余额）。
    initial_stock: i64,
    total_quantity: i64,
    /// 单位：分。**顾客实付合计**（`Σ paid_amount`），不是原价合计——
    /// Lot 分摊与手工折让都已经摊进去了。
    #[schema(value_type = Money)]
    total_revenue_per_item: i64,
}

/// 仪表盘汇总。金额单位：分。
#[derive(Serialize, FromRow, ToSchema)]
#[schema(as = StatsSummary)]
struct SummaryStats {
    /// 单位：分。
    #[schema(value_type = Money)]
    total_revenue: i64,
    completed_orders_count: i64,
    total_items_sold: i64,
}

/// 仪表盘响应：展会信息 + 汇总 + 按商品明细。
#[derive(Serialize, ToSchema)]
#[schema(as = StatsResponse)]
struct StatsResponse {
    event_info: Event,
    summary: SummaryStats,
    product_details: Vec<ProductSalesItem>,
}

/// 趋势图上的一个时间桶。金额单位：分。
#[derive(Serialize, ToSchema)]
#[schema(as = StatsTimeseriesItem)]
struct TimeseriesItem {
    date: String,
    /// 单位：分。
    #[schema(value_type = Money)]
    revenue: i64,
}

/// 销售趋势响应。金额单位：分。
#[derive(Serialize, ToSchema)]
#[schema(as = StatsSalesResponse)]
struct SalesResponse {
    event_name: String,
    /// 单位：分。
    #[schema(value_type = Money)]
    total_revenue: i64,
    summary: Vec<ProductSalesItem>,
    timeseries: Vec<TimeseriesItem>,
}

// ==========================================
// 1. 获取仪表盘统计 (Dashboard Stats)
// ==========================================
/// 展会仪表盘：总额、订单数、售出件数以及按商品汇总。
///
/// 需要管理员，或本场摊主。
#[utoipa::path(
    get,
    path = "/{event_id}/stats",
    tag = "stats",
    params(("event_id" = i64, Path, description = "展会 id")),
    security(("bearer" = [])),
    responses(
        (status = 200, body = StatsResponse, description = "仪表盘统计"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
async fn get_event_stats(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
) -> Response {
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

    let summary: SummaryStats = sqlx::query_as(
        r#"
        SELECT
            -- 金额必须单独聚合：和 order_lines join 会把一张订单按行数复制，
            -- SUM(o.final_amount) 就被放大成 N 倍（一张 2 行的单算两遍）。
            -- 这个 bug 在旧 order_items 模型里就有，迁移时一并修掉。
            COALESCE((SELECT SUM(final_amount) FROM orders
                      WHERE event_id = ? AND status != 'cancelled'), 0) as total_revenue,
            COALESCE((SELECT COUNT(*) FROM orders
                      WHERE event_id = ? AND status != 'cancelled'), 0) as completed_orders_count,
            COALESCE((SELECT SUM(ol.qty) FROM order_lines ol
                      JOIN orders o ON o.id = ol.order_id
                      WHERE o.event_id = ? AND o.status != 'cancelled'), 0) as total_items_sold
        "#,
    )
    .bind(event_id)
    .bind(event_id)
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
            -- 顾客实付，不是原价。Σ paid = final_amount（spec 4.5）保证
            -- 「按商品汇总之和 == SUM(orders.final_amount)」，仪表盘两个数字永远对得上。
            -- 货主该得多少是结算单的事（②-3），那边用 allocated_amount。
            SUM(ol.paid_amount) as total_revenue_per_item
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
/// 销售趋势图数据：按商品汇总 + 可填充空白时段的时间序列。
/// 支持按商品编号、开始/结束日期筛选，时间粒度 30 或 60 分钟。
///
/// 需要管理员，或本场摊主。
#[utoipa::path(
    get,
    path = "/{event_id}/sales_summary",
    tag = "stats",
    params(
        ("event_id" = i64, Path, description = "展会 id"),
        SummaryQuery,
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, body = SalesResponse, description = "销售趋势"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 404, body = ApiErrorBody, description = "展会不存在"),
    ),
)]
async fn get_sales_summary(
    State(state): State<AppState>,
    claims: Claims,
    Path(event_id): Path<i64>,
    Query(params): Query<SummaryQuery>,
) -> Response {
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
            -- 顾客实付，不是原价。Σ paid = final_amount（spec 4.5）保证
            -- 「按商品汇总之和 == SUM(orders.final_amount)」，仪表盘两个数字永远对得上。
            -- 货主该得多少是结算单的事（②-3），那边用 allocated_amount。
            SUM(ol.paid_amount) as total_revenue_per_item
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
/// 导出本场销售记录为 xlsx 文件。展会不存在时不返回 404，文件名回落到
/// `Event {id}` 后照常出表（现状如此，见 REPORT 的「形状清理」）。
///
/// 需要管理员，或本场摊主。
#[utoipa::path(
    get,
    path = "/{event_id}/sales_summary/download",
    tag = "stats",
    params(("event_id" = i64, Path, description = "展会 id")),
    security(("bearer" = [])),
    responses(
        (status = 200, content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", body = Vec<u8>, description = "销售记录 xlsx"),
        (status = 401, body = ApiErrorBody, description = "未登录或令牌无效"),
        (status = 403, body = ApiErrorBody, description = "无权访问这场展会"),
        (status = 500, body = ApiErrorBody, description = "生成或读取 Excel 失败"),
    ),
)]
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
            -- 顾客实付，不是原价。Σ paid = final_amount（spec 4.5）保证
            -- 「按商品汇总之和 == SUM(orders.final_amount)」，仪表盘两个数字永远对得上。
            -- 货主该得多少是结算单的事（②-3），那边用 allocated_amount。
            SUM(ol.paid_amount) as total_revenue_per_item
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
        // 这一列装的是「外部 → 现场仓」的**累计进货**（中途补货也计入），不是
        // 一次性的初始量，所以叫「带去数量」。
        "带去数量",
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
    // 注意：单元格里显示的金额是**元**，和上面的 API 响应（分，i64）不一样是有意的——
    // Excel 是给人看的，所以单价/销售额/总计都除以 100 换算成元；API 响应保持分。
    //
    // 但**累加必须用整数分**：`allocate_lot` 会算出 2857/3333/3810 这种无法用二进制
    // 精确表示的数，f64 累加会带上误差，破坏「整数分」不变量。`as f64 / 100.0`
    // 只允许出现在写单元格的那一行。
    let mut sum_revenue_cents: i64 = 0;

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
        sum_revenue_cents += item.total_revenue_per_item;
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
    let _ = worksheet.write_number_with_format(
        start_row,
        6,
        sum_revenue_cents as f64 / 100.0,
        &total_currency_format,
    );

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
    // 直接在内存里生成。以前写到系统临时目录下的固定文件名 `sales_report_{id}.xlsx`
    // 再读回来删掉——同一场展会两个下载同时进行时，一个删了另一个正要读的文件，返回 500。
    match workbook.save_to_buffer() {
        Ok(buf) => {
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
                HeaderValue::from_str(&format!("attachment; filename=\"{}\"", download_filename))
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            );
            (headers, buf).into_response()
        }
        Err(e) => {
            log::warn!("Excel generation error: {}", e);
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
        log::info!("Admin access granted");
        return Ok(());
    }

    // 摊主需要检查 access 权限
    if claims.role == "vendor" {
        if claims.access == "all" || claims.event_id == Some(event_id) {
            log::info!("Vendor access granted");
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
        admin_token, json_request, place, read_json, seed_event_and_product, test_router_with,
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

    /// 两件事：① /stats 的 SQL 还认识新 schema（全仓运行时 SQL，schema 改了编译器抓不到）
    ///        ② 多行订单的金额不能按行数重复计入——LEFT JOIN order_lines 会把一张订单
    ///           复制成 N 行，SUM(o.final_amount) 就被放大 N 倍。
    ///           这里刻意下一张**两行**的订单来钉死这一点。
    #[tokio::test]
    async fn event_stats_runs_against_the_new_schema_and_does_not_double_count() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 下一张两行订单：ep_a × 2（3000/件）+ ep_b × 1（2000/件）→ final_amount 8000 分。
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [
                    {"product_id": ep_a, "quantity": 2},
                    {"product_id": ep_b, "quantity": 1}
                ]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        // 完成它（带 channel）
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = router
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/stats"),
                Some(&token),
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
        let summary = &body["summary"];
        assert_eq!(
            summary["total_revenue"], 8000,
            "两行订单的金额只能算一遍：2 × 3000 + 1 × 2000 = 8000，不是 16000"
        );
        assert_eq!(summary["completed_orders_count"], 1);
        assert_eq!(summary["total_items_sold"], 3, "2 + 1 件");
    }

    /// Excel 导出这条路由持有自己的一套 SQL，且返回的是二进制流不是 JSON。
    /// 这里只守「SQL 还认识新 schema」这一条底线：不能 500，且返回体非空。
    #[tokio::test]
    async fn sales_summary_download_runs_against_the_new_schema() {
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        // 同 stats 测试：下一张两行订单 + 完成，让下载路由的明细查询吃进真实数据。
        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [
                    {"product_id": ep_a, "quantity": 2},
                    {"product_id": ep_b, "quantity": 1}
                ]}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信"}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = router
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/sales_summary/download"),
                Some(&token),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::OK,
            "下载路由的 SQL 也必须还认识新 schema，不能 500"
        );

        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .expect("read download body");
        assert!(!bytes.is_empty(), "xlsx 导出不能返回空流");
        assert_eq!(
            &bytes[0..2],
            &b"PK"[..],
            "xlsx 是 zip 格式，前两个字节应是 PK"
        );
    }

    #[tokio::test]
    async fn per_product_revenue_adds_up_to_the_order_total() {
        // ②-1 交接段第 4 条：Lot + 手工折让一落地，SUM(unit_price × qty) 就是
        // 「折让前的原价」，仪表盘的两个数字会对不上。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();
        // ep_a 上挂一个「任选 2 件 50」（原价 60）。候选只有一种、任选 2 件 ⇒
        // 必须允许同款重复，否则这个 Lot 会被判为永远凑不出。
        crate::test_support::seed_lot_repeat(&pool, event_id, "任选2件50", 2, 5000, &[ep_a]).await;

        let res = router
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/events/{event_id}/orders"),
                None,
                json!({"items": [{"product_id": ep_a, "quantity": 2},
                                 {"product_id": ep_b, "quantity": 1}]}),
            ))
            .await
            .unwrap();
        let order_id = read_json(res).await["id"].as_i64().unwrap();

        // solved = 5000 + 2000 = 7000，摊主再抹到 6666
        let res = router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{order_id}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "微信", "final_amount": 6666}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/stats"),
                Some(&token),
                json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = read_json(res).await;

        let total = body["summary"]["total_revenue"].as_i64().unwrap();
        assert_eq!(total, 6666);

        let per_item: i64 = body["product_details"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["total_revenue_per_item"].as_i64().unwrap())
            .sum();
        assert_eq!(
            per_item, total,
            "按商品汇总之和必须等于总额——这条由「同一订单 Σ paid = final_amount」结构上保证"
        );
    }

    #[tokio::test]
    async fn paid_amounts_sum_to_final_amounts_across_mixed_statuses() {
        // ②-2 deferred #2。pending / completed / cancelled 三种状态混在一起时，
        // 「Σ order_lines.paid_amount == Σ orders.final_amount」必须仍然成立——
        // 仪表盘的「总额」和「按商品汇总之和」结构上相等就靠这一条。
        let (router, _dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        let token = admin_token();

        let completed = place(
            &router,
            event_id,
            json!([{"product_id": ep_a, "quantity": 1}]),
        )
        .await;
        router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{completed}/status"),
                Some(&token),
                json!({"status": "completed", "channel": "现金", "final_amount": 2800}),
            ))
            .await
            .unwrap();

        let cancelled = place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 1}]),
        )
        .await;
        router
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/events/{event_id}/orders/{cancelled}/status"),
                Some(&token),
                json!({"status": "cancelled"}),
            ))
            .await
            .unwrap();

        place(
            &router,
            event_id,
            json!([{"product_id": ep_b, "quantity": 2}]),
        )
        .await; // pending

        let (sum_paid, sum_final): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT COALESCE(SUM(ol.paid_amount), 0)
                     FROM order_lines ol JOIN orders o ON o.id = ol.order_id
                     WHERE o.event_id = ?1),
                    (SELECT COALESCE(SUM(final_amount), 0) FROM orders WHERE event_id = ?1)",
        )
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sum_paid, sum_final);
    }
}

/// ③b 形状快照：钉住每个路由的 JSON 形状（键 + 类型），类型化前后必须一行不改照样绿。
#[cfg(test)]
mod shape_tests {
    use crate::test_support::{
        admin_token, json_request, read_json, seed_event_and_product, shape_of, test_router_with,
        vendor_token,
    };
    use axum::http::StatusCode;
    use axum::Router;
    use serde_json::{json, Value};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    /// 一场有成交的展会：让 event_info 的可选字段、product_details / timeseries 数组都非空。
    async fn seeded() -> (Router, tempfile::TempDir, SqlitePool, i64) {
        let (router, dir, pool) = test_router_with().await;
        let (event_id, ep_a, ep_b) = seed_event_and_product(&pool).await;
        // 夹具的 events 行 location / payment_qr_code_path 是 NULL；这里补上，
        // 否则 event_info 的形状里两个可选字段全钉成 "null"。
        sqlx::query("UPDATE events SET location = ?, payment_qr_code_path = ? WHERE id = ?")
            .bind("B1 馆")
            .bind("events/qr.png")
            .bind(event_id)
            .execute(&pool)
            .await
            .unwrap();

        let (s, body) = call(
            &router,
            "POST",
            &format!("/api/events/{event_id}/orders"),
            None,
            json!({"items": [
                {"product_id": ep_a, "quantity": 2},
                {"product_id": ep_b, "quantity": 1}
            ]}),
        )
        .await;
        assert_eq!(s, StatusCode::CREATED);
        let order_id = body["id"].as_i64().unwrap();
        let (s, _) = call(
            &router,
            "PUT",
            &format!("/api/events/{event_id}/orders/{order_id}/status"),
            Some(&admin_token()),
            json!({"status": "completed", "channel": "微信"}),
        )
        .await;
        assert_eq!(s, StatusCode::OK);

        (router, dir, pool, event_id)
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

    /// 错误体是历史遗留的纯文本，`read_json` 会 panic，所以单独读原始字节。
    async fn call_text(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, String) {
        let res = router
            .clone()
            .oneshot(json_request(method, uri, token, body))
            .await
            .unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    fn product_item() -> Value {
        json!({
            "initial_stock": "int",
            "product_code": "string",
            "product_id": "int",
            "product_name": "string",
            "total_quantity": "int",
            "total_revenue_per_item": "int",
            "unit_price": "int",
        })
    }

    #[tokio::test]
    async fn shape_get_event_stats() {
        let (router, _dir, _pool, event_id) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "GET",
            &format!("/api/events/{event_id}/stats"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "event_info": {
                    "date": "string",
                    "id": "int",
                    "location": "string",
                    "name": "string",
                    "payment_qr_code_path": "string",
                    "status": "string",
                },
                "product_details": [product_item()],
                "summary": {
                    "completed_orders_count": "int",
                    "total_items_sold": "int",
                    "total_revenue": "int",
                },
            })
        );

        // 权限不足：历史遗留的纯文本 403（形状清理清单里有记录）。
        let (s, body) = call_text(
            &router,
            "GET",
            &format!("/api/events/{event_id}/stats"),
            Some(&vendor_token(999)),
            json!(null),
        )
        .await;
        assert_eq!((s, body.as_str()), (StatusCode::FORBIDDEN, "Access denied"));

        // 展会不存在：纯文本 404。
        let (s, body) = call_text(
            &router,
            "GET",
            "/api/events/99999/stats",
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, body.as_str()),
            (StatusCode::NOT_FOUND, "Event not found")
        );

        // 未登录：Claims 提取器的标准 {"error": ...}。
        let (s, body) = call(
            &router,
            "GET",
            &format!("/api/events/{event_id}/stats"),
            None,
            json!(null),
        )
        .await;
        assert_eq!(
            (s, shape_of(&body)),
            (StatusCode::UNAUTHORIZED, json!({"error": "string"}))
        );
    }

    #[tokio::test]
    async fn shape_get_sales_summary() {
        let (router, _dir, _pool, event_id) = seeded().await;
        let t = admin_token();

        let (s, body) = call(
            &router,
            "GET",
            &format!("/api/events/{event_id}/sales_summary"),
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "event_name": "string",
                "summary": [product_item()],
                "timeseries": [{"date": "string", "revenue": "int"}],
                "total_revenue": "int",
            })
        );

        let (s, body) = call_text(
            &router,
            "GET",
            "/api/events/99999/sales_summary",
            Some(&t),
            json!(null),
        )
        .await;
        assert_eq!(
            (s, body.as_str()),
            (StatusCode::NOT_FOUND, "Event not found")
        );
    }

    /// 两台设备同时导出同一场展会：以前写的是系统临时目录下的固定文件名，
    /// 一个下载读完就删，另一个读到一半文件没了，返回 500。
    // 多线程运行时：handler 里的文件读写是同步的，单线程下它们不会交错，复现不出来。
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn concurrent_downloads_of_the_same_event_both_succeed() {
        let (router, _dir, _pool, event_id) = seeded().await;
        let t = admin_token();
        let uri = format!("/api/events/{event_id}/sales_summary/download");
        let mut set = tokio::task::JoinSet::new();
        for _ in 0..200 {
            let req = router
                .clone()
                .oneshot(json_request("GET", &uri, Some(&t), json!(null)));
            set.spawn(req);
        }
        while let Some(res) = set.join_next().await {
            assert_eq!(res.unwrap().unwrap().status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn shape_download_sales_summary() {
        let (router, _dir, _pool, event_id) = seeded().await;
        let t = admin_token();

        let res = router
            .clone()
            .oneshot(json_request(
                "GET",
                &format!("/api/events/{event_id}/sales_summary/download"),
                Some(&t),
                json!(null),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get("content-type").unwrap(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );

        // 权限不足走纯文本 403；展会不存在**不是** 404（文件名回落到
        // "Event {id}" 后照样出表），这里钉住现状免得类型化时被顺手改成 404。
        let (s, body) = call_text(
            &router,
            "GET",
            &format!("/api/events/{event_id}/sales_summary/download"),
            Some(&vendor_token(999)),
            json!(null),
        )
        .await;
        assert_eq!((s, body.as_str()), (StatusCode::FORBIDDEN, "Access denied"));
    }
}
