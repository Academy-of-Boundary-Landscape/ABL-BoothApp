use rand::Rng;
use std::fs;
use std::path::PathBuf;
// [修改 1] DragDropEvent 现在直接在 tauri 模块下，WindowEvent 也建议引入
use tauri::{DragDropEvent, Emitter, Manager, WindowEvent};

mod api;
mod db;
mod domain;
mod error;
mod server;
mod state;
#[cfg(test)]
pub mod test_support;
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

        parent.join(dir_name)
    }

    #[cfg(not(debug_assertions))]
    {
        default_app_data_dir
    }
}

// Tauri 命令：日志文件所在目录（「关于」页面显示，出问题时让用户把日志发过来）
#[tauri::command]
fn get_log_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_log_dir()
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

/// 日志落盘。release 构建没有控制台（Windows 的 windows_subsystem、Android 同理），
/// 以前的 println!/eprintln! 在用户机器上全部丢失——2026-09-25 真机闪退时，是靠用户从
/// cmd 重定向才拿到 panic 信息的。现在同时写 stdout（dev 照常看）和系统日志目录，
/// 单个文件 5MB 轮转、保留 5 份。
fn log_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("booth".into()),
            }),
        ])
        .level(log::LevelFilter::Info)
        // 窗口/网页引擎的 info 日志很吵，只留警告以上
        .level_for("tao", log::LevelFilter::Warn)
        .level_for("wry", log::LevelFilter::Warn)
        .max_file_size(5 * 1024 * 1024)
        .rotation_strategy(RotationStrategy::KeepSome(5))
        .build()
}

/// panic 也进日志文件：默认的 panic 输出只去 stderr，release 里谁都看不到。
fn install_panic_logger() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!(
            "PANIC: {info}\n{}",
            std::backtrace::Backtrace::force_capture()
        );
        default_hook(info);
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_logger();
    let builder = tauri::Builder::default()
        // 放在第一个：之后所有插件与 setup 里的日志都能落盘
        .plugin(log_plugin())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .invoke_handler(tauri::generate_handler![get_backend_url, get_log_dir])
        .setup(|app| {
            // 1. 获取 AppHandle (Tauri v2 推荐方式)
            let app_handle = app.handle();

            // 调试：监听桌面端文件拖放事件，验证 webview 是否能收到 file-drop
            if let Some(main) = app_handle.get_webview_window("main") {
                // 调试：打印窗口事件，并在检测到文件拖放时向前端发送自定义事件
                let main_clone = main.clone();
                main.on_window_event(move |event| {
                    // 过滤掉频繁的事件，避免日志刷屏（可选）
                    // log::info!("[Debug][WindowEvent] {:?}", event);

                    // [修改 2] v2 中变体名称由 Dropped 改为 Drop
                    if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, position }) = event {
                        log::info!(
                            "[Debug][FileDrop][Backend] paths: {:?} @ {:?}",
                            paths, position
                        );
                        // 将文件路径推送到前端
                        let _ = main_clone.emit("boothpack-file-drop", paths.clone());
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

            log::info!("[Config] Database Path: {:?}", app_data_dir);
            log::info!("[Config] Uploads Path : {:?}", upload_dir);
            #[cfg(debug_assertions)]
            log::info!("[Config] Data Mode    : development");
            #[cfg(not(debug_assertions))]
            log::info!("[Config] Data Mode    : production");

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
                    fs::write(&secret_path, &secret).expect("Failed to write jwt_secret.key");
                    log::info!("[Config] Generated new JWT secret");
                    secret
                }
            };

            // 5. 绑定端口。首选 5140 / 5141，被占就依次回退（server::*_port_candidates）。
            //    端口写死的年代，5140 被别的程序占了后端就起不来，而窗口照常打开、
            //    所有请求静默失败（2026-09-25 真机上是 VS Code 的端口自动转发）。
            let bound = server::bind_first_available(
                std::net::Ipv4Addr::LOCALHOST,
                &server::http_port_candidates(),
            )
            .and_then(|http| {
                server::bind_first_available(
                    std::net::Ipv4Addr::UNSPECIFIED,
                    &server::https_port_candidates(),
                )
                .map(|https| (http, https))
            });
            let (listeners, http_port, lan_https_port) = match bound {
                Ok(((http_l, http_p), (https_l, https_p))) => (Some((http_l, https_l)), http_p, https_p),
                Err(e) => {
                    log::error!("[Booth Tool] FATAL: cannot bind server ports: {e}");
                    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                    app_handle
                        .dialog()
                        .message(format!(
                            "摊盒的本地服务启动失败：{e}\n\n请关闭占用这些端口的程序（或已经打开的另一个摊盒）后重新启动。"
                        ))
                        .kind(MessageDialogKind::Error)
                        .title("无法启动本地服务")
                        .show(|_| {});
                    (None, server::HTTP_PORT, server::HTTPS_PORT)
                }
            };

            // 6. 构建 AppState
            let state = state::AppState {
                db: db_pool.clone(),
                upload_dir: upload_dir.clone(),
                app_data_dir: app_data_dir.clone(),
                jwt_secret,
                vision_runtime: std::sync::Arc::new(vision::VisionRuntime::new(
                    app_data_dir.clone(),
                    upload_dir.clone(),
                    db_pool.clone(),
                )),
                lan_https_port,
            };

            // 先加载 ONNX Runtime 动态库，再做任何会碰 ort 的事（下面的模型预加载）。
            // 失败只让 AI 识别不可用——见 vision::session::init_runtime 的说明。
            #[cfg(feature = "vision")]
            let resource_dir = app_handle.path().resource_dir().ok();
            #[cfg(feature = "vision")]
            {
                // Windows/macOS/Linux: 从 resources 目录加载（Windows 上就是安装目录，
                // VC++ 运行库也在那里）；找不到就交给系统加载器。
                // Android: .so 在 jniLibs 里，按裸文件名 dlopen。
                #[cfg(not(target_os = "android"))]
                let ort_lib_path = resource_dir.as_ref().map(|d| {
                    d.join(if cfg!(target_os = "windows") {
                        "onnxruntime.dll"
                    } else if cfg!(target_os = "macos") {
                        "libonnxruntime.dylib"
                    } else {
                        "libonnxruntime.so"
                    })
                });
                #[cfg(not(target_os = "android"))]
                let ort_lib_path = ort_lib_path.filter(|p| p.exists());
                #[cfg(target_os = "android")]
                let ort_lib_path: Option<std::path::PathBuf> = None;

                // 错误已在 init_runtime 里记日志；之后每次加载模型都会返回同一个错误
                let _ = vision::session::init_runtime(ort_lib_path.as_deref());
            }

            // 内嵌模型释放：从 Tauri 资源目录复制到 AppData。
            // 关掉 vision 时 vision::download 整个模块不存在，这段也就不编译。
            #[cfg(feature = "vision")]
            tauri::async_runtime::block_on(async {
                // 先确保配置文件存在（bootstrap 内部也会调，但复制模型需要先有 registry）
                if let Err(e) = vision::download::ensure_default_files(&app_data_dir).await {
                    log::warn!("[Vision] ensure_default_files failed: {}", e);
                }
                if let Err(e) =
                    vision::download::install_builtin_models(&app_data_dir, resource_dir.as_deref())
                        .await
                {
                    log::warn!("[Vision] install_builtin_models failed: {}", e);
                }
                // v1.1.x 把每张拍照搜索的查询图都存进了 uploads/vision/query，没有任何
                // 记录引用它们，而且顾客端用前置摄像头、多半拍到人脸。现在已不再落盘，
                // 这里把老版本攒下的清掉。
                let query_dir = state.upload_dir.join("vision").join("query");
                if query_dir.exists() {
                    match tokio::fs::remove_dir_all(&query_dir).await {
                        Ok(_) => log::info!("[Vision] removed stale query images: {:?}", query_dir),
                        Err(e) => log::warn!("[Vision] remove {:?} failed: {}", query_dir, e),
                    }
                }
            });

            if let Err(e) =
                tauri::async_runtime::block_on(state.vision_runtime.bootstrap(state.db.clone()))
            {
                log::warn!("[Vision] bootstrap failed: {}", e);
            }

            // 获取后端 URL
            let backend_url = std::env::var("BACKEND_URL")
                .unwrap_or_else(|_| format!("http://127.0.0.1:{http_port}"));

            log::info!("[Config] Backend URL  : {}", backend_url);

            app.manage(backend_url.clone());

            // -------------------------------------------------------------
            // [优化点 2] 使用 Tauri 内置异步运行时
            // -------------------------------------------------------------
            let app_data_dir_for_server = app_data_dir.clone();
            if let Some((http_listener, https_listener)) = listeners {
                tauri::async_runtime::spawn(async move {
                    log::info!(
                        "[Booth Tool] Starting HTTP+HTTPS server ({http_port} loopback / {lan_https_port} LAN)..."
                    );
                    server::start_server(
                        state,
                        http_listener,
                        https_listener,
                        app_data_dir_for_server,
                    )
                    .await;
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
