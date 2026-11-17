mod handler;
mod request;
mod response;
pub fn routes() -> axum::Router<crate::app::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/v1/skills/candidates", get(handler::list))
        .route(
            "/api/v1/skills/candidates/{candidate_code}",
            get(handler::detail),
        )
        .route(
            "/api/v1/skills/candidates/{candidate_code}/approve",
            post(handler::approve),
        )
        .route(
            "/api/v1/skills/candidates/{candidate_code}/reject",
            post(handler::reject),
        )
        .route(
            "/api/v1/skills/candidates/{candidate_code}/revise",
            post(handler::revise),
        )
        .route(
            "/api/v1/skills/candidates/{candidate_code}/retry-install",
            post(handler::retry_install),
        )
        .route(
            "/api/v1/assistants/{assistant_code}/memory-batches",
            get(handler::batches),
        )
}
