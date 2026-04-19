#[cfg(feature = "vision")]
pub mod download;
#[cfg(feature = "vision")]
pub mod index;
#[cfg(feature = "vision")]
pub mod model;
#[cfg(feature = "vision")]
pub mod model_manager;
#[cfg(feature = "vision")]
pub mod rebuild;
#[cfg(feature = "vision")]
pub mod session;
#[cfg(feature = "vision")]
pub mod state;
#[cfg(feature = "vision")]
pub mod store;

#[cfg(feature = "vision")]
use std::path::PathBuf;
#[cfg(feature = "vision")]
use std::sync::Arc;

#[cfg(feature = "vision")]
use sqlx::SqlitePool;
#[cfg(feature = "vision")]
use tokio::sync::Semaphore;

#[cfg(feature = "vision")]
pub use model_manager::ModelManager;
#[cfg(feature = "vision")]
pub use rebuild::{RebuildExecutor, RebuildResult};
#[cfg(feature = "vision")]
pub use session::{OnnxSession, SessionCache};
#[cfg(feature = "vision")]
pub use state::{ModelInstallTaskSnapshot, StateManager, VisionStatusSnapshot};

#[cfg(feature = "vision")]
pub struct VisionRuntime {
    pub state: Arc<StateManager>,
    pub session_cache: Arc<SessionCache>,
    pub model_manager: Arc<ModelManager>,
    pub rebuild_executor: Arc<RebuildExecutor>,
    pub semaphore: Arc<Semaphore>,
}

#[cfg(feature = "vision")]
impl VisionRuntime {
    pub fn new(app_data_dir: PathBuf, upload_dir: PathBuf, db: SqlitePool) -> Self {
        Self {
            state: Arc::new(StateManager::new()),
            session_cache: Arc::new(SessionCache::new()),
            model_manager: Arc::new(ModelManager::new(app_data_dir.clone())),
            rebuild_executor: Arc::new(RebuildExecutor::new(app_data_dir, upload_dir, db)),
            semaphore: Arc::new(Semaphore::new(2)),
        }
    }

    pub async fn bootstrap(&self, db: SqlitePool) -> Result<(), String> {
        self.model_manager.bootstrap().await?;

        let store = store::VisionStore::new(db);
        let active_model_id = self.model_manager.get_active_model_id().await;

        if let Some(manifest) = self
            .model_manager
            .get_manifest_for_model(&active_model_id)
            .await
        {
            store
                .ensure_index_meta(&manifest.model_version)
                .await
                .map_err(|e| e.to_string())?;

            let meta = store.get_index_meta().await.map_err(|e| e.to_string())?;
            if let Some(meta) = meta {
                let index_size = store
                    .count_embeddings_by_model(&meta.model_version)
                    .await
                    .map_err(|e| e.to_string())?;

                self.state
                    .set(VisionStatusSnapshot {
                        model_id: manifest.model_id.clone(),
                        model_version: meta.model_version.clone(),
                        index_version: meta.index_version,
                        index_size,
                        is_ready: index_size > 0,
                        is_rebuilding: false,
                        last_rebuild_at: Some(meta.updated_at.clone()),
                        reason: if index_size > 0 {
                            None
                        } else {
                            Some("VISION_INDEX_EMPTY".to_string())
                        },
                        rebuild_processed: 0,
                        rebuild_total: 0,
                    })
                    .await;
            }

            // 预加载模型到缓存，避免首次搜索冷启动
            let model_path = download::model_abs_path(self.model_manager.app_data_dir(), &manifest);
            let ep_pref = self.model_manager.get_runtime_config().await
                .map(|c| c.execution_provider).unwrap_or_else(|| "auto".to_string());
            if model_path.exists() {
                match self
                    .session_cache
                    .get_or_load_with_check(
                        &manifest.model_id,
                        &manifest.model_version,
                        &model_path,
                        manifest.clone(),
                        ep_pref,
                    )
                    .await
                {
                    Ok(_) => println!("[Vision] Model pre-loaded: {}", manifest.model_id),
                    Err(e) => eprintln!("[Vision] Model pre-load failed: {}", e),
                }
            }
        }

        Ok(())
    }

    pub async fn snapshot(&self) -> VisionStatusSnapshot {
        self.state.snapshot().await
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }

    pub async fn timeout_ms(&self) -> u64 {
        self.model_manager
            .get_runtime_config()
            .await
            .map(|c| c.timeout_ms)
            .unwrap_or(5000)
    }

    pub async fn list_models(&self) -> Vec<(download::ModelManifest, bool, bool)> {
        let active_model_id = self.model_manager.get_active_model_id().await;
        self.model_manager.list_models(&active_model_id).await
    }

    pub async fn thresholds_for_mode(&self, mode: Option<&str>) -> (f32, f32) {
        self.model_manager.thresholds_for_mode(mode).await
    }

    pub async fn create_install_task(
        &self,
        model_id: &str,
        source: Option<String>,
    ) -> Result<String, String> {
        self.model_manager
            .create_install_task(model_id, source)
            .await
    }

    pub async fn get_install_task(&self, task_id: &str) -> Option<ModelInstallTaskSnapshot> {
        self.model_manager.get_install_task(task_id).await
    }

    pub async fn activate_model(&self, db: SqlitePool, model_id: &str) -> Result<(), String> {
        self.model_manager
            .activate_model(db, &self.state, model_id)
            .await?;
        self.session_cache.clear().await;
        Ok(())
    }

    pub async fn delete_model(&self, model_id: &str) -> Result<(), String> {
        let active_model_id = self.model_manager.get_active_model_id().await;
        self.model_manager
            .delete_model(&active_model_id, model_id)
            .await
    }

    pub async fn embed_query(&self, image_bytes: &[u8]) -> Result<Vec<f32>, String> {
        let snapshot = self.snapshot().await;
        let manifest = self
            .model_manager
            .get_manifest_for_model(&snapshot.model_id)
            .await
            .ok_or_else(|| format!("active model missing: {}", snapshot.model_id))?;

        let model_path = download::model_abs_path(self.model_manager.app_data_dir(), &manifest);
        if !model_path.exists() {
            return Err(format!("model file not found: {}", model_path.display()));
        }

        let ep_pref = self.model_manager.get_runtime_config().await
            .map(|c| c.execution_provider).unwrap_or_else(|| "auto".to_string());
        let session = self
            .session_cache
            .get_or_load_with_check(
                &manifest.model_id,
                &manifest.model_version,
                &model_path,
                manifest.clone(),
                ep_pref,
            )
            .await?;

        // 在 spawn_blocking 中执行 CPU 密集的推理，不阻塞 tokio runtime
        let bytes_owned = image_bytes.to_vec();
        tokio::task::spawn_blocking(move || session.embed(&bytes_owned))
            .await
            .map_err(|e| format!("spawn_blocking join error: {}", e))?
    }

    pub fn start_rebuild_task(
        self: Arc<Self>,
        db: SqlitePool,
        upload_dir: PathBuf,
        force_full: bool,
        only_image_ids: Option<Vec<i64>>,
    ) {
        tokio::spawn(async move {
            let state = self.state.clone();
            let model_manager = self.model_manager.clone();
            let session_cache = self.session_cache.clone();

            // 原子性 check-and-set：防止并发重复重建
            if !state.try_start_rebuilding().await {
                return;
            }

            let snapshot = state.snapshot().await;

            let manifest = match model_manager
                .get_manifest_for_model(&snapshot.model_id)
                .await
            {
                Some(m) => m,
                None => {
                    state.set_rebuilding(false).await;
                    eprintln!("Rebuild failed: active model not found");
                    return;
                }
            };

            let model_path = download::model_abs_path(model_manager.app_data_dir(), &manifest);
            if !model_path.exists() {
                state.set_rebuilding(false).await;
                eprintln!(
                    "Rebuild failed: model file not found: {}",
                    model_path.display()
                );
                return;
            }

            let ep_pref = model_manager.get_runtime_config().await
                .map(|c| c.execution_provider).unwrap_or_else(|| "auto".to_string());
            let session = match session_cache
                .get_or_load_with_check(
                    &manifest.model_id,
                    &manifest.model_version,
                    &model_path,
                    manifest.clone(),
                    ep_pref,
                )
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    state.set_rebuilding(false).await;
                    eprintln!("Rebuild failed: {}", e);
                    return;
                }
            };

            let executor = RebuildExecutor::new_for_task(upload_dir.clone(), db.clone());
            match executor
                .run(force_full, only_image_ids, snapshot, &session, &state)
                .await
            {
                Ok(result) => {
                    state
                        .set(state::VisionStatusSnapshot {
                            model_id: manifest.model_id,
                            model_version: manifest.model_version,
                            index_version: result.index_version,
                            index_size: result.index_size,
                            is_ready: result.index_size > 0,
                            is_rebuilding: false,
                            last_rebuild_at: result.last_rebuild_at,
                            reason: if result.embedded_count > 0 {
                                None
                            } else {
                                Some("VISION_INDEX_EMPTY".to_string())
                            },
                            rebuild_processed: 0,
                            rebuild_total: 0,
                        })
                        .await;
                }
                Err(e) => {
                    state.set_rebuilding(false).await;
                    eprintln!("Rebuild failed: {}", e);
                }
            }
        });
    }

    pub fn start_incremental_for_images(
        self: Arc<Self>,
        db: SqlitePool,
        upload_dir: PathBuf,
        image_ids: Vec<i64>,
    ) {
        self.start_rebuild_task(db, upload_dir, false, Some(image_ids));
    }
}

#[cfg(not(feature = "vision"))]
use std::path::PathBuf;
#[cfg(not(feature = "vision"))]
use std::sync::Arc;

#[cfg(not(feature = "vision"))]
use sqlx::SqlitePool;

#[cfg(not(feature = "vision"))]
pub struct VisionRuntime;

#[cfg(not(feature = "vision"))]
impl VisionRuntime {
    pub fn new(_app_data_dir: PathBuf, _upload_dir: PathBuf, _db: SqlitePool) -> Self {
        Self
    }

    pub async fn bootstrap(&self, _db: SqlitePool) -> Result<(), String> {
        Ok(())
    }

    pub fn start_incremental_for_images(
        self: Arc<Self>,
        _db: SqlitePool,
        _upload_dir: PathBuf,
        _image_ids: Vec<i64>,
    ) {
    }
}
