//! Vision 模型管理模块
//!
//! 负责模型的下载、安装、激活、删除等管理任务

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::vision::download;
use crate::vision::state::{ModelInstallTaskSnapshot, StateManager, VisionStatusSnapshot};

/// 模型管理器
pub struct ModelManager {
    app_data_dir: PathBuf,
    registry: Arc<RwLock<download::ModelRegistry>>,
    config: Arc<RwLock<Option<download::VisionModelConfig>>>,
    install_tasks: Arc<RwLock<HashMap<String, ModelInstallTaskSnapshot>>>,
}

impl ModelManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            app_data_dir,
            registry: Arc::new(RwLock::new(download::ModelRegistry::default())),
            config: Arc::new(RwLock::new(None)),
            install_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ========== 配置加载 ==========

    /// 从数据库加载配置和注册表
    pub async fn bootstrap(&self) -> Result<(), String> {
        download::ensure_default_files(&self.app_data_dir).await?;

        let registry = download::load_registry(&self.app_data_dir).await?;
        if registry.models.is_empty() {
            return Err("model registry is empty".to_string());
        }

        let mut cfg = download::load_runtime_config(&self.app_data_dir).await?;

        // 确保 active_model_id 在注册表中存在
        if !registry
            .models
            .iter()
            .any(|m| m.model_id == cfg.active_model_id)
        {
            cfg.active_model_id = registry.models[0].model_id.clone();
            download::save_runtime_config(&self.app_data_dir, &cfg).await?;
        }

        *self.registry.write().await = registry.clone();
        *self.config.write().await = Some(cfg.clone());

        Ok(())
    }

    // ========== 模型列表 ==========

    /// 获取所有模型及其状态
    pub async fn list_models(
        &self,
        active_model_id: &str,
    ) -> Vec<(download::ModelManifest, bool, bool)> {
        let registry = self.registry.read().await;

        registry
            .models
            .iter()
            .map(|model| {
                let installed = download::is_model_installed(&self.app_data_dir, model);
                let active = model.model_id == active_model_id;
                (model.clone(), installed, active)
            })
            .collect()
    }

    /// 获取阈值配置
    pub async fn thresholds_for_mode(&self, mode: Option<&str>) -> (f32, f32) {
        let guard = self.config.read().await;
        if let Some(cfg) = guard.as_ref() {
            if let Some(mode_key) = mode {
                if let Some(item) = cfg.thresholds.by_mode.get(mode_key) {
                    return (item.top1_min, item.top1_top2_gap_min);
                }
            }
            return (
                cfg.thresholds.default_top1_min,
                cfg.thresholds.default_top1_top2_gap_min,
            );
        }
        (0.28, 0.03)
    }

    /// 获取运行时配置
    pub async fn get_runtime_config(&self) -> Option<download::VisionRuntimeConfig> {
        self.config.read().await.as_ref().map(|c| c.runtime.clone())
    }

    // ========== 模型安装 ==========

    /// 创建模型下载任务
    pub async fn create_install_task(
        &self,
        model_id: &str,
        source: Option<String>,
    ) -> Result<String, String> {
        let registry = self.registry.read().await.clone();
        let manifest = download::find_model(&registry, model_id)
            .ok_or_else(|| format!("Unsupported model_id: {}", model_id))?
            .clone();

        if download::is_model_installed(&self.app_data_dir, &manifest) {
            return Err(format!("Model already installed: {}", model_id));
        }

        let task_id = Uuid::new_v4().to_string();
        self.install_tasks.write().await.insert(
            task_id.clone(),
            ModelInstallTaskSnapshot {
                task_id: task_id.clone(),
                model_id: model_id.to_string(),
                status: "downloading".to_string(),
                progress: 1,
                message: Some("Task created".to_string()),
                error: None,
            },
        );

        // 启动后台下载任务
        self.spawn_download_task(task_id.clone(), model_id.to_string(), source)
            .await;

        Ok(task_id)
    }

    async fn spawn_download_task(&self, task_id: String, model_id: String, source: Option<String>) {
        let app_data_dir = self.app_data_dir.clone();
        let install_tasks = self.install_tasks.clone();
        let config_lock = self.config.clone();
        let registry = self.registry.read().await.clone();
        let manifest = download::find_model(&registry, &model_id).cloned();

        let Some(manifest) = manifest else {
            let mut tasks = install_tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = "failed".to_string();
                task.error = Some(format!("Model not found: {}", model_id));
            }
            return;
        };

        tokio::spawn(async move {
            let mut pref = config_lock
                .read()
                .await
                .as_ref()
                .map(|c| c.download.clone())
                .unwrap_or_default();

            if let Some(src) = source {
                pref.source_preference = src;
            }

            {
                let mut tasks = install_tasks.write().await;
                if let Some(task) = tasks.get_mut(&task_id) {
                    task.progress = 1;
                    task.message = Some("Downloading model...".to_string());
                }
            }

            // 进度回调：将下载进度映射到 1-95 范围
            let tasks_for_progress = install_tasks.clone();
            let tid_for_progress = task_id.clone();
            let on_progress: download::ProgressCallback = Box::new(move |downloaded, total| {
                let pct = if let Some(total) = total {
                    if total > 0 {
                        ((downloaded as f64 / total as f64) * 94.0) as i32 + 1
                    } else {
                        1
                    }
                } else {
                    // 无 Content-Length 时用对数增长模拟
                    let mb = downloaded as f64 / (1024.0 * 1024.0);
                    (mb.ln().max(0.0) * 15.0).min(90.0) as i32 + 1
                };
                // 非阻塞更新：try_write 避免阻塞下载线程
                if let Ok(mut tasks) = tasks_for_progress.try_write() {
                    if let Some(task) = tasks.get_mut(&tid_for_progress) {
                        task.progress = pct.clamp(1, 95);
                    }
                }
            });

            match download::download_model_with_progress(
                &app_data_dir, &manifest, &pref, Some(on_progress),
            ).await {
                Ok(_) => {
                    let mut tasks = install_tasks.write().await;
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.progress = 100;
                        task.status = "completed".to_string();
                        task.message = Some("Model installed".to_string());
                    }
                }
                Err(e) => {
                    let mut tasks = install_tasks.write().await;
                    if let Some(task) = tasks.get_mut(&task_id) {
                        task.status = "failed".to_string();
                        task.error = Some(e);
                    }
                }
            }
        });
    }

    /// 获取安装任务状态
    pub async fn get_install_task(&self, task_id: &str) -> Option<ModelInstallTaskSnapshot> {
        self.install_tasks.read().await.get(task_id).cloned()
    }

    // ========== 模型激活 ==========

    /// 激活模型
    pub async fn activate_model(
        &self,
        db: SqlitePool,
        state_manager: &StateManager,
        model_id: &str,
    ) -> Result<(), String> {
        let registry = self.registry.read().await.clone();
        let manifest = download::find_model(&registry, model_id)
            .ok_or_else(|| format!("Unsupported model_id: {}", model_id))?
            .clone();

        if !download::is_model_installed(&self.app_data_dir, &manifest) {
            return Err(format!("Model not installed: {}", model_id));
        }

        // 设置重建中状态
        state_manager.set_rebuilding(true).await;

        // 更新配置
        {
            let mut config_guard = self.config.write().await;
            let config = config_guard
                .as_mut()
                .ok_or_else(|| "vision config not loaded".to_string())?;
            config.active_model_id = model_id.to_string();
            download::save_runtime_config(&self.app_data_dir, config).await?;
        }

        // 更新索引元信息
        let store = crate::vision::store::VisionStore::new(db);
        store
            .ensure_index_meta(&manifest.model_version)
            .await
            .map_err(|e| e.to_string())?;

        let index_version = store
            .bump_index_meta_for_model(&manifest.model_version)
            .await
            .map_err(|e| e.to_string())?;

        let index_size = store
            .count_embeddings_by_model(&manifest.model_version)
            .await
            .map_err(|e| e.to_string())?;

        // 更新状态
        state_manager
            .set(VisionStatusSnapshot {
                model_id: manifest.model_id,
                model_version: manifest.model_version,
                index_version,
                index_size,
                is_ready: false,
                is_rebuilding: false,
                last_rebuild_at: None,
                reason: Some("VISION_REBUILD_REQUIRED".to_string()),
                rebuild_processed: 0,
                rebuild_total: 0,
            })
            .await;

        Ok(())
    }

    // ========== 模型删除 ==========

    /// 删除模型
    pub async fn delete_model(&self, active_model_id: &str, model_id: &str) -> Result<(), String> {
        if active_model_id == model_id {
            return Err("Cannot delete active model".to_string());
        }

        let registry = self.registry.read().await.clone();
        let manifest = download::find_model(&registry, model_id)
            .ok_or_else(|| format!("Unsupported model_id: {}", model_id))?
            .clone();

        if manifest.sources.is_empty() {
            return Err("Built-in model cannot be deleted".to_string());
        }

        let model_path = download::model_abs_path(&self.app_data_dir, &manifest);
        if !model_path.exists() {
            return Err("Model not installed".to_string());
        }

        tokio::fs::remove_file(model_path)
            .await
            .map_err(|e| e.to_string())
    }

    // ========== 配置访问 ==========

    pub fn app_data_dir(&self) -> &PathBuf {
        &self.app_data_dir
    }

    pub async fn get_active_model_id(&self) -> String {
        self.config
            .read()
            .await
            .as_ref()
            .map(|c| c.active_model_id.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub async fn get_manifest_for_model(&self, model_id: &str) -> Option<download::ModelManifest> {
        let registry = self.registry.read().await;
        download::find_model(&registry, model_id).cloned()
    }
}
