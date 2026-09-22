//! 领域层：账本、金额、位置与账户。
//!
//! 这一层**不认识 axum，也不认识 Tauri**，只认识 sqlx 的 Executor/Transaction。
//! 把它单独拎出来是为了 ②-2 的折扣求解器能作为纯函数测试。

pub mod allocation;
pub mod ledger;
pub mod money;
pub mod pricing;
pub mod solver;
