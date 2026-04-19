use std::fs;
use std::path::PathBuf;
use rand::Rng;
// [修改 1] DragDropEvent 现在直接在 tauri 模块下，WindowEvent 也建议引入
use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

mod api;
mod db;
mod server;
mod state;
mod utils;
mod vision;
mod web;

// Tauri 命令：获取后端 URL
#[tauri::command]
fn get_backend_url(backend_url: tauri::State<String>) -> String {
    backend_url.inner().clone()
}

fn resolve_app_data_dir(default_app_data_dir: PathBuf) -> PathBuf {
    #[cfg(debug_assertions)]
    {
        let suffix = std::env::var("BOOTH_APP_DATA_SUFFIX")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-dev".to_string());

        let parent = default_app_data_dir
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| default_app_data_dir.clone());

        let dir_name = default_app_data_dir
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| format!("{name}{suffix}"))
            .unwrap_or_else(|| format!("booth-tool{suffix}"));

        return parent.join(dir_name);
    }

    #[cfg(not(debug_assertions))]
    {
        default_app_data_dir
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        //.plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![get_backend_url])
        .setup(|app| {
            // 1. 获取 AppHandle (Tauri v2 推荐方式)
            let app_handle = app.handle();

            // 调试：监听桌面端文件拖放事件，验证 webview 是否能收到 file-drop
            if let Some(main) = app_handle.get_webview_window("main") {
                // 调试：打印窗口事件，并在检测到文件拖放时向前端发送自定义事件
                let main_clone = main.clone();
                main.on_window_event(move |event| {
                    // 过滤掉频繁的事件，避免日志刷屏（可选）
                    // println!("[Debug][WindowEvent] {:?}", event);

                    if let WindowEvent::DragDrop(drop_event) = event {
                        // [修改 2] v2 中变体名称由 Dropped 改为 Drop
                        match drop_event {
                            DragDropEvent::Drop { paths, position } => {
                                println!(
                                    "[Debug][FileDrop][Backend] paths: {:?} @ {:?}",
                                    paths, position
                                );
                                // 将文件路径推送到前端
                                let _ = main_clone.emit("boothpack-file-drop", paths.clone());
                            }
                            // 处理其他拖拽状态（如 Enter, Over, Leave）以免编译警告
                            _ => {}
                        }
                    }
                });
            }

            // -------------------------------------------------------------
            // [优化点 1] 统一路径策略
            // -------------------------------------------------------------

            // A. 获取系统数据目录
            // 注意：需要在 tauri.conf.json 中配置 "identifier"，否则可能会报错
            let default_app_data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            let app_data_dir = resolve_app_data_dir(default_app_data_dir);

            // B. 上传文件目录
            let upload_dir = app_data_dir.join("uploads");

            println!("[Config] Database Path: {:?}", app_data_dir);
            println!("[Config] Uploads Path : {:?}", upload_dir);
            #[cfg(debug_assertions)]
            println!("[Config] Data Mode    : development");
            #[cfg(not(debug_assertions))]
            println!("[Config] Data Mode    : production");

            // 2. 确保目录存在
            if !upload_dir.exists() {
                fs::create_dir_all(&upload_dir).expect("Failed to create uploads directory");
            }

            // 3. 初始化数据库
            let db_pool = tauri::async_runtime::block_on(async {
                db::init_db(&app_data_dir)
                    .await
                    .expect("Database initialization failed")
            });

            // 4. 读取或生成 JWT Secret（持久化到 app data 目录）
            let jwt_secret = {
                let secret_path = app_data_dir.join("jwt_secret.key");
                if secret_path.exists() {
                    fs::read_to_string(&secret_path)
                        .expect("Failed to read jwt_secret.key")
                        .trim()
                        .to_string()
                } else {
                    let secret: String = rand::thread_rng()
                        .sample_iter(&rand::distributions::Alphanumeric)
                        .take(64)
                        .map(char::from)
                        .collect();
                    fs::write(&secret_path, &secret)
                        .expect("Failed to write jwt_secret.key");
                    println!("[Config] Generated new JWT secret");
                    secret
                }
            };

            // 5. 构建 AppState
            let state = state::AppState {
                db: db_pool.clone(),
                upload_dir: upload_dir.clone(),
                jwt_secret,
                vision_runtime: std::sync::Arc::new(vision::VisionRuntime::new(
                    app_data_dir.clone(),
                    upload_dir.clone(),
                    db_pool.clone(),
                )),
            };

            // 初始化 ONNX Runtime 动态库路径
            let resource_dir = app_handle.path().resource_dir().ok();
            #[cfg(feature = "vision")]
            {
                // Android: .so 在 jniLibs 中，dlopen 自动找到，不需要设 ORT_DYLIB_PATH
                // Windows/macOS/Linux: 从 resources 目录加载
                #[cfg(not(target_os = "android"))]
                if let Some(ref res_dir) = resource_dir {
                    let lib_name = if cfg!(target_os = "windows") {
                        "onnxruntime.dll"
                    } else if cfg!(target_os = "macos") {
                        "libonnxruntime.dylib"
                    } else {
                        "libonnxruntime.so"
                    };
                    let ort_lib_path = res_dir.join(lib_name);
                    if ort_lib_path.exists() {
                        std::env::set_var("ORT_DYLIB_PATH", &ort_lib_path);
                        println!("[Vision] ORT_DYLIB_PATH set to: {:?}", ort_lib_path);
                    } else {
                        println!("[Vision] ORT library not found at: {:?}, will use system default", ort_lib_path);
                    }
                }

                #[cfg(target_os = "android")]
                println!("[Vision] Android: using system linker for libonnxruntime.so");
            }

            // 内嵌模型释放：从 Tauri 资源目录复制到 AppData
            tauri::async_runtime::block_on(async {
                // 先确保配置文件存在（bootstrap 内部也会调，但复制模型需要先有 registry）
                if let Err(e) = vision::download::ensure_default_files(&app_data_dir).await {
                    eprintln!("[Vision] ensure_default_files failed: {}", e);
                }
                if let Err(e) = vision::download::install_builtin_models(
                    &app_data_dir,
                    resource_dir.as_deref(),
                ).await {
                    eprintln!("[Vision] install_builtin_models failed: {}", e);
                }
            });

            if let Err(e) =
                tauri::async_runtime::block_on(state.vision_runtime.bootstrap(state.db.clone()))
            {
                eprintln!("[Vision] bootstrap failed: {}", e);
            }

            // 获取后端 URL
            let backend_url = std::env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:5140".to_string());

            println!("[Config] Backend URL  : {}", backend_url);

            app.manage(backend_url.clone());

            // -------------------------------------------------------------
            // [优化点 2] 使用 Tauri 内置异步运行时
            // -------------------------------------------------------------
            tauri::async_runtime::spawn(async move {
                println!("[Booth Tool] Starting HTTP server on port 5140...");
                server::start_server(state, 5140).await;
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
