use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    application::app::AppService,
    domain::app::{AppListQuery, AppOptionQuery},
    response::ApiResponse,
};

use super::{
    error_mapper::app_error_response,
    request::{CreateAppRequest, UpdateAppRequest},
};

pub async fn list_apps(
    State(apps): State<AppService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<AppListQuery>,
) -> Response {
    match apps.list_apps(&workspace_code, query) {
        Ok(page) => Json(ApiResponse::success(page)).into_response(),
        Err(error) => app_error_response(error),
    }
}

pub async fn create_app(
    State(apps): State<AppService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<CreateAppRequest>,
) -> Response {
    match apps.create_app(&workspace_code, request.into()) {
        Ok(detail) => (StatusCode::CREATED, Json(ApiResponse::success(detail))).into_response(),
        Err(error) => app_error_response(error),
    }
}

pub async fn list_app_options(
    State(apps): State<AppService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<AppOptionQuery>,
) -> Response {
    match apps.list_app_options(&workspace_code, query) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => app_error_response(error),
    }
}

pub async fn get_app(State(apps): State<AppService>, Path(app_code): Path<String>) -> Response {
    match apps.get_app(&app_code) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => app_error_response(error),
    }
}

pub async fn patch_app(
    State(apps): State<AppService>,
    Path(app_code): Path<String>,
    Json(request): Json<UpdateAppRequest>,
) -> Response {
    match apps.update_app(&app_code, request.into()) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => app_error_response(error),
    }
}

pub async fn archive_app(State(apps): State<AppService>, Path(app_code): Path<String>) -> Response {
    match apps.archive_app(&app_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => app_error_response(error),
    }
}

pub async fn restore_app(State(apps): State<AppService>, Path(app_code): Path<String>) -> Response {
    match apps.restore_app(&app_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => app_error_response(error),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteAppResponse {
    app_code: String,
}

pub async fn delete_app(State(apps): State<AppService>, Path(app_code): Path<String>) -> Response {
    match apps.delete_app(&app_code) {
        Ok(()) => Json(ApiResponse::success(DeleteAppResponse { app_code })).into_response(),
        Err(error) => app_error_response(error),
    }
}
