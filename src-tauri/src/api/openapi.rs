//! OpenAPI 文档的公共部分与契约快照测试。
//!
//! 路径与 schema 由各模块的 `routes!(...)` 自动收集，这里只放全局的东西：
//! 文档元信息、bearer 鉴权方案、所有错误响应共用的 `ApiErrorBody`。

use serde::Serialize;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

/// security scheme 名。handler 注解里写 `security((BEARER = []))`。
pub const BEARER: &str = "bearer";

/// 所有错误响应的形状。为什么必须是这个形状，见 `crate::error` 的模块注释。
#[derive(Serialize, ToSchema)]
pub struct ApiErrorBody {
    pub error: String,
}

/// `info.version` 故意固定为 "1" 而不是 App 版本号：否则每次 `set-version.sh`
/// 都会让契约快照变红，而契约本身并没有变。
#[derive(OpenApi)]
#[openapi(
    info(title = "摊盒 Booth-Kernel API", version = "1"),
    components(schemas(ApiErrorBody)),
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

#[cfg(test)]
mod tests {
    /// 契约快照：`openapi.json` 必须与当前代码生成的文档逐字节一致。
    ///
    /// 只在 `--all-features` 下断言——文档按含 vision 的路由集生成，关掉 vision
    /// 时路由少了，但快照不该跟着变。
    #[test]
    #[cfg(feature = "vision")]
    fn openapi_snapshot() {
        let doc = crate::api::router().into_openapi();
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
