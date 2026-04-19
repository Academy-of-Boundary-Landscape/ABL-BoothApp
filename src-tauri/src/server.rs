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

pub async fn start_server(state: AppState, port: u16) {
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

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("[Booth Tool] HTTP Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind server address");

    axum::serve(listener, app).await.expect("Server crashed");
}
