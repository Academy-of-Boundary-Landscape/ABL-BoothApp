use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};

const DEFAULT_REGISTRY_JSON: &str = include_str!("../../config/vision/model_registry.json");
const DEFAULT_RUNTIME_JSON: &str = include_str!("../../config/vision/vision_model.json");

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelRegistry {
    pub models: Vec<ModelManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub model_id: String,
    pub model_version: String,
    pub onnx_rel_path: String,
    pub input_size: usize,
    pub embed_dim: usize,
    pub mean: [f32; 3],
    pub std: [f32; 3],
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub size_mb: Option<f64>,
    #[serde(default)]
    pub sources: Vec<ModelSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSource {
    pub source: String,
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadPreference {
    pub source_preference: String,
    pub github_proxy_prefix: String,
    pub hf_base_url: String,
}

impl Default for DownloadPreference {
    fn default() -> Self {
        Self {
            source_preference: "auto".to_string(),
            github_proxy_prefix: String::new(),
            hf_base_url: "https://huggingface.co".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionThresholdItem {
    pub top1_min: f32,
    pub top1_top2_gap_min: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionThresholdConfig {
    pub default_top1_min: f32,
    pub default_top1_top2_gap_min: f32,
    #[serde(default)]
    pub by_mode: std::collections::HashMap<String, VisionThresholdItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRuntimeConfig {
    pub max_concurrency: usize,
    pub timeout_ms: u64,
    /// 推理设备选择: "auto" (GPU优先降级CPU) | "cpu" | "gpu:0" | "gpu:1" ...
    #[serde(default = "default_ep")]
    pub execution_provider: String,
}

fn default_ep() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionModelConfig {
    pub active_model_id: String,
    pub download: DownloadPreference,
    pub thresholds: VisionThresholdConfig,
    pub runtime: VisionRuntimeConfig,
}

pub fn model_root_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("models").join("vision")
}

pub fn registry_path(app_data_dir: &Path) -> PathBuf {
    model_root_dir(app_data_dir).join("model_registry.json")
}

pub fn runtime_config_path(app_data_dir: &Path) -> PathBuf {
    model_root_dir(app_data_dir).join("vision_model.json")
}

/// 从 Tauri 资源目录复制内嵌模型到 AppData（首次启动时）
pub async fn install_builtin_models(
    app_data_dir: &Path,
    resource_dir: Option<&Path>,
) -> Result<(), String> {
    let Some(res_dir) = resource_dir else { return Ok(()) };

    let registry = load_registry(app_data_dir).await.unwrap_or_default();
    for model in &registry.models {
        if model.tier.as_deref() != Some("builtin") {
            continue;
        }
        let target = model_abs_path(app_data_dir, model);
        if target.exists() {
            continue; // 已安装，跳过
        }
        // 从资源目录复制
        let source = res_dir.join("models").join("vision").join(&model.onnx_rel_path);
        if source.exists() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
            }
            fs::copy(&source, &target).await.map_err(|e| {
                format!("copy builtin model {}: {}", model.model_id, e)
            })?;
            println!("[Vision] Installed builtin model: {} -> {:?}", model.model_id, target);
        }
    }
    Ok(())
}

pub async fn ensure_default_files(app_data_dir: &Path) -> Result<(), String> {
    let root = model_root_dir(app_data_dir);
    if !root.exists() {
        fs::create_dir_all(&root).await.map_err(|e| e.to_string())?;
    }

    // model_registry.json 是静态元数据（模型目录、预处理参数等），
    // 每次启动都用内嵌版本强制覆盖，确保新版本的模型定义总能生效。
    // 用户不应该手动编辑这个文件。
    let registry_file = registry_path(app_data_dir);
    let needs_write = match fs::read_to_string(&registry_file).await {
        Ok(existing) => existing != DEFAULT_REGISTRY_JSON,
        Err(_) => true,
    };
    if needs_write {
        fs::write(&registry_file, DEFAULT_REGISTRY_JSON)
            .await
            .map_err(|e| e.to_string())?;
    }

    // vision_model.json 是用户运行时配置（激活模型、阈值、下载偏好等），
    // 只在不存在时写入默认值，不覆盖用户的自定义设置。
    let runtime_file = runtime_config_path(app_data_dir);
    if !runtime_file.exists() {
        fs::write(&runtime_file, DEFAULT_RUNTIME_JSON)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub async fn load_registry(app_data_dir: &Path) -> Result<ModelRegistry, String> {
    let raw = fs::read_to_string(registry_path(app_data_dir))
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_str::<ModelRegistry>(&raw).map_err(|e| e.to_string())
}

pub async fn load_runtime_config(app_data_dir: &Path) -> Result<VisionModelConfig, String> {
    let raw = fs::read_to_string(runtime_config_path(app_data_dir))
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_str::<VisionModelConfig>(&raw).map_err(|e| e.to_string())
}

pub async fn save_runtime_config(
    app_data_dir: &Path,
    cfg: &VisionModelConfig,
) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(runtime_config_path(app_data_dir), raw)
        .await
        .map_err(|e| e.to_string())
}

pub fn find_model<'a>(registry: &'a ModelRegistry, model_id: &str) -> Option<&'a ModelManifest> {
    registry.models.iter().find(|m| m.model_id == model_id)
}

pub fn model_abs_path(app_data_dir: &Path, manifest: &ModelManifest) -> PathBuf {
    model_root_dir(app_data_dir).join(&manifest.onnx_rel_path)
}

pub fn is_model_installed(app_data_dir: &Path, manifest: &ModelManifest) -> bool {
    model_abs_path(app_data_dir, manifest).exists()
}

/// 进度回调类型：参数为 (已下载字节, 总字节 Option)
pub type ProgressCallback = Box<dyn Fn(u64, Option<u64>) + Send + Sync>;

pub async fn download_model(
    app_data_dir: &Path,
    manifest: &ModelManifest,
    pref: &DownloadPreference,
) -> Result<(), String> {
    download_model_with_progress(app_data_dir, manifest, pref, None).await
}

pub async fn download_model_with_progress(
    app_data_dir: &Path,
    manifest: &ModelManifest,
    pref: &DownloadPreference,
    on_progress: Option<ProgressCallback>,
) -> Result<(), String> {
    if manifest.sources.is_empty() {
        return Err(format!(
            "No download sources for model {}",
            manifest.model_id
        ));
    }

    let ordered_sources = source_order(manifest, pref);
    if ordered_sources.is_empty() {
        return Err(format!(
            "No available source for model {}",
            manifest.model_id
        ));
    }

    let target = model_abs_path(app_data_dir, manifest);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

    let temp = target.with_extension("onnx.part");
    let client = reqwest::Client::new();

    let mut last_error: Option<String> = None;
    for src in ordered_sources {
        let final_url = apply_source_rewrite(&src.url, &src.source, pref);
        match download_and_verify(&client, &final_url, &temp, &src.sha256, &on_progress).await {
            Ok(_) => {
                fs::rename(&temp, &target)
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
            Err(e) => {
                last_error = Some(format!("{}: {}", src.source, e));
                let _ = fs::remove_file(&temp).await;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "download failed".to_string()))
}

fn source_order<'a>(
    manifest: &'a ModelManifest,
    pref: &DownloadPreference,
) -> Vec<&'a ModelSource> {
    if pref.source_preference != "auto" {
        let mut first = manifest
            .sources
            .iter()
            .filter(|s| s.source == pref.source_preference)
            .collect::<Vec<_>>();
        let mut rest = manifest
            .sources
            .iter()
            .filter(|s| s.source != pref.source_preference)
            .collect::<Vec<_>>();
        first.append(&mut rest);
        first
    } else {
        manifest.sources.iter().collect::<Vec<_>>()
    }
}

fn apply_source_rewrite(url: &str, source: &str, pref: &DownloadPreference) -> String {
    if source == "github" && !pref.github_proxy_prefix.is_empty() {
        return format!("{}{}", pref.github_proxy_prefix, url);
    }

    if (source == "hf" || source == "hf_mirror") && !pref.hf_base_url.is_empty() {
        return url.replacen("https://huggingface.co", &pref.hf_base_url, 1);
    }

    url.to_string()
}

async fn download_and_verify(
    client: &reqwest::Client,
    url: &str,
    out_path: &Path,
    expected_sha256: &str,
    on_progress: &Option<ProgressCallback>,
) -> Result<(), String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("http status {}", resp.status()));
    }

    let total_size = resp.content_length();

    let mut file = fs::File::create(out_path)
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        if let Some(cb) = on_progress {
            cb(downloaded, total_size);
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;

    if !expected_sha256.is_empty() {
        let got = file_sha256_hex(out_path).await?;
        if !got.eq_ignore_ascii_case(expected_sha256) {
            return Err("sha256 mismatch".to_string());
        }
    }

    Ok(())
}

async fn file_sha256_hex(path: &Path) -> Result<String, String> {
    use tokio::io::AsyncReadExt;
    let mut file = fs::File::open(path).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024]; // 64KB chunks
    loop {
        let n = file.read(&mut buf).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
