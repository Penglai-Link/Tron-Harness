use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{application::task::TaskService, domain::task::TaskListQuery, response::ApiResponse};

use super::{
    error_mapper::task_error_response,
    request::{CreateTaskRequest, UpdateTaskRequest, UpdateTaskStatusRequest},
};

pub async fn options(State(tasks): State<TaskService>) -> Response {
    Json(ApiResponse::success(tasks.options())).into_response()
}

pub async fn kanban(
    State(tasks): State<TaskService>,
    Query(query): Query<TaskListQuery>,
) -> Response {
    match tasks.kanban(query) {
        Ok(columns) => Json(ApiResponse::success(columns)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn list_tasks(
    State(tasks): State<TaskService>,
    Query(query): Query<TaskListQuery>,
) -> Response {
    match tasks.list_tasks(query) {
        Ok(page) => Json(ApiResponse::success(page)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn create_task(
    State(tasks): State<TaskService>,
    Json(request): Json<CreateTaskRequest>,
) -> Response {
    match tasks.create_task(request.into()) {
        Ok(detail) => (StatusCode::CREATED, Json(ApiResponse::success(detail))).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn get_task(State(tasks): State<TaskService>, Path(task_code): Path<String>) -> Response {
    match tasks.get_task(&task_code) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn patch_task(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
    Json(request): Json<UpdateTaskRequest>,
) -> Response {
    match tasks.update_task(&task_code, request.into()) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn patch_status(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
    Json(request): Json<UpdateTaskStatusRequest>,
) -> Response {
    match tasks.update_status(&task_code, request.into()) {
        Ok(detail) => Json(ApiResponse::success(detail)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn archive_task(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
) -> Response {
    match tasks.archive_task(&task_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => task_error_response(error),
    }
}

pub async fn restore_task(
    State(tasks): State<TaskService>,
    Path(task_code): Path<String>,
) -> Response {
    match tasks.restore_task(&task_code) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => task_error_response(error),
    }
}
