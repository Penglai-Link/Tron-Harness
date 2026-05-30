use axum::{Router, routing::post};

use crate::app::AppState;

use super::handler::ingest;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/rag/ingest", post(ingest))
}
