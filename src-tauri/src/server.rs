use axum::{
    extract::Request,
    http::{header, Method, StatusCode},
    response::IntoResponse,
    Router,
};
use std::path::PathBuf;
use tower::ServiceBuilder;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};

use crate::{api, state::AppState, web};

/// HTTP 的首选端口（仅 127.0.0.1 回环，给 Tauri webview 用）。
pub const HTTP_PORT: u16 = 5140;
/// HTTPS 的首选端口（0.0.0.0，给所有 LAN 设备用）。
pub const HTTPS_PORT: u16 = 5141;

/// HTTP 的候选端口：先 5140，被占就依次试 5150–5159。
///
/// 端口写死的年代，5140 被别的程序占了（2026-09-25 真机上是 VS Code 的端口自动转发）
/// 后端就起不来，而窗口照常打开、所有请求静默失败。实际用的端口经 `get_backend_url`
/// 告诉前端（frontend/src/api/backendOrigin.ts），所以换端口对前端透明。
pub fn http_port_candidates() -> Vec<u16> {
    std::iter::once(HTTP_PORT).chain(5150..=5159).collect()
}

/// HTTPS 的候选端口：先 5141，被占就依次试 5160–5169。实际端口放进
/// `AppState::lan_https_port`，二维码链接用它（api/info.rs）。
/// 注意 Windows 防火墙放行的是具体端口：换了端口，摊主可能要重新放行一次。
pub fn https_port_candidates() -> Vec<u16> {
    std::iter::once(HTTPS_PORT).chain(5160..=5169).collect()
}

/// 依次尝试绑定候选端口，返回第一个成功的 listener 与端口号（已设为非阻塞，可直接交给 tokio）。
pub fn bind_first_available(
    ip: std::net::Ipv4Addr,
    candidates: &[u16],
) -> std::io::Result<(std::net::TcpListener, u16)> {
    let mut last_err = None;
    for &port in candidates {
        match std::net::TcpListener::bind((ip, port)) {
            Ok(l) => {
                l.set_nonblocking(true)?;
                if port != candidates[0] {
                    println!(
                        "[Booth Tool] port {} is in use; using {} instead",
                        candidates[0], port
                    );
                }
                return Ok((l, port));
            }
            Err(e) => last_err = Some(e),
        }
    }
    let tried: Vec<String> = candidates.iter().map(|p| p.to_string()).collect();
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        format!(
            "端口 {} 都被占用了（最后一个错误：{}）",
            tried.join("、"),
            last_err.map(|e| e.to_string()).unwrap_or_default()
        ),
    ))
}

/// 在已经绑定好的两个 listener 上起服务（端口的选择在调用方同步完成，见 `bind_first_available`）。
pub async fn start_server(
    state: AppState,
    http_listener: std::net::TcpListener,
    https_listener: std::net::TcpListener,
    app_data_dir: PathBuf,
) {
    // 复制一份 upload_dir 供 fallback 闭包使用
    let upload_dir = state.upload_dir.clone();

    // 打印一下当前的上传根目录，确保它符合预期
    //println!("[Server Debug] Upload Root Path: {:?}", upload_dir);

    let app = Router::new()
        .nest("/api", api::router().split_for_parts().0) // API 路由
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

    // rustls 0.23 拒绝在没有显式 crypto provider 的情况下工作。
    // 我们没直接拉 rustls 的 provider feature（axum-server 和 sqlx 都拉了 rustls 但都未启用 provider），
    // 所以这里手动安装 ring provider 一次。install_default 是幂等的：已安装会返回 Err，忽略即可。
    let _ = rustls::crypto::ring::default_provider().install_default();

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
    let http_addr = http_listener.local_addr().ok();
    println!("[Booth Tool] HTTP server (loopback only)  http://{http_addr:?}");
    // HTTPS listener: 绑 0.0.0.0，所有 LAN 设备走这里
    let https_addr = https_listener.local_addr().ok();
    println!("[Booth Tool] HTTPS server (LAN)           https://{https_addr:?}");

    let app_for_http = app.clone();
    // 让闭包返回 io::Result：bind/serve 任一失败必须冒泡到 try_join 才能让用户看到，
    // 不能让 HTTPS 端默默死掉而 HTTP 还在跑（那样 QR 码全部指向死端口，摊主完全察觉不到）。
    let http_task: tokio::task::JoinHandle<std::io::Result<()>> = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::from_std(http_listener).map_err(|e| {
            eprintln!("[Booth Tool] FATAL: HTTP listener unusable: {e}");
            e
        })?;
        axum::serve(listener, app_for_http).await.map_err(|e| {
            eprintln!("[Booth Tool] HTTP server crashed: {}", e);
            e
        })?;
        Ok(())
    });

    let https_task: tokio::task::JoinHandle<std::io::Result<()>> = tokio::spawn(async move {
        axum_server::from_tcp_rustls(https_listener, tls_cfg)
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

#[cfg(test)]
mod port_tests {
    use super::bind_first_available;
    use std::net::{Ipv4Addr, TcpListener};

    #[test]
    fn skips_an_occupied_port_and_takes_the_next_candidate() {
        // 先占住一个端口（模拟 VS Code 端口转发 / 另一个程序）
        let squatter = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let taken = squatter.local_addr().unwrap().port();
        let free = {
            let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            probe.local_addr().unwrap().port()
        };

        let (listener, port) = bind_first_available(Ipv4Addr::LOCALHOST, &[taken, free]).unwrap();
        assert_eq!(port, free);
        assert_eq!(listener.local_addr().unwrap().port(), free);
    }

    #[test]
    fn all_candidates_occupied_is_an_error_naming_them() {
        let squatter = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let taken = squatter.local_addr().unwrap().port();
        let err = bind_first_available(Ipv4Addr::LOCALHOST, &[taken]).unwrap_err();
        assert!(err.to_string().contains(&taken.to_string()), "{err}");
    }

    #[test]
    fn candidate_lists_start_with_the_historical_ports_and_do_not_overlap() {
        let http = super::http_port_candidates();
        let https = super::https_port_candidates();
        assert_eq!(http[0], 5140);
        assert_eq!(https[0], 5141);
        assert!(http.iter().all(|p| !https.contains(p)));
    }
}
