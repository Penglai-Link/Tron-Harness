use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::resource::{
        ResourceService, ServerAccountAssignCommand, ServerAccountUpdateCommand,
        ServerAccountVerifyCommand,
    },
    domain::resource::{ResourceKind, ServerAuthenticationType},
    response::ApiResponse,
};

use super::{error_mapper::resource_error_response, handler::json_body};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountUpdateRequest {
    workspace_code: String,
    name: String,
    description: Option<String>,
    username: String,
    authentication_type: ServerAuthenticationType,
    password: Option<String>,
    private_key: Option<String>,
    private_key_passphrase: Option<String>,
    allow_inheritance: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountAssignRequest {
    workspace_code: String,
    server_codes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerAccountVerifyRequest {
    workspace_code: String,
    server_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ServerAccountCreateResponse {
    pub workspace_code: String,
    pub resource_code: String,
    pub kind: ResourceKind,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerAccountUpdateResponse {
    workspace_code: String,
    resource_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerAccountAssignResponse {
    workspace_code: String,
    resource_code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerAccountVerifyResponse {
    verified: bool,
}

pub async fn update_server_account(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    Path(account_code): Path<String>,
    request: Result<Json<ServerAccountUpdateRequest>, JsonRejection>,
) -> Response {
    let request = match json_body(request, &auth.locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command = ServerAccountUpdateCommand {
        workspace_code: request.workspace_code,
        account_code,
        name: request.name,
        description: request.description.unwrap_or_default(),
        username: request.username,
        authentication_type: request.authentication_type,
        password: request.password,
        private_key: request.private_key,
        private_key_passphrase: request.private_key_passphrase,
        allow_inheritance: request.allow_inheritance,
    };
    match resources.update_server_account_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ApiResponse::protocol_success(ServerAccountUpdateResponse {
            workspace_code: result.workspace_code,
            resource_code: result.resource_code,
        }))
        .into_response(),
        Err(error) => resource_error_response(error, &auth.locale),
    }
}

pub async fn assign_server_account(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    Path(account_code): Path<String>,
    request: Result<Json<ServerAccountAssignRequest>, JsonRejection>,
) -> Response {
    let request = match json_body(request, &auth.locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command = ServerAccountAssignCommand {
        workspace_code: request.workspace_code,
        account_code,
        server_codes: request.server_codes,
    };
    match resources.assign_server_account_for_http(&auth.user().user_code, command) {
        Ok(result) => Json(ApiResponse::protocol_success(ServerAccountAssignResponse {
            workspace_code: result.workspace_code,
            resource_code: result.resource_code,
        }))
        .into_response(),
        Err(error) => resource_error_response(error, &auth.locale),
    }
}

pub async fn verify_server_account(
    State(resources): State<ResourceService>,
    auth: RequestAuthContext,
    Path(account_code): Path<String>,
    request: Result<Json<ServerAccountVerifyRequest>, JsonRejection>,
) -> Response {
    let request = match json_body(request, &auth.locale) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let command = ServerAccountVerifyCommand {
        workspace_code: request.workspace_code,
        account_code,
        server_code: request.server_code,
    };
    match resources
        .verify_server_account_for_http(&auth.user().user_code, command)
        .await
    {
        Ok(()) => Json(ApiResponse::protocol_success(ServerAccountVerifyResponse {
            verified: true,
        }))
        .into_response(),
        Err(error) => resource_error_response(error, &auth.locale),
    }
}
