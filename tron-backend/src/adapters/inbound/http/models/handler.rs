use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{application::model::ModelService, response::ApiResponse};

use super::{
    error_mapper::model_error_response,
    request::{ModelRequest, SetAllowInheritanceRequest},
};

pub async fn list_workspace_models(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
) -> Response {
    match models.list_workspace_models(&workspace_code) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn list_select_options(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
) -> Response {
    match models.list_workspace_select_options(&workspace_code) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn create_workspace_model(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models.create_workspace_model(&workspace_code, request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn verify_workspace_model_create_draft(
    State(models): State<ModelService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models
        .verify_workspace_model_create_draft(&workspace_code, request.into())
        .await
    {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn get_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.get_workspace_model(&workspace_code, &model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn patch_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models.update_workspace_model(&workspace_code, &model_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn verify_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
    request: Option<Json<ModelRequest>>,
) -> Response {
    let result = match request {
        Some(Json(request)) => {
            models
                .verify_workspace_model_draft(&workspace_code, &model_code, request.into())
                .await
        }
        None => {
            models
                .verify_workspace_model(&workspace_code, &model_code)
                .await
        }
    };
    match result {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn duplicate_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.duplicate_workspace_model(&workspace_code, &model_code) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn enable_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.enable_workspace_model(&workspace_code, &model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn disable_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.disable_workspace_model(&workspace_code, &model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn set_workspace_default(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.set_workspace_default(&workspace_code, &model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn set_allow_inheritance(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
    Json(request): Json<SetAllowInheritanceRequest>,
) -> Response {
    match models.set_workspace_allow_inheritance(&workspace_code, &model_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn delete_workspace_model(
    State(models): State<ModelService>,
    Path((workspace_code, model_code)): Path<(String, String)>,
) -> Response {
    match models.delete_workspace_model(&workspace_code, &model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}
