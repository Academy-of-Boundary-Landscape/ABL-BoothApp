use crate::state::AppState;
use axum::Router;

mod admin;
mod auth;
mod event;
pub mod guard;
mod info;
mod master_product;
mod order;
mod product;
mod stats;
mod sync;
#[cfg(feature = "vision")]
mod vision;

pub fn router() -> Router<AppState> {
    let router = Router::new()
        .nest("/auth", auth::router())
        .nest("/events", event::router())
        .nest("/events", stats::router())
        .nest("/master-products", master_product::router())
        .nest("/admin", admin::router())
        .merge(sync::router())
        .merge(info::router())
        .merge(product::router())
        .merge(order::router());

    #[cfg(feature = "vision")]
    let router = router.nest("/vision", vision::router());

    router
}
