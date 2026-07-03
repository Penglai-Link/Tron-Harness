use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;

use super::handler::{expand, list_namespaces, query, snapshot};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/knowledge-graph", get(snapshot))
        .route(
            "/api/v1/knowledge-graph/list-namespaces",
            get(list_namespaces),
        )
        .route("/api/v1/knowledge-graph/query", post(query))
        .route("/api/v1/knowledge-graph/expand", post(expand))
}
