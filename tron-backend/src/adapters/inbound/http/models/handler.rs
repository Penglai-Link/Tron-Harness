use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};

use crate::{
    application::model::ModelService,
    domain::model::WorkspaceModelQuery,
    response::ApiResponse,
};

use super::error_mapper::model_error_response;

pub async fn list_models(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<WorkspaceModelQuery>,
) -> Response {
    match models.list_models(&workspace_code, query) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn remove_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.remove_model(&workspace_code, &model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn restore_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.restore_model(&workspace_code, &model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}
