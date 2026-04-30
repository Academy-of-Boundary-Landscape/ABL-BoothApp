// src/api/info.rs

use crate::{state::AppState, utils::ip::get_lan_ip};
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<AppState> {
    Router::new().route("/server-info", get(server_info_handler))
}

async fn server_info_handler() -> Json<Value> {
    let ip = get_lan_ip();
    let https_port = 5141;

    // 给 LAN 设备的 URL 都用 HTTPS（指向 0.0.0.0:5141 listener），
    // 这样浏览器才会把页面当作 secure context，getUserMedia / clipboard 等 API 才可用。
    // Tauri 客户端自己仍然通过 http://127.0.0.1:5140 访问 API，但那条路径不经过这个 endpoint。
    let base_url = format!("https://{}:{}", ip, https_port);

    Json(json!({
        "ip": ip,
        "port": https_port,
        "base_url": base_url,
        "order_url": format!("{}/", base_url),
        "vendor_url": format!("{}/vendor", base_url),
        "admin_url": format!("{}/admin", base_url),
        "api_base_url": format!("{}/api", base_url)
    }))
}
