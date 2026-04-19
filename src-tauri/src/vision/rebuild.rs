//! Vision 索引重建模块
//!
//! 负责执行全量或增量 embedding 重建任务

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::fs;

use crate::vision::session::OnnxSession;
use crate::vision::state::{StateManager, VisionStatusSnapshot};
use crate::vision::store::VisionStore;

/// 重建执行器
pub struct RebuildExecutor {
    app_data_dir: PathBuf,
    upload_dir: PathBuf,
    db: SqlitePool,
}

impl RebuildExecutor {
    pub fn new(app_data_dir: PathBuf, upload_dir: PathBuf, db: SqlitePool) -> Self {
        Self {
            app_data_dir,
            upload_dir,
            db,
        }
    }

    /// 创建重建执行器（简化版，不需要 app_data_dir）
    pub fn new_for_task(upload_dir: PathBuf, db: SqlitePool) -> Self {
        Self {
            app_data_dir: upload_dir.clone(),
            upload_dir,
            db,
        }
    }

    /// 执行重建任务
    ///
    /// # 参数
    /// * `force_full` - true: 全量重建，false: 增量补全
    /// * `only_image_ids` - Some: 只处理指定图片，None: 按 force_full 决定
    /// * `snapshot` - 当前状态快照（包含 model_version 等信息）
    /// * `session` - ONNX 会话（用于 embedding 计算）
    /// * `state` - 状态管理器（用于实时更新进度）
    pub async fn run(
        &self,
        force_full: bool,
        only_image_ids: Option<Vec<i64>>,
        snapshot: VisionStatusSnapshot,
        session: &Arc<OnnxSession>,
        state: &StateManager,
    ) -> Result<RebuildResult, String> {
        let store = VisionStore::new(self.db.clone());

        // 1. 全量重建时清空旧 embedding
        if force_full {
            store
                .clear_embeddings_for_model(&snapshot.model_version)
                .await
                .map_err(|e| e.to_string())?;
        }

        // 3. 确定要处理的图片列表
        let image_rows = self
            .select_images_to_process(&store, force_full, only_image_ids, &snapshot.model_version)
            .await?;

        let total = image_rows.len() as i64;
        state.set_rebuild_progress(0, total).await;

        // 4. 逐图计算 embedding，实时更新进度
        let mut embedded_count = 0_i64;
        let mut processed = 0_i64;
        for row in &image_rows {
            processed += 1;

            let abs = image_url_to_abs_path(&self.upload_dir, &row.image_url);
            if !abs.exists() {
                state.set_rebuild_progress(processed, total).await;
                continue;
            }

            let bytes = fs::read(&abs).await.map_err(|e| e.to_string())?;
            // 在 spawn_blocking 中执行 CPU 密集的推理，不阻塞 tokio runtime
            let session_clone = session.clone();
            let embed_result = tokio::task::spawn_blocking(move || session_clone.embed(&bytes))
                .await
                .map_err(|e| format!("spawn_blocking join error: {}", e))?;
            match embed_result {
                Ok(vector) => {
                    store
                        .upsert_embedding(row.image_id, &snapshot.model_version, &vector)
                        .await
                        .map_err(|e| e.to_string())?;
                    embedded_count += 1;
                }
                Err(e) => {
                    eprintln!("[Vision Rebuild] embed failed for image_id={}: {}", row.image_id, e);
                }
            }

            state.set_rebuild_progress(processed, total).await;
        }

        // 5. 更新索引元信息
        let index_version = if embedded_count > 0 || force_full {
            store
                .bump_index_meta_for_model(&snapshot.model_version)
                .await
                .map_err(|e| e.to_string())?
        } else {
            store
                .get_index_meta()
                .await
                .map_err(|e| e.to_string())?
                .map(|m| m.index_version)
                .unwrap_or(1)
        };

        let index_size = store
            .count_embeddings_by_model(&snapshot.model_version)
            .await
            .map_err(|e| e.to_string())?;

        let last_rebuild_at = store
            .get_index_meta()
            .await
            .map_err(|e| e.to_string())?
            .map(|meta| meta.updated_at);

        Ok(RebuildResult {
            embedded_count,
            index_version,
            index_size,
            last_rebuild_at,
        })
    }

    /// 选择要处理的图片
    async fn select_images_to_process(
        &self,
        store: &VisionStore,
        force_full: bool,
        only_image_ids: Option<Vec<i64>>,
        model_version: &str,
    ) -> Result<Vec<crate::vision::store::RebuildImageRow>, String> {
        if let Some(ids) = only_image_ids {
            store
                .list_product_images_by_ids(&ids)
                .await
                .map_err(|e| e.to_string())
        } else if force_full {
            store
                .list_all_product_images()
                .await
                .map_err(|e| e.to_string())
        } else {
            store
                .list_images_missing_embedding_for_model(model_version)
                .await
                .map_err(|e| e.to_string())
        }
    }
}

/// 重建结果
#[derive(Debug)]
pub struct RebuildResult {
    pub embedded_count: i64,
    pub index_version: i64,
    pub index_size: i64,
    pub last_rebuild_at: Option<String>,
}

/// 图片 URL 转绝对路径
fn image_url_to_abs_path(upload_dir: &Path, image_url: &str) -> PathBuf {
    let trimmed = image_url.trim_start_matches("/uploads/");
    let decoded = urlencoding::decode(trimmed).unwrap_or(std::borrow::Cow::Borrowed(trimmed));
    upload_dir.join(decoded.as_ref())
}
