use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use serde::Serialize;

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::{user::StepUpVariableRevealTarget, variable::VariableService},
    domain::variable::VariableCatalogQuery,
    response::ApiResponse,
};

use super::{
    error_mapper::variable_error_response,
    request::{CreateVariableRequest, RevealWorkspaceVariableRequest, UpdateVariableRequest},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RevealVariableValueDto {
    value: String,
}

pub async fn list_variables(
    State(variables): State<VariableService>,
    Path(workspace_code): Path<String>,
    Query(query): Query<VariableCatalogQuery>,
) -> Response {
    match variables.list_variables(&workspace_code, query) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => variable_error_response(error),
    }
}

pub async fn create_variable(
    State(variables): State<VariableService>,
    Path(workspace_code): Path<String>,
    Json(request): Json<CreateVariableRequest>,
) -> Response {
    match variables.create_variable(&workspace_code, request.into()) {
        Ok(record) => (StatusCode::CREATED, Json(ApiResponse::success(record))).into_response(),
        Err(error) => variable_error_response(error),
    }
}

pub async fn patch_variable(
    State(variables): State<VariableService>,
    Path((workspace_code, variable_key)): Path<(String, String)>,
    Json(request): Json<UpdateVariableRequest>,
) -> Response {
    match variables.update_variable(&workspace_code, &variable_key, request.into()) {
        Ok(record) => Json(ApiResponse::success(record)).into_response(),
        Err(error) => variable_error_response(error),
    }
}

pub async fn delete_variable(
    State(variables): State<VariableService>,
    Path((workspace_code, variable_key)): Path<(String, String)>,
) -> Response {
    match variables.delete_variable(&workspace_code, &variable_key) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => variable_error_response(error),
    }
}

pub async fn reveal_workspace_variable(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<RevealWorkspaceVariableRequest>,
) -> Response {
    let step_up_target =
        StepUpVariableRevealTarget::workspace(&request.workspace_code, &request.variable_key);
    if let Err(error) = state.users.verify_step_up_token(
        &auth.user().user_code,
        &request.step_up_token,
        &step_up_target,
    ) {
        return crate::adapters::inbound::http::auth::error_mapper::auth_error_response(error);
    }
    match state
        .variables
        .reveal_workspace_variable(&request.workspace_code, &request.variable_key)
    {
        Ok(value) => Json(ApiResponse::success(RevealVariableValueDto { value })).into_response(),
        Err(error) => variable_error_response(error),
    }
}
