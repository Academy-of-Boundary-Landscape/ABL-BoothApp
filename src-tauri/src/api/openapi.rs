//! OpenAPI 文档的公共部分与契约快照测试。
//!
//! 路径与 schema 由各模块的 `routes!(...)` 自动收集，这里只放全局的东西：
//! 文档元信息、bearer 鉴权方案、所有错误响应共用的 `ApiErrorBody`。

use serde::Serialize;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

/// security scheme 名。handler 注解里必须写字面量 `security(("bearer" = []))`
/// （utoipa 宏不接受常量），改名时两处一起改。
pub const BEARER: &str = "bearer";

/// 所有错误响应的形状。为什么必须是这个形状，见 `crate::error` 的模块注释。
#[derive(Serialize, ToSchema)]
pub struct ApiErrorBody {
    pub error: String,
}

// ---- 值域固定的字符串：只用于文档 ----
//
// 这些字段在 Rust 里仍是 `String`（改成真枚举会动到 SQL 读写与校验逻辑，③b 不做），
// 在文档里声明成枚举，让前端生成的类型是字面量联合。字段上用
// `#[schema(value_type = EventStatus)]` 引用。取值必须与校验代码一致：
// `event.rs` 的 update_status、`order.rs` 的 update_order_status、`refund.rs` 的去向、
// `settlement.rs` 的调整方向。

/// 展会状态。
#[derive(Serialize, ToSchema)]
#[allow(dead_code)]
pub enum EventStatus {
    #[serde(rename = "筹备")]
    Preparing,
    #[serde(rename = "进行中")]
    Ongoing,
    #[serde(rename = "已结算")]
    Settled,
}

/// 订单状态。
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum OrderStatus {
    Pending,
    Completed,
    Cancelled,
}

/// 退货的货去哪：`现场仓`（还能卖）或 `损耗`（已损坏）。
#[derive(Serialize, ToSchema)]
#[allow(dead_code)]
pub enum RefundDestination {
    #[serde(rename = "现场仓")]
    OnSite,
    #[serde(rename = "损耗")]
    Loss,
}

/// 结算调整方向：`to_them` = 我要多给他们；`to_me` = 他们要多给我。
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum AdjustmentDirection {
    ToThem,
    ToMe,
}

/// `info.version` 故意固定为 "1" 而不是 App 版本号：否则每次 `set-version.sh`
/// 都会让契约快照变红，而契约本身并没有变。
#[derive(OpenApi)]
#[openapi(
    info(title = "摊盒 Booth-Kernel API", version = "1"),
    components(schemas(
        ApiErrorBody,
        EventStatus,
        OrderStatus,
        RefundDestination,
        AdjustmentDirection
    )),
    modifiers(&BearerScheme)
)]
pub struct ApiDoc;

struct BearerScheme;

impl Modify for BearerScheme {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            BEARER,
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

/// 完整的 API 文档：路由收集到的路径 + 全局部分，operationId 改写成全局唯一。
///
/// utoipa 默认拿 handler 名当 operationId，但不同模块会有同名 handler
/// （`update_product`、`update_status`），一个 handler 也可能同时挂 POST 和 PUT
/// （`method(post, put)` 会让两个操作共用一个 id）。openapi-typescript 遇到重复 id 直接拒绝生成。
///
/// 改写规则：`<tag>.<handler>`；同一 handler 挂了多个方法时再加 `.<method>`。
/// `scripts/check-event-guards.py` 靠第二段认 handler，改规则时两边一起改。
///
/// 只有契约快照测试用它：运行时不对外暴露文档。
#[cfg(test)]
pub fn document() -> utoipa::openapi::OpenApi {
    use std::collections::HashMap;
    use utoipa::openapi::path::Operation;

    let mut doc = crate::api::router().into_openapi();

    fn ops(item: &mut utoipa::openapi::PathItem) -> [(&'static str, &mut Option<Operation>); 5] {
        [
            ("get", &mut item.get),
            ("post", &mut item.post),
            ("put", &mut item.put),
            ("patch", &mut item.patch),
            ("delete", &mut item.delete),
        ]
    }
    let key = |op: &Operation| {
        let tag = op
            .tags
            .as_ref()
            .and_then(|t| t.first())
            .cloned()
            .unwrap_or_default();
        let handler = op.operation_id.clone().unwrap_or_default();
        format!("{tag}.{handler}")
    };

    let mut counts: HashMap<String, usize> = HashMap::new();
    for item in doc.paths.paths.values_mut() {
        for (_, op) in ops(item) {
            if let Some(op) = op {
                *counts.entry(key(op)).or_default() += 1;
            }
        }
    }
    for item in doc.paths.paths.values_mut() {
        for (method, op) in ops(item) {
            if let Some(op) = op {
                let base = key(op);
                op.operation_id = Some(if counts[&base] > 1 {
                    format!("{base}.{method}")
                } else {
                    base
                });
            }
        }
    }
    doc
}

#[cfg(test)]
mod tests {
    /// 契约快照：`openapi.json` 必须与当前代码生成的文档逐字节一致。
    ///
    /// 只在 `--all-features` 下断言——文档按含 vision 的路由集生成，关掉 vision
    /// 时路由少了，但快照不该跟着变。
    /// openapi-typescript 要求 operationId 全局唯一。utoipa 默认用 handler 名，
    /// 而不同模块会有同名 handler（`update_product`），一个 handler 也可能同时挂
    /// POST 和 PUT（`method(post, put)`）。
    #[test]
    fn operation_ids_are_unique_and_keep_the_handler_name() {
        let doc = super::document();
        let mut seen = std::collections::HashSet::new();
        let mut count = 0;
        for (path, item) in doc.paths.paths.iter() {
            let v = serde_json::to_value(item).unwrap();
            for (method, op) in v.as_object().unwrap() {
                let Some(id) = op.get("operationId").and_then(|x| x.as_str()) else {
                    continue;
                };
                count += 1;
                assert!(
                    seen.insert(id.to_string()),
                    "重复的 operationId {id}（{method} {path}）"
                );
                // 形如 `<tag>.<handler>` 或 `<tag>.<handler>.<method>`：守卫门禁靠第二段认 handler
                let parts: Vec<&str> = id.split('.').collect();
                assert!(parts.len() == 2 || parts.len() == 3, "{id}");
            }
        }
        assert!(count > 0);
    }

    /// 值域固定的字符串字段在文档里是枚举：TS 侧得到字面量联合类型，拼错状态在编译期报错。
    /// 线上 JSON 不变——只是文档层的声明（shape-cleanups.md 的 D 类）。
    #[test]
    fn fixed_value_strings_are_documented_as_enums() {
        let doc = serde_json::to_value(super::document()).unwrap();
        let schemas = &doc["components"]["schemas"];
        let enum_of = |name: &str| -> Vec<String> {
            serde_json::from_value(schemas[name]["enum"].clone())
                .unwrap_or_else(|_| panic!("{name} 不是枚举：{}", schemas[name]))
        };
        assert_eq!(enum_of("EventStatus"), ["筹备", "进行中", "已结算"]);
        assert_eq!(
            enum_of("OrderStatus"),
            ["pending", "completed", "cancelled"]
        );
        assert_eq!(enum_of("RefundDestination"), ["现场仓", "损耗"]);
        assert_eq!(enum_of("AdjustmentDirection"), ["to_them", "to_me"]);

        // 列表接口的 status 查询参数
        for path in ["/events", "/events/{event_id}/orders"] {
            let params = doc["paths"][path]["get"]["parameters"].as_array().unwrap();
            let status = params.iter().find(|p| p["name"] == "status").unwrap();
            assert!(
                status["schema"].to_string().contains("Status"),
                "{path}: {status}"
            );
        }

        let field_ref = |schema: &str, field: &str| {
            schemas[schema]["properties"][field]["$ref"]
                .as_str()
                .unwrap_or_else(|| {
                    panic!(
                        "{schema}.{field} 没有引用枚举：{}",
                        schemas[schema]["properties"][field]
                    )
                })
                .rsplit('/')
                .next()
                .unwrap()
                .to_string()
        };
        for (schema, field, e) in [
            ("EventResponse", "status", "EventStatus"),
            ("Event", "status", "EventStatus"),
            ("ClosingState", "status", "EventStatus"),
            ("SettleResponse", "status", "EventStatus"),
            ("EventUpdateStatusRequest", "status", "EventStatus"),
            ("OrderRow", "status", "OrderStatus"),
            ("OrderUpdateStatusRequest", "status", "OrderStatus"),
            ("RefundLineRequest", "destination", "RefundDestination"),
            ("RefundHistoryRow", "destination", "RefundDestination"),
            ("AdjustmentRequest", "direction", "AdjustmentDirection"),
        ] {
            assert_eq!(field_ref(schema, field), e, "{schema}.{field}");
        }
    }

    #[test]
    #[cfg(feature = "vision")]
    fn openapi_snapshot() {
        let doc = super::document();
        let actual = serde_json::to_string_pretty(&doc).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
        if std::env::var_os("UPDATE_OPENAPI").is_some() {
            std::fs::write(path, &actual).unwrap();
            return;
        }
        let expected = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            actual == expected,
            "openapi.json 与代码不一致。确认契约变更是有意的，然后跑：\n  \
             UPDATE_OPENAPI=1 tauri-env linux cargo test --all-features openapi_snapshot"
        );
    }

    #[test]
    #[cfg(not(feature = "vision"))]
    fn openapi_snapshot() {
        eprintln!(
            "openapi_snapshot 只在 --all-features 下断言：openapi.json 按含 vision 的路由集生成"
        );
    }
}
