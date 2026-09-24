use crate::state::AppState;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

mod admin;
mod auth;
mod closing;
mod event;
pub mod guard;
mod info;
mod inventory;
mod legacy;
mod lot;
mod master_product;
pub mod openapi;
mod order;
mod product;
mod refund;
mod settlement;
mod society;
mod stats;
mod sync;
#[cfg(feature = "vision")]
mod vision;

/// 全部 API 路由，连同它们的 OpenAPI 文档。
///
/// 调用方用 `.split_for_parts().0`（或 `Router::from`）取 axum 的 `Router`；
/// 文档只在契约快照测试里用（`api/openapi.rs`），运行时不对外暴露。
pub fn router() -> OpenApiRouter<AppState> {
    let router = OpenApiRouter::with_openapi(openapi::ApiDoc::openapi())
        .nest("/auth", auth::router())
        .nest("/events", event::router())
        .nest("/events", stats::router())
        .nest("/master-products", master_product::router())
        .nest("/societies", society::router())
        .nest("/admin", admin::router())
        .merge(sync::router())
        .merge(info::router())
        .merge(product::router())
        .merge(lot::router())
        .merge(order::router())
        .merge(refund::router())
        .merge(closing::router())
        .merge(settlement::router())
        .merge(inventory::router())
        .nest("/legacy", legacy::router());

    #[cfg(feature = "vision")]
    let router = router.nest("/vision", vision::router());

    router
}
