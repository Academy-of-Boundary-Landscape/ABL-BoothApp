use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ==========================================
// 1. Master Product (全局商品)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MasterProduct {
    pub id: i64, // SQLite 的 INTEGER 对应 Rust 的 i64
    pub product_code: String,
    pub name: String,
    pub default_price: f64,        // SQLite REAL 对应 f64
    pub image_url: Option<String>, // 可能为空
    pub category: Option<String>,
    pub is_active: bool,
    #[serde(default)]
    pub tags: String, // 逗号分隔的标签，如 "博丽灵梦,红色,东方Project"
}

// 用于接收前端创建商品的请求 Body
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CreateMasterProductDTO {
    pub product_code: String,
    pub name: String,
    pub default_price: f64,
    pub category: Option<String>,
}

// ==========================================
// 2. Event (漫展场次)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: i64,
    pub name: String,
    #[serde(rename = "date")] //以此匹配前端 JSON 字段名 "date"
    pub event_date: String,
    pub location: Option<String>,
    pub status: String,
    // vendor_password 不应该通过 API 直接返回给前端，加上 skip_serializing
    #[serde(skip_serializing)]
    pub vendor_password: Option<String>,
    pub payment_qr_code_path: Option<String>,
}

// ==========================================
// 3. Product (场次库存商品)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: i64,
    pub event_id: i64,
    pub master_product_id: i64,
    pub product_code: String,
    pub name: String,
    pub price: f64,
    pub initial_stock: i64,
    pub current_stock: i64,
    // 以下字段数据库中没有，需要通过 JOIN master_products 获取
    // 使用 sqlx 里的 default 属性处理 JOIN 出来的 nullable 字段
    #[sqlx(default)]
    pub image_url: Option<String>,
    #[sqlx(default)]
    pub category: Option<String>,
    #[sqlx(default)]
    pub tags: Option<String>,
}

// ==========================================
// 4. Order (订单)
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: i64,
    pub event_id: i64,
    pub total_amount: f64,
    pub status: String, // "pending", "completed", "cancelled"
    #[serde(rename = "timestamp")]
    pub created_at: NaiveDateTime, // sqlx 会自动处理 SQLite 的 DATETIME
}

// 这是一个"复合结构体"，用于 API 返回包含 items 的完整订单信息
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct OrderWithItems {
    #[serde(flatten)] // 将 Order 的字段展开到当前 JSON 层级
    pub order: Order,
    pub items: Vec<OrderItem>,
}

// ==========================================
// 5. Order Item (订单明细)
// ==========================================
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrderItem {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub product_price: f64,
    pub quantity: i64,
    // 如果需要显示商品图片，可能需要 JOIN 后填充这个字段
    #[sqlx(default)]
    pub product_image_url: Option<String>,
}

// ==========================================
// 6. API 请求 DTO (Data Transfer Objects)
// ==========================================

// 创建订单时的请求体结构
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CreateOrderItemDTO {
    pub product_id: i64,
    pub quantity: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CreateOrderDTO {
    pub items: Vec<CreateOrderItemDTO>,
}

// ==========================================
// 7. 统计相关 DTO
// ==========================================

// 销售详情统计（对应 Python 的 summary_list）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProductSalesDetail {
    pub product_id: i64,
    pub product_code: String,
    pub product_name: String,
    pub unit_price: f64,
    pub initial_stock: i64,
    pub total_quantity: i64,
    pub total_revenue_per_item: f64,
}

// 时间序列数据点
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SalesTimeSeries {
    pub time_bucket: String,
    pub revenue: f64,
}

// 汇总统计
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SalesSummary {
    pub total_revenue: f64,
    pub completed_orders_count: i64,
    pub total_items_sold: i64,
}

// 完整的销售报告（对应 Python 的 _get_sales_summary_data 返回值）
#[derive(Debug, Serialize)]
pub struct SalesReport {
    pub event_id: i64,
    pub event_name: String,
    pub total_revenue: f64,
    pub summary: Vec<ProductSalesDetail>,
    pub timeseries: Vec<SalesTimeSeries>,
}
