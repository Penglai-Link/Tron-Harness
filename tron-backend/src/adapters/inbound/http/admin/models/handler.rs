use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::models::{error_mapper::model_error_response, request::ModelRequest},
    application::model::ModelService,
    response::ApiResponse,
};

pub async fn list_platform_models(State(models): State<ModelService>) -> Response {
    match models.list_platform_models() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn create_platform_model(
    State(models): State<ModelService>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models.create_platform_model(request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn duplicate_platform_model(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
) -> Response {
    match models.duplicate_platform_model(&model_code) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn get_platform_model(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
) -> Response {
    match models.get_platform_model(&model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn patch_platform_model(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
    Json(request): Json<ModelRequest>,
) -> Response {
    match models.update_platform_model(&model_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn verify_platform_model(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
    request: Option<Json<ModelRequest>>,
) -> Response {
    let result = match request {
        Some(Json(request)) => {
            models
                .verify_platform_model_draft(&model_code, request.into())
                .await
        }
        None => models.verify_platform_model(&model_code).await,
    };
    match result {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn enable_platform_model(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
) -> Response {
    match models.enable_platform_model(&model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn disable_platform_model(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
) -> Response {
    match models.disable_platform_model(&model_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn set_platform_default(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
) -> Response {
    match models.set_platform_default(&model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}

pub async fn delete_platform_model(
    State(models): State<ModelService>,
    Path(model_code): Path<String>,
) -> Response {
    match models.delete_platform_model(&model_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => model_error_response(error),
    }
}
