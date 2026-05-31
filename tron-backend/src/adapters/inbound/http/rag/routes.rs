use axum::{Router, routing::post};

use crate::app::AppState;

use super::handler::{ingest, search};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/rag/ingest", post(ingest))
        .route("/api/v1/rag/search", post(search))
}
