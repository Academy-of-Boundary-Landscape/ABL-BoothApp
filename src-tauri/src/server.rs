use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower::ServiceBuilder;
use tower_http::cors::{AllowHeaders, AllowOrigin, Any, CorsLayer};

use crate::{api, state::AppState, web};

/// HTTP 端口（仅 127.0.0.1 回环，给 Tauri webview 用）。
/// 改这里时 lib.rs 的调用方 自动生效，无需同步修改。
pub const HTTP_PORT: u16 = 5140;
/// HTTPS 端口（0.0.0.0，给所有 LAN 设备用）。
/// 改这里时 api/info.rs 必须同步改：那里的 server_info 端点会把这个端口拼到 QR 码 URL 里。
pub const HTTPS_PORT: u16 = 5141;

pub async fn start_server(state: AppState, http_port: u16, https_port: u16, app_data_dir: PathBuf) {
    // 复制一份 upload_dir 供 fallback 闭包使用
    let upload_dir = state.upload_dir.clone();

    // 打印一下当前的上传根目录，确保它符合预期
    //println!("[Server Debug] Upload Root Path: {:?}", upload_dir);

    let app = Router::new()
        .nest("/api", api::router()) // API 路由
        .fallback(move |req: Request| {
            let uri = req.uri().clone();
            let upload_dir = upload_dir.clone(); // 再次 clone 进入 async 块

            async move {
                let path = uri.path();

                if path.starts_with("/api/") {
                    return (
                        StatusCode::NOT_FOUND,
                        axum::Json(serde_json::json!({"error": "API Route Not Found"})),
                    )
                        .into_response();
                }

                // ---------------------------------------------------------
                // 1. 处理静态资源图片请求 (通过 /uploads/ 路由访问)
                // ---------------------------------------------------------
                if path.starts_with("/uploads/") {
                    // 关键步骤：去掉 URL 前缀 "/uploads/"
                    // 变成 "products/xxx.jpg"
                    let relative_path = path.trim_start_matches("/uploads/");

                    // 解码 URL (处理文件名中的空格、中文等)
                    let decoded_path = urlencoding::decode(relative_path)
                        .unwrap_or(std::borrow::Cow::Borrowed(relative_path));

                    // 拼接物理路径
                    let file_path = upload_dir.join(decoded_path.as_ref());

                    // 安全检查：防止路径遍历攻击 (如 ../../etc/passwd)
                    // 规范化路径后确认仍在 upload_dir 内
                    let canonical_upload = match upload_dir.canonicalize() {
                        Ok(p) => p,
                        Err(_) => {
                            return (StatusCode::INTERNAL_SERVER_ERROR, "Upload dir error")
                                .into_response();
                        }
                    };
                    let canonical_file = match file_path.canonicalize() {
                        Ok(p) => p,
                        Err(_) => {
                            // 文件不存在或路径无效
                            return (StatusCode::NOT_FOUND, "Image Not Found").into_response();
                        }
                    };
                    if !canonical_file.starts_with(&canonical_upload) {
                        return (StatusCode::FORBIDDEN, "Access denied").into_response();
                    }

                    if file_path.exists() && file_path.is_file() {
                        match tokio::fs::read(&file_path).await {
                            Ok(bytes) => {
                                // 自动猜测 MIME 类型 (image/jpeg, image/png)
                                let mime =
                                    mime_guess::from_path(&file_path).first_or_octet_stream();
                                return ([(header::CONTENT_TYPE, mime.as_ref())], bytes)
                                    .into_response();
                            }
                            Err(e) => {
                                println!("[Static Error] Read failed: {}", e);
                                return (StatusCode::INTERNAL_SERVER_ERROR, "File Read Error")
                                    .into_response();
                            }
                        }
                    } else {
                        println!("[Static Error] File not found: {:?}", file_path);
                        return (StatusCode::NOT_FOUND, "Image Not Found").into_response();
                    }
                }

                // ---------------------------------------------------------
                // 2. 处理前端 Vue 页面 (嵌入资源)
                // ---------------------------------------------------------
                web::static_file_handler(uri, upload_dir).await
            }
        })
        .layer(
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_origin(AllowOrigin::mirror_request())
                    .allow_methods([
                        Method::GET,
                        Method::POST,
                        Method::PUT,
                        Method::DELETE,
                        Method::PATCH,
                        Method::OPTIONS,
                    ])
                    .allow_headers(AllowHeaders::list(vec![
                        header::AUTHORIZATION,                               // 用于 Bearer Token
                        header::CONTENT_TYPE, // 用于 application/json
                        header::ACCEPT,       // Axios 发送的 Accept 头
                        header::COOKIE,       // 用于 Cookie 传输
                        header::HeaderName::from_static("x-requested-with"), // 某些 WebView 会带
                        header::HeaderName::from_static("x-custom-header"), // 如果你有自定义头，加在这里
                    ]))
                    .allow_credentials(true),
            ),
        )
        .with_state(state);

    // 准备证书
    let lan_ips = crate::utils::ip::get_all_lan_ipv4_addrs();
    let (cert_pem, key_pem) =
        match crate::utils::cert::load_or_generate_cert(&app_data_dir, &lan_ips).await {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("[Booth Tool] FATAL: cert load/generate failed: {}", e);
                return;
            }
        };
    let tls_cfg = match axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Booth Tool] FATAL: RustlsConfig::from_pem failed: {}", e);
            return;
        }
    };

    // HTTP listener: 仅绑回环，给 Tauri webview 用，避免 LAN 接触 HTTP
    let http_addr = SocketAddr::from(([127, 0, 0, 1], http_port));
    println!("[Booth Tool] HTTP server (loopback only)  http://{}", http_addr);

    // HTTPS listener: 绑 0.0.0.0，所有 LAN 设备走这里
    let https_addr = SocketAddr::from(([0, 0, 0, 0], https_port));
    println!("[Booth Tool] HTTPS server (LAN)           https://{}", https_addr);

    let app_for_http = app.clone();
    // 让闭包返回 io::Result：bind/serve 任一失败必须冒泡到 try_join 才能让用户看到，
    // 不能让 HTTPS 端默默死掉而 HTTP 还在跑（那样 QR 码全部指向死端口，摊主完全察觉不到）。
    let http_task: tokio::task::JoinHandle<std::io::Result<()>> =
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(http_addr).await.map_err(|e| {
                eprintln!("[Booth Tool] FATAL: bind {} failed: {}", http_addr, e);
                e
            })?;
            axum::serve(listener, app_for_http).await.map_err(|e| {
                eprintln!("[Booth Tool] HTTP server crashed: {}", e);
                e
            })?;
            Ok(())
        });

    let https_task: tokio::task::JoinHandle<std::io::Result<()>> = tokio::spawn(async move {
        axum_server::bind_rustls(https_addr, tls_cfg)
            .serve(app.into_make_service())
            .await
            .map_err(|e| {
                eprintln!("[Booth Tool] HTTPS server crashed: {}", e);
                e
            })?;
        Ok(())
    });

    // 任一 task 出错或 panic → 整体退出，让外层 tauri 进程能感知到
    match tokio::try_join!(http_task, https_task) {
        Ok((http_res, https_res)) => {
            if let Err(e) = http_res {
                eprintln!("[Booth Tool] HTTP listener exited with error: {}", e);
            }
            if let Err(e) = https_res {
                eprintln!("[Booth Tool] HTTPS listener exited with error: {}", e);
            }
        }
        Err(join_err) => {
            eprintln!("[Booth Tool] server task panicked: {}", join_err);
        }
    }
}
