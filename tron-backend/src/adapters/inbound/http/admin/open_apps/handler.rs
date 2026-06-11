use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{application::open_app::OpenAppService, response::ApiResponse};

use super::{
    error_mapper::open_app_error_response,
    request::{
        CreateOpenAppRequest, ReplaceCapabilityGrantsRequest, ReplaceUserGrantsRequest,
        UpdateOpenAppRequest,
    },
};

pub async fn list_platform_open_apps(State(open_apps): State<OpenAppService>) -> Response {
    match open_apps.list_open_apps() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => open_app_error_response(error),
    }
}

pub async fn create_platform_open_app(
    State(open_apps): State<OpenAppService>,
    Json(request): Json<CreateOpenAppRequest>,
) -> Response {
    match open_apps.create_open_app_with_secret_reveal(request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => open_app_error_response(error),
    }
}

pub async fn list_platform_open_app_catalog(State(open_apps): State<OpenAppService>) -> Response {
    Json(ApiResponse::success(open_apps.list_capability_catalog())).into_response()
}

pub async fn get_platform_open_app(
    State(open_apps): State<OpenAppService>,
    Path(open_app_code): Path<String>,
) -> Response {
    match open_apps.get_open_app_detail(&open_app_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => open_app_error_response(error),
    }
}

pub async fn patch_platform_open_app(
    State(open_apps): State<OpenAppService>,
    Path(open_app_code): Path<String>,
    Json(request): Json<UpdateOpenAppRequest>,
) -> Response {
    match open_apps.update_open_app(&open_app_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => open_app_error_response(error),
    }
}

pub async fn rotate_platform_open_app_key(
    State(open_apps): State<OpenAppService>,
    Path(open_app_code): Path<String>,
) -> Response {
    match open_apps.rotate_open_app_key_with_secret_reveal(&open_app_code) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => open_app_error_response(error),
    }
}

pub async fn put_platform_open_app_user_grants(
    State(open_apps): State<OpenAppService>,
    Path(open_app_code): Path<String>,
    Json(request): Json<ReplaceUserGrantsRequest>,
) -> Response {
    match open_apps.replace_user_grants(&open_app_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => open_app_error_response(error),
    }
}

pub async fn put_platform_open_app_capability_grants(
    State(open_apps): State<OpenAppService>,
    Path(open_app_code): Path<String>,
    Json(request): Json<ReplaceCapabilityGrantsRequest>,
) -> Response {
    match open_apps.replace_capability_grants(&open_app_code, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => open_app_error_response(error),
    }
}
