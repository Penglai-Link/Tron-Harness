use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::system_config::SystemConfigService,
    response::{ApiResponse, EmptyData},
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
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<UpdateSystemConfigRequest>,
) -> Response {
    if !auth.user().role_codes.iter().any(|role| role == "ADMIN") {
        return admin_required_response();
    }
    match state.system_config.update(
        &request.revision,
        request.changes,
        request.confirm_migration,
    ) {
        Ok(result) => {
            if result
                .changed_paths
                .iter()
                .any(|path| path == "variableCrypto.keyBase64")
            {
                let key_base64 = match state.system_config.variable_crypto_key_base64() {
                    Ok(Some(key)) => key,
                    Ok(None) => String::new(),
                    Err(error) => return system_config_error_response(error),
                };
                if state.variables.configure_cipher(&key_base64).is_err() {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<EmptyData>::error(
                            "SYSTEM_CONFIG_CRYPTO_RELOAD_FAILED",
                            "encryption key could not be loaded",
                        )),
                    )
                        .into_response();
                }
            }
            Json(ApiResponse::success(result)).into_response()
        }
        Err(error) => system_config_error_response(error),
    }
}
