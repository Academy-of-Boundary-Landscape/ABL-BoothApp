use crate::state::AppState;
use axum::Router;

mod admin;
mod auth;
mod event;
pub mod guard;
mod info;
mod inventory;
mod legacy;
mod lot;
mod master_product;
mod order;
mod product;
mod refund;
mod settlement;
mod society;
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
        .nest("/societies", society::router())
        .nest("/admin", admin::router())
        .merge(sync::router())
        .merge(info::router())
        .merge(product::router())
        .merge(lot::router())
        .merge(order::router())
        .merge(refund::router())
        .merge(settlement::router())
        .merge(inventory::router())
        .nest("/legacy", legacy::router());

    #[cfg(feature = "vision")]
    let router = router.nest("/vision", vision::router());

    router
}
