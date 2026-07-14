use axum::{Router, routing::get};

use crate::app::AppState;

use super::handler::{get_system_config, update_system_config};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/system/config",
        get(get_system_config).put(update_system_config),
    )
}
