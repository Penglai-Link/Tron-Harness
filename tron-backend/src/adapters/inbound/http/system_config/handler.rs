use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{
    app::AppState,
    application::system_config::SystemConfigService,
    response::{ApiResponse, EmptyData},
};

use super::{error_mapper::system_config_error_response, request::UpdateSystemConfigRequest};

fn request_locale(headers: &HeaderMap) -> String {
    crate::shared::locale::resolve_request_locale(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    )
}

pub async fn get_system_config(
    State(service): State<SystemConfigService>,
    headers: HeaderMap,
) -> Response {
    let locale = request_locale(&headers);
    match service.get() {
        Ok(document) => Json(ApiResponse::success(document)).into_response(),
        Err(error) => system_config_error_response(error, &locale),
    }
}

pub async fn update_system_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateSystemConfigRequest>,
) -> Response {
    let locale = request_locale(&headers);
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
                    Err(error) => return system_config_error_response(error, &locale),
                };
                if state.variables.configure_cipher(&key_base64).is_err() {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<EmptyData>::error(
                            "SYSTEM_CONFIG_CRYPTO_RELOAD_FAILED",
                            tron_i18n::t!("system_config.crypto_reload_failed", locale = &locale),
                        )),
                    )
                        .into_response();
                }
            }
            Json(ApiResponse::success(result)).into_response()
        }
        Err(error) => system_config_error_response(error, &locale),
    }
}

#[derive(serde::Serialize)]
#[serde(transparent)]
pub struct ShortcutDefaultsResponse(
    std::collections::BTreeMap<String, crate::application::system_config::ShortcutBinding>,
);

pub async fn get_shortcut_defaults(
    State(service): State<SystemConfigService>,
    headers: HeaderMap,
) -> Response {
    let locale = request_locale(&headers);
    match service.shortcut_defaults() {
        Ok(defaults) => {
            Json(ApiResponse::success(ShortcutDefaultsResponse(defaults))).into_response()
        }
        Err(error) => system_config_error_response(error, &locale),
    }
}
