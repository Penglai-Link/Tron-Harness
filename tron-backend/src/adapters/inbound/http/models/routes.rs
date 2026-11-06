use crate::app::AppState;
use axum::{Router, routing::get};
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/model-providers/select-options",
            get(super::handler::list_select_options),
        )
        .route(
            "/api/v1/models/speech-transcriptions",
            axum::routing::post(super::speech::transcribe)
                .layer(axum::extract::DefaultBodyLimit::max(12 * 1024 * 1024)),
        )
        .route(
            "/api/v1/models/speech-transcriptions/{request_id}",
            axum::routing::delete(super::speech::cancel),
        )
}
