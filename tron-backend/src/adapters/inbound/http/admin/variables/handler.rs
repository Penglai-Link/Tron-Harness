use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::variables::{
        error_mapper::variable_error_response,
        request::{CreateVariableRequest, UpdateVariableRequest},
    },
    application::variable::VariableService,
    domain::variable::VariableError,
    response::{ApiResponse, EmptyData},
};

fn platform_variable_error_response(error: VariableError) -> Response {
    if matches!(error, VariableError::DuplicateKey) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<EmptyData>::error(
                "VARIABLE_KEY_DUPLICATE",
                "platform variable key already exists",
            )),
        )
            .into_response();
    }

    variable_error_response(error)
}

pub async fn list_platform_variables(State(variables): State<VariableService>) -> Response {
    match variables.list_platform_variables() {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => platform_variable_error_response(error),
    }
}

pub async fn create_platform_variable(
    State(variables): State<VariableService>,
    Json(request): Json<CreateVariableRequest>,
) -> Response {
    match variables.create_platform_variable(request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => platform_variable_error_response(error),
    }
}

pub async fn patch_platform_variable(
    State(variables): State<VariableService>,
    Path(variable_key): Path<String>,
    Json(request): Json<UpdateVariableRequest>,
) -> Response {
    match variables.update_platform_variable(&variable_key, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => platform_variable_error_response(error),
    }
}

pub async fn delete_platform_variable(
    State(variables): State<VariableService>,
    Path(variable_key): Path<String>,
) -> Response {
    match variables.delete_platform_variable(&variable_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => platform_variable_error_response(error),
    }
}
