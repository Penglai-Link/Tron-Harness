use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{debug_profile, debug_source, ingest, retry_index, search};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/rag/ingest", post(ingest))
        .route("/api/v1/rag/search", post(search))
        .route("/api/v1/rag/debug/profile", get(debug_profile))
        .route("/api/v1/rag/debug/source", get(debug_source))
        .route("/api/v1/rag/retry/index", post(retry_index))
}
