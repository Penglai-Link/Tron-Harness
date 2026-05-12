use axum::{Router, routing::get};

use crate::{config::AppConfig, health::health};

pub fn build_router(config: AppConfig) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .with_state(config)
}
