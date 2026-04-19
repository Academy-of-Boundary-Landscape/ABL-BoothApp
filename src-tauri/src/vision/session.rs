//! ONNX 会话管理模块
//!
//! 使用 ort (ONNX Runtime) 作为推理后端

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use ndarray::Array4;
use ort::session::Session;

use crate::vision::download::ModelManifest;

/// ONNX 推理会话（带性能统计）
///
/// `Session::run` 在 ort 2.x-rc 中需要 `&mut self`，
/// 所以用 Mutex 包装以支持从 `&self` 调用。
pub struct OnnxSession {
    model_id: String,
    model_version: String,
    model_path: String,
    manifest: ModelManifest,
    session: Mutex<Session>,
    inference_count: AtomicU64,
    inference_total_us: AtomicU64,
}

/// 当前使用的执行设备名称（加载后记录，供前端查询）
static ACTIVE_EP_NAME: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// 系统 GPU 列表（启动时探测一次）
static GPU_DEVICES: std::sync::OnceLock<Vec<GpuDevice>> = std::sync::OnceLock::new();

#[derive(Debug, Clone, serde::Serialize)]
pub struct GpuDevice {
    pub device_id: i32,
    pub name: String,
}

/// 探测系统 GPU 列表（通过 DXGI 枚举，与 DirectML device_id 顺序完全一致）
pub fn probe_gpu_devices() -> Vec<GpuDevice> {
    GPU_DEVICES.get_or_init(|| {
        let mut devices = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

            let factory: Result<IDXGIFactory1, _> = unsafe { CreateDXGIFactory1() };
            if let Ok(factory) = factory {
                let mut idx = 0u32;
                loop {
                    match unsafe { factory.EnumAdapters1(idx) } {
                        Ok(adapter) => {
                            if let Ok(desc) = unsafe { adapter.GetDesc1() } {
                                // desc.Description 是 [u16; 128]，转成 String
                                let name_len = desc.Description.iter().position(|&c| c == 0).unwrap_or(128);
                                let name = String::from_utf16_lossy(&desc.Description[..name_len]);
                                devices.push(GpuDevice {
                                    device_id: idx as i32,
                                    name,
                                });
                            }
                            idx += 1;
                        }
                        Err(_) => break, // 枚举完毕
                    }
                }
            }

            if devices.is_empty() {
                println!("[Vision] No DXGI adapters found");
            } else {
                for d in &devices {
                    println!("[Vision] DXGI adapter {}: {}", d.device_id, d.name);
                }
            }
        }

        devices
    }).clone()
}

/// 获取当前推理设备名称
pub fn get_active_ep_name() -> String {
    ACTIVE_EP_NAME.read().map(|s| s.clone()).unwrap_or_else(|_| "unknown".to_string())
}

fn set_active_ep_name(name: &str) {
    if let Ok(mut w) = ACTIVE_EP_NAME.write() {
        *w = name.to_string();
    }
}

impl OnnxSession {
    /// 从文件加载 ONNX 模型
    ///
    /// ep_pref: "auto" | "cpu" | "gpu:0" | "gpu:1" ... | "nnapi"
    pub fn load(model_path: &Path, manifest: &ModelManifest, ep_pref: &str) -> Result<Self, String> {
        let t0 = Instant::now();

        let (session, ep_name) = match ep_pref {
            "cpu" => {
                let s = Self::load_cpu_only(model_path)?;
                (s, "CPU".to_string())
            }
            #[cfg(target_os = "windows")]
            pref if pref.starts_with("gpu:") => {
                let device_id: i32 = pref.trim_start_matches("gpu:").parse().unwrap_or(0);
                Self::try_load_gpu_device(model_path, device_id)
                    .or_else(|e| {
                        println!("[Vision] GPU device {} failed, falling back to CPU: {}", device_id, e);
                        Self::load_cpu_only(model_path).map(|s| (s, "CPU (fallback)".to_string()))
                    })?
            }
            "nnapi" => {
                Self::try_load_nnapi(model_path)
                    .or_else(|e| {
                        println!("[Vision] NNAPI failed, falling back to CPU: {}", e);
                        Self::load_cpu_only(model_path).map(|s| (s, "CPU (fallback)".to_string()))
                    })?
            }
            _ => {
                // "auto": 平台自适应加速 → CPU fallback
                Self::try_load_accelerated(model_path)
                    .or_else(|e| {
                        println!("[Vision] Accelerated load failed, falling back to CPU: {}", e);
                        Self::load_cpu_only(model_path).map(|s| (s, "CPU".to_string()))
                    })?
            }
        };

        let load_ms = t0.elapsed().as_millis();
        set_active_ep_name(&ep_name);
        println!(
            "[Vision] Model loaded: {} ({}) in {}ms [EP: {}]",
            manifest.model_id,
            model_path.display(),
            load_ms,
            ep_name,
        );

        Ok(Self {
            model_id: manifest.model_id.clone(),
            model_version: manifest.model_version.clone(),
            model_path: model_path.to_string_lossy().to_string(),
            manifest: manifest.clone(),
            session: Mutex::new(session),
            inference_count: AtomicU64::new(0),
            inference_total_us: AtomicU64::new(0),
        })
    }

    /// 尝试指定 device_id 的 GPU
    fn try_load_gpu_device(model_path: &Path, device_id: i32) -> Result<(Session, String), String> {
        #[cfg(target_os = "windows")]
        {
            use ort::execution_providers::ExecutionProvider;
            let mut builder = Session::builder()
                .and_then(|b| b.with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3))
                .and_then(|b| b.with_intra_threads(4))
                .map_err(|e| e.to_string())?;

            let dml = ort::execution_providers::DirectMLExecutionProvider::default()
                .with_device_id(device_id);
            dml.register(&mut builder).map_err(|e| e.to_string())?;

            let session = builder.commit_from_file(model_path).map_err(|e| e.to_string())?;
            // 查找设备名称
            let dev_name = probe_gpu_devices()
                .iter()
                .find(|d| d.device_id == device_id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| format!("device {}", device_id));
            let ep_name = format!("DirectML ({})", dev_name);
            println!("[Vision] {} — loaded successfully", ep_name);
            return Ok((session, ep_name));
        }

        #[cfg(not(target_os = "windows"))]
        Err("GPU not supported on this platform".to_string())
    }

    /// 自动选择平台最佳加速方案
    /// Windows: DirectML (遍历 GPU) → 失败返回 Err
    /// Android: NNAPI → 失败返回 Err
    /// 其他平台: 直接返回 Err（由调用方 fallback 到 CPU）
    fn try_load_accelerated(model_path: &Path) -> Result<(Session, String), String> {
        // Windows: 遍历 DirectML GPU 设备
        #[cfg(target_os = "windows")]
        {
            let devices = probe_gpu_devices();
            if devices.is_empty() {
                return Err("No GPU adapter found".to_string());
            }
            for dev in &devices {
                let lower = dev.name.to_lowercase();
                if lower.contains("virtual") || lower.contains("basic") || lower.contains("remote") || lower.contains("microsoft") {
                    println!("[Vision] Skipping virtual adapter {}: {}", dev.device_id, dev.name);
                    continue;
                }
                match Self::try_load_gpu_device(model_path, dev.device_id) {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        println!("[Vision] DirectML device {} ({}) failed: {}", dev.device_id, dev.name, e);
                    }
                }
            }
            return Err("No DirectML GPU available".to_string());
        }

        // Android: 尝试 NNAPI
        #[cfg(target_os = "android")]
        {
            return Self::try_load_nnapi(model_path);
        }

        // 其他平台: 无加速
        #[allow(unreachable_code)]
        Err("No accelerator available on this platform".to_string())
    }

    /// 尝试 NNAPI 加速（Android NPU/GPU/DSP）
    fn try_load_nnapi(model_path: &Path) -> Result<(Session, String), String> {
        #[cfg(target_os = "android")]
        {
            use ort::execution_providers::ExecutionProvider;
            let mut builder = Session::builder()
                .and_then(|b| b.with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3))
                .and_then(|b| b.with_intra_threads(4))
                .map_err(|e| e.to_string())?;

            match ort::execution_providers::NNAPIExecutionProvider::default()
                .register(&mut builder)
            {
                Ok(_) => println!("[Vision] NNAPI EP registered"),
                Err(e) => return Err(format!("NNAPI registration failed: {}", e)),
            }

            let session = builder.commit_from_file(model_path).map_err(|e| e.to_string())?;
            let ep_name = "NNAPI (NPU/GPU)".to_string();
            println!("[Vision] {} — loaded successfully", ep_name);
            return Ok((session, ep_name));
        }

        #[cfg(not(target_os = "android"))]
        Err("NNAPI is only available on Android".to_string())
    }

    /// 纯 CPU 加载
    fn load_cpu_only(model_path: &Path) -> Result<Session, String> {
        println!("[Vision] Loading model with CPU only...");
        Session::builder()
            .map_err(|e| e.to_string())?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| e.to_string())?
            .with_intra_threads(4)
            .map_err(|e| e.to_string())?
            .commit_from_file(model_path)
            .map_err(|e| e.to_string())
    }

    /// 检查会话是否匹配指定的模型
    pub fn matches(&self, model_id: &str, model_version: &str, model_path: &Path) -> bool {
        self.model_id == model_id
            && self.model_version == model_version
            && self.model_path == model_path.to_string_lossy()
    }

    /// 执行图像 embedding
    pub fn embed(&self, image_bytes: &[u8]) -> Result<Vec<f32>, String> {
        let t0 = Instant::now();

        let chw = crate::vision::model::preprocess(image_bytes, &self.manifest)?;
        let preprocess_us = t0.elapsed().as_micros();

        let size = self.manifest.input_size;
        let input_array = Array4::from_shape_vec((1, 3, size, size), chw)
            .map_err(|e| format!("Failed to create input array: {}", e))?;

        // 用 shape + raw data 构造 ort Tensor，避免 ndarray 版本不匹配
        let shape = vec![1_i64, 3, size as i64, size as i64];
        let input_value = ort::value::Value::from_array(
            (shape, input_array.into_raw_vec_and_offset().0.into_boxed_slice())
        ).map_err(|e| format!("Failed to create input value: {}", e))?;

        let t1 = Instant::now();
        let mut session = self.session.lock().map_err(|e| format!("Session lock poisoned: {}", e))?;
        let outputs = session
            .run(ort::inputs!["image" => input_value])
            .map_err(|e| format!("Inference failed: {}", e))?;
        let inference_us = t1.elapsed().as_micros();

        let output = outputs
            .iter()
            .next()
            .map(|(_, v)| v)
            .ok_or_else(|| "Model has no output tensor".to_string())?;

        let output_tensor = output
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract output: {}", e))?;

        let mut out: Vec<f32> = output_tensor.1.to_vec();
        if out.len() < self.manifest.embed_dim {
            return Err(format!(
                "Output dim {} < expected {}. Wrong model file?",
                out.len(), self.manifest.embed_dim
            ));
        }
        if out.len() > self.manifest.embed_dim {
            out.truncate(self.manifest.embed_dim);
        }

        let total_us = t0.elapsed().as_micros() as u64;
        let count = self.inference_count.fetch_add(1, Ordering::Relaxed) + 1;
        let cumulative = self.inference_total_us.fetch_add(total_us, Ordering::Relaxed) + total_us;
        let avg_ms = (cumulative as f64 / count as f64) / 1000.0;

        println!(
            "[Vision] Embed #{}: preprocess={}us, inference={}us, total={}ms (avg={:.1}ms)",
            count, preprocess_us, inference_us, total_us / 1000, avg_ms,
        );

        Ok(crate::vision::model::l2_normalize(out))
    }
}

/// 会话缓存管理器
pub struct SessionCache {
    cache: tokio::sync::RwLock<Option<Arc<OnnxSession>>>,
}

impl SessionCache {
    pub fn new() -> Self {
        Self {
            cache: tokio::sync::RwLock::new(None),
        }
    }

    pub async fn get_or_load(
        &self,
        loader: impl FnOnce() -> Result<OnnxSession, String>,
    ) -> Result<Arc<OnnxSession>, String> {
        {
            let guard = self.cache.read().await;
            if let Some(session) = guard.as_ref() {
                return Ok(session.clone());
            }
        }
        let mut guard = self.cache.write().await;
        if let Some(session) = guard.as_ref() {
            return Ok(session.clone());
        }
        let loaded = Arc::new(loader()?);
        *guard = Some(loaded.clone());
        Ok(loaded)
    }

    pub async fn get_or_load_with_check(
        &self,
        model_id: &str,
        model_version: &str,
        model_path: &Path,
        manifest: ModelManifest,
        ep_pref: String,
    ) -> Result<Arc<OnnxSession>, String> {
        {
            let guard = self.cache.read().await;
            if let Some(session) = guard.as_ref() {
                if session.matches(model_id, model_version, model_path) {
                    return Ok(session.clone());
                }
            }
        }
        let mut guard = self.cache.write().await;
        if let Some(existing) = guard.as_ref() {
            if existing.matches(model_id, model_version, model_path) {
                return Ok(existing.clone());
            }
        }
        let path_owned = model_path.to_path_buf();
        let manifest_owned = manifest.clone();
        let ep_pref_owned = ep_pref;
        let loaded = tokio::task::spawn_blocking(move || {
            OnnxSession::load(&path_owned, &manifest_owned, &ep_pref_owned)
        })
        .await
        .map_err(|e| format!("spawn_blocking join error: {}", e))?
        .map(Arc::new)?;
        *guard = Some(loaded.clone());
        Ok(loaded)
    }

    pub async fn clear(&self) {
        let mut guard = self.cache.write().await;
        *guard = None;
    }
}

impl Default for SessionCache {
    fn default() -> Self {
        Self::new()
    }
}
