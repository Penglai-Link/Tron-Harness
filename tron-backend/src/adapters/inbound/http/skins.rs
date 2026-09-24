mod error_mapper;
mod handler;
mod request;
mod response;

use crate::app::AppState;
use axum::{Router, extract::DefaultBodyLimit, routing::get};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/system/skins",
            get(handler::list).post(handler::install),
        )
        .route(
            "/api/v1/system/skins/active",
            get(handler::active).put(handler::activate),
        )
        .route("/api/v1/system/skins/events", get(handler::events))
        .route(
            "/api/v1/system/skins/{id}/resources/{*path}",
            get(handler::resource),
        )
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024))
}
