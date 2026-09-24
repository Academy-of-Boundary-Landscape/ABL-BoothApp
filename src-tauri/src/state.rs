// src/state.rs

use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

use crate::vision::VisionRuntime;

/// 全局共享状态，通过 Axum 的 State 机制注入到 Handler 中
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub upload_dir: PathBuf,
    /// legacy 端点要靠它定位 v1 备份；不从 upload_dir 反推是为了避免隐式耦合。
    pub app_data_dir: PathBuf,
    pub jwt_secret: String,
    pub vision_runtime: Arc<VisionRuntime>,
    /// LAN HTTPS 实际监听的端口（首选 5141，被占时回退，见 server::https_port_candidates）。
    /// 二维码链接必须用它，不能用常量。
    pub lan_https_port: u16,
}
