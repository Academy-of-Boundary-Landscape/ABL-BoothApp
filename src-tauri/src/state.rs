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
    pub jwt_secret: String,
    pub vision_runtime: Arc<VisionRuntime>,
}
