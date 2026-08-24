use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use serde::{Deserialize, Serialize};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::{
        user::StepUpVariableRevealTarget,
        variable::{PromptVariableOption, VariableService},
    },
    domain::{
        prompt::{
            PromptRuntimeProvider, PromptVariableSourceKind, PromptVariableType,
            registered_runtime_prompt_variables,
        },
        variable::VariableCatalogQuery,
    },
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptVariableOptionResponse {
    id: String,
    key: String,
    #[serde(rename = "type")]
    variable_type: PromptVariableType,
    description: Option<String>,
    source_kind: PromptVariableSourceKind,
    source_label: String,
    managed_variable_key: Option<String>,
    runtime_provider: Option<PromptRuntimeProvider>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptVariableOptionsQuery {
    pub search: Option<String>,
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

pub async fn list_workspace_prompt_variable_options(
    State(state): State<AppState>,
    Path(workspace_code): Path<String>,
    Query(query): Query<PromptVariableOptionsQuery>,
) -> Response {
    match state.variables.list_workspace_prompt_variable_options(
        &workspace_code,
        VariableCatalogQuery {
            search: query.search.clone(),
        },
    ) {
        Ok(records) => Json(ApiResponse::success(prompt_variable_options(
            records,
            query.search.as_deref(),
        )))
        .into_response(),
        Err(error) => variable_error_response(error),
    }
}

pub(crate) fn prompt_variable_options(
    managed: Vec<PromptVariableOption>,
    search: Option<&str>,
) -> Vec<PromptVariableOptionResponse> {
    let normalized_search = search.map(str::trim).filter(|value| !value.is_empty());
    let mut options = managed
        .into_iter()
        .map(|variable| PromptVariableOptionResponse {
            id: format!("MANAGED:{}", variable.key),
            key: variable.key.clone(),
            variable_type: PromptVariableType::String,
            description: variable.description,
            source_kind: PromptVariableSourceKind::Managed,
            source_label: variable.source_label,
            managed_variable_key: Some(variable.key),
            runtime_provider: None,
        })
        .collect::<Vec<_>>();
    options.extend(
        registered_runtime_prompt_variables()
            .filter(|variable| matches_prompt_variable_option(variable, normalized_search))
            .map(|variable| PromptVariableOptionResponse {
                id: format!("RUNTIME:{}:{}", variable.runtime_provider.code(), variable.name),
                key: variable.name.clone(),
                variable_type: variable.variable_type,
                description: Some(variable.description.clone()),
                source_kind: PromptVariableSourceKind::Runtime,
                source_label: variable.runtime_provider.source_label().to_string(),
                managed_variable_key: None,
                runtime_provider: Some(variable.runtime_provider),
            }),
    );
    options.sort_by(|left, right| {
        left.source_kind
            .code()
            .cmp(right.source_kind.code())
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.id.cmp(&right.id))
    });
    options
}

fn matches_prompt_variable_option(
    variable: &crate::domain::prompt::BuiltinPromptParameter,
    search: Option<&str>,
) -> bool {
    let Some(search) = search else {
        return true;
    };
    format!(
        "{} {} {} {}",
        variable.name,
        variable.description,
        variable.runtime_provider.code(),
        variable.runtime_provider.source_label(),
    )
    .to_lowercase()
    .contains(&search.to_lowercase())
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
