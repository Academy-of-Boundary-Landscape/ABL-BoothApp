// src/api/info.rs

use crate::{error::ApiResult, server::HTTPS_PORT, state::AppState, utils::ip::get_lan_ip};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

/// server-info 的响应：LAN 访问所需的 IP、端口与各入口 URL。
///
/// 字段按字母序声明：`serde_json` 未开 `preserve_order`，原来的 `json!` 输出
/// 就是这个顺序，这样序列化出来的字节与迁移前一致。
#[derive(Serialize, ToSchema)]
struct ServerInfo {
    /// 管理员入口。
    admin_url: String,
    /// API 根路径。
    api_base_url: String,
    /// LAN 访问的 HTTPS 根 URL。
    base_url: String,
    /// 给 LAN 设备用的 IP。
    ip: String,
    /// 顾客下单入口。
    order_url: String,
    /// HTTPS 端口。
    port: u16,
    /// 摊主入口。
    vendor_url: String,
}

/// 返回本机 LAN 的 IP、端口与各入口 URL，供连接检测与二维码使用。
#[utoipa::path(
    get,
    path = "/server-info",
    tag = "info",
    responses(
        (status = 200, body = ServerInfo, description = "LAN 的 IP、端口与各入口 URL"),
    ),
)]
async fn server_info_handler() -> ApiResult<Json<ServerInfo>> {
    let ip = get_lan_ip();
    let https_port = HTTPS_PORT;

    // 给 LAN 设备的 URL 都用 HTTPS（指向 0.0.0.0:5141 listener），
    // 这样浏览器才会把页面当作 secure context，getUserMedia / clipboard 等 API 才可用。
    // Tauri 客户端自己仍然通过 http://127.0.0.1:5140 访问 API，但那条路径不经过这个 endpoint。
    let base_url = format!("https://{}:{}", ip, https_port);

    let order_url = format!("{}/", base_url);
    let vendor_url = format!("{}/vendor", base_url);
    let admin_url = format!("{}/admin", base_url);
    let api_base_url = format!("{}/api", base_url);

    Ok(Json(ServerInfo {
        admin_url,
        api_base_url,
        base_url,
        ip,
        order_url,
        port: https_port,
        vendor_url,
    }))
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(server_info_handler))
}

/// ③b 形状快照：钉住 server-info 的 JSON 形状（键 + 类型），类型化前后必须一行不改照样绿。
#[cfg(test)]
mod shape_tests {
    use crate::test_support::{json_request, read_json, shape_of, test_router_with};
    use axum::http::StatusCode;
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    /// server-info 不读库；照模块模板统一 `seeded()` 形状，返回 router 与临时目录。
    async fn seeded() -> (Router, tempfile::TempDir) {
        let (router, dir, _pool) = test_router_with().await;
        (router, dir)
    }

    async fn call(
        router: &Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Value,
    ) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(json_request(method, uri, token, body))
            .await
            .unwrap();
        let status = res.status();
        (status, read_json(res).await)
    }

    #[tokio::test]
    async fn shape_server_info_handler() {
        let (router, _dir) = seeded().await;
        let (s, body) = call(&router, "GET", "/api/server-info", None, json!(null)).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(
            shape_of(&body),
            json!({
                "ip": "string",
                "port": "int",
                "base_url": "string",
                "order_url": "string",
                "vendor_url": "string",
                "admin_url": "string",
                "api_base_url": "string"
            })
        );
    }
}
