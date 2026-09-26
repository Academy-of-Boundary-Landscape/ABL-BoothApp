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
    /// 最近一次重建失败的原因；下一次重建成功后清空
    pub last_rebuild_error: Option<String>,
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
            last_rebuild_error: None,
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
    inner: RwLock<Inner>,
}

struct Inner {
    status: VisionStatusSnapshot,
    /// 重建进行中又来了新的重建请求（比如连传几张识别图）。当前这轮结束时
    /// 再补跑一轮增量，否则那几张图就永远没有 embedding。
    rebuild_pending: bool,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                status: VisionStatusSnapshot::default(),
                rebuild_pending: false,
            }),
        }
    }

    /// 获取状态快照
    pub async fn snapshot(&self) -> VisionStatusSnapshot {
        self.inner.read().await.status.clone()
    }

    /// 设置新状态
    pub async fn set(&self, new_status: VisionStatusSnapshot) {
        self.inner.write().await.status = new_status;
    }

    /// 原子性 check-and-set：如果当前未在重建则标记为重建中并返回 true；
    /// 如果已在重建，则记下「结束后要再补一轮」并返回 false。单次写锁内完成。
    pub async fn try_start_rebuilding(&self) -> bool {
        let mut inner = self.inner.write().await;
        if inner.status.is_rebuilding {
            inner.rebuild_pending = true;
            return false;
        }
        let status = &mut inner.status;
        status.is_rebuilding = true;
        status.reason = Some("VISION_REBUILDING".to_string());
        status.rebuild_processed = 0;
        status.rebuild_total = 0;
        true
    }

    /// 一轮重建结束。返回 true 表示期间有新请求挂起，调用方应当接着再跑一轮
    /// （状态保持「重建中」）；返回 false 表示重建彻底结束。
    ///
    /// 「看有没有挂起」和「清掉重建中」必须在同一把锁里做，否则两者之间进来的
    /// 请求会被 try_start_rebuilding 挂起、却再也没人去跑。
    pub async fn finish_rebuild(&self, result: Result<VisionStatusSnapshot, String>) -> bool {
        let mut inner = self.inner.write().await;
        let again = std::mem::take(&mut inner.rebuild_pending);
        match result {
            Ok(done) => inner.status = done,
            Err(e) => {
                inner.status.reason = Some("VISION_REBUILD_FAILED".to_string());
                inner.status.last_rebuild_error = Some(e);
            }
        }
        let status = &mut inner.status;
        status.rebuild_processed = 0;
        status.rebuild_total = 0;
        status.is_rebuilding = again;
        if again {
            status.reason = Some("VISION_REBUILDING".to_string());
        }
        again
    }

    /// 更新重建进度
    pub async fn set_rebuild_progress(&self, processed: i64, total: i64) {
        let status = &mut self.inner.write().await.status;
        status.rebuild_processed = processed;
        status.rebuild_total = total;
    }

    /// 设置就绪状态
    ///
    /// 目前没有调用方——is_ready 的实际更新路径在别处（bootstrap/模型切换流程），没有
    /// 走这个方法。留着是因为它和同结构体里已在用的 finish_rebuild/set_rebuild_progress
    /// 是同一组状态更新 API，接口形状看起来是配套设计的，删掉这一个不对称。
    #[allow(dead_code)]
    pub async fn set_ready(&self, is_ready: bool, reason: Option<String>) {
        let status = &mut self.inner.write().await.status;
        status.is_ready = is_ready;
        status.reason = reason;
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn done(index_size: i64) -> VisionStatusSnapshot {
        VisionStatusSnapshot {
            index_size,
            is_ready: index_size > 0,
            reason: None,
            ..VisionStatusSnapshot::default()
        }
    }

    #[tokio::test]
    async fn request_during_rebuild_is_queued_not_dropped() {
        let s = StateManager::new();
        assert!(s.try_start_rebuilding().await);
        // 重建中又来一张图：不能开第二个任务，但也不能丢
        assert!(!s.try_start_rebuilding().await);

        assert!(
            s.finish_rebuild(Ok(done(3))).await,
            "挂起的请求要求再跑一轮"
        );
        let snap = s.snapshot().await;
        assert!(snap.is_rebuilding, "补跑期间仍是重建中");
        assert_eq!(snap.index_size, 3);

        assert!(!s.finish_rebuild(Ok(done(4))).await);
        let snap = s.snapshot().await;
        assert!(!snap.is_rebuilding);
        assert_eq!(snap.index_size, 4);
        assert_eq!(snap.reason, None);
    }

    #[tokio::test]
    async fn failed_rebuild_is_reported_and_cleared_by_next_success() {
        let s = StateManager::new();
        s.set(done(5)).await;
        assert!(s.try_start_rebuilding().await);
        assert!(!s.finish_rebuild(Err("model file not found".into())).await);
        let snap = s.snapshot().await;
        assert!(!snap.is_rebuilding);
        assert_eq!(snap.reason.as_deref(), Some("VISION_REBUILD_FAILED"));
        assert_eq!(
            snap.last_rebuild_error.as_deref(),
            Some("model file not found")
        );
        assert!(snap.is_ready, "失败不影响已有索引继续可用");

        assert!(s.try_start_rebuilding().await);
        assert!(!s.finish_rebuild(Ok(done(5))).await);
        assert_eq!(s.snapshot().await.last_rebuild_error, None);
    }
}
