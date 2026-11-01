use crate::app::AppState;
use axum::{Router, routing::get};
pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/model-providers/select-options",
        get(super::handler::list_select_options),
    )
}
