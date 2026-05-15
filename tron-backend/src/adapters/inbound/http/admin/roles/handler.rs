use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext, application::role::RoleService,
    response::ApiResponse,
};

use super::{error_mapper::role_error_response, request::ReplaceRolesRequest, response::RoleDto};

pub async fn list_permission_domains(State(roles): State<RoleService>) -> Response {
    Json(ApiResponse::success(roles.list_permission_domains())).into_response()
}

pub async fn list_roles(State(roles): State<RoleService>) -> Response {
    match roles.list_roles() {
        Ok(items) => Json(ApiResponse::success(
            items.into_iter().map(RoleDto::from).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => role_error_response(error),
    }
}

pub async fn replace_roles(
    State(roles): State<RoleService>,
    auth: RequestAuthContext,
    Json(request): Json<ReplaceRolesRequest>,
) -> Response {
    match roles.replace_roles(auth.user(), request.into()) {
        Ok(items) => Json(ApiResponse::success(
            items.into_iter().map(RoleDto::from).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => role_error_response(error),
    }
}
