use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    application::system_config::SystemConfigService, response::ApiResponse,
};

use super::{
    error_mapper::{admin_required_response, system_config_error_response},
    request::UpdateSystemConfigRequest,
};

pub async fn get_system_config(
    State(service): State<SystemConfigService>,
    auth: RequestAuthContext,
) -> Response {
    if !auth.user().role_codes.iter().any(|role| role == "ADMIN") {
        return admin_required_response();
    }
    match service.get() {
        Ok(document) => Json(ApiResponse::success(document)).into_response(),
        Err(error) => system_config_error_response(error),
    }
}

pub async fn update_system_config(
    State(service): State<SystemConfigService>,
    auth: RequestAuthContext,
    Json(request): Json<UpdateSystemConfigRequest>,
) -> Response {
    if !auth.user().role_codes.iter().any(|role| role == "ADMIN") {
        return admin_required_response();
    }
    match service.update(
        &request.revision,
        request.changes,
        request.confirm_migration,
    ) {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => system_config_error_response(error),
    }
}
