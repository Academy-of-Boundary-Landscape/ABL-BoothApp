use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ==========================================
// 1. Master Product (全局商品)
// ==========================================

/// `master_products.owner_society_id` 的 serde 默认值。`.boothpack` 从老版本导入时
/// 这一列可能缺失，缺失就回落到本社团。
fn default_home_society() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
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
    /// 商业条码（JAN/EAN-13、ISBN…）。没有就为 `None`，扫描时回落到 `product_code`。
    /// `#[serde(default)]`：旧 `.boothpack` 没有这个字段，导入时不能被当成 `null` 覆盖本机已有值。
    #[serde(default)]
    pub barcode: Option<String>,
    // 由 list_products 的 LEFT JOIN + COUNT 计算得到的识别图数量；
    // 其它 SELECT * 的查询该列缺失时 sqlx::default() 返回 None
    #[serde(default)]
    #[sqlx(default)]
    pub image_count: Option<i64>,
    /// 归属社团的**默认值**。选品时会被快照到 `event_products.owner_society_id`，
    /// 之后改这里不影响已有展会的账（spec 3.1）。
    #[serde(default = "default_home_society")]
    pub owner_society_id: i64,
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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Event {
    pub id: i64,
    pub name: String,
    #[serde(rename = "date")] //以此匹配前端 JSON 字段名 "date"
    pub event_date: String,
    pub location: Option<String>,
    #[schema(value_type = crate::api::openapi::EventStatus)]
    pub status: String,
    // vendor_password 不应该通过 API 直接返回给前端，加上 skip_serializing
    #[serde(skip_serializing)]
    pub vendor_password: Option<String>,
    pub payment_qr_code_path: Option<String>,
}

// ==========================================
// 社团（货主的单位）
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Society {
    pub id: i64,
    pub name: String,
    pub is_home: bool,
}

// ==========================================
// 订单
// ==========================================
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct OrderRow {
    pub id: i64,
    pub event_id: i64,
    #[schema(value_type = crate::api::openapi::OrderStatus)]
    pub status: String,
    pub channel: Option<String>,
    /// 以下三个单位都是分。②-1 里恒相等；②-2 引入 Lot 和手工覆盖后才会分开。
    #[schema(value_type = crate::domain::money::Money)]
    pub gross_amount: i64,
    #[schema(value_type = crate::domain::money::Money)]
    pub solved_amount: i64,
    #[schema(value_type = crate::domain::money::Money)]
    pub final_amount: i64,
    /// 前端读的是 `timestamp` —— 这个 rename 是个隐形契约，
    /// `frontend/src/components/order/OrderCard.vue:55` 和
    /// `frontend/src/views/AdminEventOrders.vue:114` 都依赖它。改名会静默白屏。
    #[serde(rename = "timestamp")]
    pub created_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}
