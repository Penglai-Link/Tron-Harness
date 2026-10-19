//! Ownership-authorized graph APIs. Rust/FE consensus §§2.1,2.4–2.6.
mod error;
mod request;
mod response;
use crate::{
    adapters::inbound::http::auth::RequestAuthContext, app::AppState, response::ApiResponse,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    routing::post,
};
use error::GraphApiError;
use request::*;
use response::*;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/rag/graph/browse", post(browse))
        .route("/api/v1/rag/graph/search", post(search))
        .route("/api/v1/rag/graph/expand", post(expand))
        .layer(DefaultBodyLimit::max(64 * 1024))
}
async fn search(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    payload: Result<Json<GraphSearchRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<GraphSearchView>>, GraphApiError> {
    let input = payload
        .map_err(|_| GraphApiError::invalid(&auth.locale))?
        .0
        .into_input();
    let output = state
        .knowledge_bases
        .graph_search_for_user(&auth.user().user_code, input)
        .await
        .map_err(|error| GraphApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(output.into())))
}
async fn expand(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    payload: Result<Json<GraphExpandRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<GraphExpandView>>, GraphApiError> {
    let input = payload
        .map_err(|_| GraphApiError::invalid(&auth.locale))?
        .0
        .into_input();
    let output = state
        .knowledge_bases
        .graph_expand_for_user(&auth.user().user_code, input)
        .await
        .map_err(|error| GraphApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(output.into())))
}

async fn browse(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    payload: Result<Json<GraphBrowseRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<GraphBrowseView>>, GraphApiError> {
    let input = payload
        .map_err(|_| GraphApiError::invalid(&auth.locale))?
        .0
        .into_input();
    let output = state
        .knowledge_bases
        .graph_browse_for_user(&auth.user().user_code, input)
        .await
        .map_err(|error| GraphApiError(error, auth.locale.clone()))?;
    Ok(Json(ApiResponse::protocol_success(output.into())))
}
