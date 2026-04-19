//! Vision 状态管理模块
//!
//! 负责管理 Vision 运行时状态的快照和状态变更

use tokio::sync::RwLock;

/// Vision 运行时状态快照
#[derive(Debug, Clone)]
pub struct VisionStatusSnapshot {
    pub model_id: String,
    pub model_version: String,
    pub index_version: i64,
    pub index_size: i64,
    pub is_ready: bool,
    pub is_rebuilding: bool,
    pub last_rebuild_at: Option<String>,
    pub reason: Option<String>,
    /// 重建进度：已处理图片数
    pub rebuild_processed: i64,
    /// 重建进度：总图片数
    pub rebuild_total: i64,
}

impl Default for VisionStatusSnapshot {
    fn default() -> Self {
        Self {
            model_id: "unknown".to_string(),
            model_version: "unknown".to_string(),
            index_version: 1,
            index_size: 0,
            is_ready: false,
            is_rebuilding: false,
            last_rebuild_at: None,
            reason: Some("VISION_BOOTSTRAP_PENDING".to_string()),
            rebuild_processed: 0,
            rebuild_total: 0,
        }
    }
}

/// 模型安装任务状态快照
#[derive(Debug, Clone)]
pub struct ModelInstallTaskSnapshot {
    pub task_id: String,
    pub model_id: String,
    pub status: String,
    pub progress: i32,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// 状态管理器 - 封装状态读写
pub struct StateManager {
    status: RwLock<VisionStatusSnapshot>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            status: RwLock::new(VisionStatusSnapshot::default()),
        }
    }

    /// 获取状态快照
    pub async fn snapshot(&self) -> VisionStatusSnapshot {
        self.status.read().await.clone()
    }

    /// 设置新状态
    pub async fn set(&self, new_status: VisionStatusSnapshot) {
        let mut status = self.status.write().await;
        *status = new_status;
    }

    /// 原子性 check-and-set：如果当前未在重建则标记为重建中并返回 true，
    /// 如果已在重建则返回 false。单次写锁内完成，防止并发重入。
    pub async fn try_start_rebuilding(&self) -> bool {
        let mut status = self.status.write().await;
        if status.is_rebuilding {
            return false;
        }
        status.is_rebuilding = true;
        status.reason = Some("VISION_REBUILDING".to_string());
        status.rebuild_processed = 0;
        status.rebuild_total = 0;
        true
    }

    /// 更新重建中状态
    pub async fn set_rebuilding(&self, is_rebuilding: bool) {
        let mut status = self.status.write().await;
        status.is_rebuilding = is_rebuilding;
        if is_rebuilding {
            status.reason = Some("VISION_REBUILDING".to_string());
            status.rebuild_processed = 0;
            status.rebuild_total = 0;
        }
    }

    /// 更新重建进度
    pub async fn set_rebuild_progress(&self, processed: i64, total: i64) {
        let mut status = self.status.write().await;
        status.rebuild_processed = processed;
        status.rebuild_total = total;
    }

    /// 设置就绪状态
    pub async fn set_ready(&self, is_ready: bool, reason: Option<String>) {
        let mut status = self.status.write().await;
        status.is_ready = is_ready;
        status.reason = reason;
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}
