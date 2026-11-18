use crate::{
    app::AppState,
    application::agent::{installed_extension_implementations, query_installed_capability_packs},
    response::ApiResponse,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::Deserialize;

mod center;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(center::routes())
        .route("/api/v1/capabilities/packs", get(list_capability_packs))
        .route(
            "/api/v1/capabilities/packs/{id}/pages/{page}",
            get(get_page),
        )
        .route("/api/v1/capabilities/packs/{id}/pages", get(list_pages))
        .route("/api/v1/capabilities/packs/{id}", delete(uninstall_pack))
        .route(
            "/api/v1/capabilities/packs/import",
            post(import_pack).layer(DefaultBodyLimit::max(128 * 1024 * 1024)),
        )
        .route(
            "/api/v1/capabilities/packs/{id}/enabled",
            put(set_pack_enabled),
        )
        .route(
            "/api/v1/capabilities/extensions",
            get(list_extension_implementations),
        )
        .route(
            "/api/v1/capabilities/extensions/{point}/enabled",
            put(set_extension_enabled),
        )
        .route(
            "/api/v1/capabilities/extensions/tool-policy/config",
            get(get_tool_policy_config).put(update_tool_policy_config),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateToolPolicyConfigRequest {
    smart: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GetToolPolicyConfigVo {
    smart: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateToolPolicyConfigVo {
    smart: bool,
}

async fn get_tool_policy_config(
    State(state): State<AppState>,
) -> Json<ApiResponse<GetToolPolicyConfigVo>> {
    Json(ApiResponse::protocol_success(GetToolPolicyConfigVo {
        smart: state.agent_framework.tool_policy_smart(),
    }))
}

async fn update_tool_policy_config(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(state): State<AppState>,
    request: Result<Json<UpdateToolPolicyConfigRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(_) => {
            return tool_policy_failure(
                StatusCode::BAD_REQUEST,
                crate::application::capability::error::TOOL_POLICY_INVALID,
                &auth.locale,
            );
        }
    };
    match state
        .agent_framework
        .set_tool_policy_smart(request.smart)
        .await
    {
        Ok(()) => Json(ApiResponse::protocol_success(UpdateToolPolicyConfigVo {
            smart: request.smart,
        }))
        .into_response(),
        Err(error) => {
            tracing::error!(%error, "failed to persist tool policy configuration");
            tool_policy_failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                crate::application::capability::error::TOOL_POLICY_SAVE_FAILED,
                &auth.locale,
            )
        }
    }
}

fn tool_policy_failure(
    status: StatusCode,
    descriptor: crate::application::capability::error::ApplicationErrorDescriptor,
    locale: &str,
) -> Response {
    let message_key = match descriptor.message_key {
        "tron.capability.tool_policy_invalid" => "tron.capability.tool_policy_invalid",
        _ => "tron.capability.tool_policy_save_failed",
    };
    let message = tron_i18n::t!(message_key, locale = locale);
    (
        status,
        Json(ApiResponse::protocol_error(descriptor.code, message)),
    )
        .into_response()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PackListQuery {
    #[serde(default = "include_contents_by_default")]
    include_contents: bool,
    id: Option<String>,
}
fn include_contents_by_default() -> bool {
    true
}
async fn list_capability_packs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PackListQuery>,
) -> Response {
    let locale = crate::shared::locale::resolve_request_locale(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    );
    let (packs, _) = state.agent_framework.capability_state();
    let root = state.agent_framework.pack_root().to_path_buf();
    let records = tokio::task::spawn_blocking(move || {
        query_installed_capability_packs(
            &root,
            &locale,
            query.include_contents,
            query.id.as_deref(),
        )
    })
    .await;
    match records {
        Err(error) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CAPABILITY_LIST_FAILED",
            error.to_string(),
        ),
        Ok(result) => match result {
            Ok(records) => Json(ApiResponse::success(
                records
                    .into_iter()
                    .map(|mut pack| {
                        pack.effective = packs.get(&pack.id).copied().unwrap_or(pack.effective);
                        pack
                    })
                    .collect::<Vec<_>>(),
            ))
            .into_response(),
            Err(error) => failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CAPABILITY_LIST_FAILED",
                error,
            ),
        },
    }
}

async fn list_extension_implementations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let locale = crate::shared::locale::resolve_request_locale(
        headers
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    );
    list_extension_implementations_for(state, &locale).await
}

async fn list_extension_implementations_for(state: AppState, locale: &str) -> Response {
    let (packs, extensions) = state.agent_framework.capability_state();
    match installed_extension_implementations(
        state.agent_framework.pack_root(),
        &packs,
        &extensions,
        locale,
    ) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CAPABILITY_LIST_FAILED",
            error,
        ),
    }
}

#[derive(Deserialize)]
struct EnabledRequest {
    enabled: bool,
}
async fn set_pack_enabled(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<EnabledRequest>,
) -> Response {
    let is_builtin = crate::application::agent::capability_pack_catalog()
        .iter()
        .any(|p| p.id == id);
    if !is_builtin {
        let root = state.agent_framework.pack_root().to_path_buf();
        let package = id.clone();
        let installed = crate::app::telemetry::spawn_blocking_traced(move || {
            eliteforge_agent_capability_runtime::CapabilityPackManager::new(root).and_then(
                |manager| {
                    manager
                        .installed()
                        .map(|packs| packs.into_iter().any(|pack| pack.manifest.id == package))
                },
            )
        });
        match installed.await {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => {
                let locale = crate::shared::locale::resolve_request_locale(
                    headers
                        .get(axum::http::header::ACCEPT_LANGUAGE)
                        .and_then(|value| value.to_str().ok()),
                );
                return failure(
                    StatusCode::NOT_FOUND,
                    "CAPABILITY_PACK_NOT_FOUND",
                    tron_i18n::t!("capabilities.pack_not_installed", locale = &locale).into_owned(),
                );
            }
            Ok(Err(error)) => {
                return failure(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CAPABILITY_LIST_FAILED",
                    error.to_string(),
                );
            }
            Err(error) => {
                return failure(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CAPABILITY_LIST_FAILED",
                    error.to_string(),
                );
            }
        }
    }
    match state
        .agent_framework
        .set_pack_enabled(&id, request.enabled)
        .await
    {
        Ok(()) => {
            list_capability_packs(
                State(state),
                headers,
                Query(PackListQuery {
                    include_contents: false,
                    id: None,
                }),
            )
            .await
        }
        Err(error) => failure(StatusCode::CONFLICT, "CAPABILITY_RECONFIGURE_FAILED", error),
    }
}
async fn set_extension_enabled(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(state): State<AppState>,
    Path(point): Path<String>,
    Json(request): Json<EnabledRequest>,
) -> Response {
    if !matches!(
        point.as_str(),
        "model" | "tool" | "observe" | "context" | "runtime" | "storage"
    ) {
        return failure(
            StatusCode::NOT_FOUND,
            "EXTENSION_POINT_NOT_FOUND",
            tron_i18n::t!(
                "capabilities.extension_point_not_found",
                locale = &auth.locale
            )
            .into_owned(),
        );
    }
    match state
        .agent_framework
        .set_extension_enabled(&point, request.enabled)
        .await
    {
        Ok(()) => list_extension_implementations_for(state, &auth.locale).await,
        Err(error) => failure(StatusCode::CONFLICT, "CAPABILITY_RECONFIGURE_FAILED", error),
    }
}
async fn import_pack(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    let mut bytes = None;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) if field.name() == Some("file") => match field.bytes().await {
                Ok(value) if value.len() <= 128 * 1024 * 1024 => {
                    bytes = Some(value);
                    break;
                }
                Ok(_) => {
                    return failure(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "CAPABILITY_PACKAGE_TOO_LARGE",
                        tron_i18n::t!("capabilities.package_too_large", locale = &auth.locale)
                            .into_owned(),
                    );
                }
                Err(error) => {
                    return failure(
                        StatusCode::BAD_REQUEST,
                        "CAPABILITY_UPLOAD_FAILED",
                        error.to_string(),
                    );
                }
            },
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(error) => {
                return failure(
                    StatusCode::BAD_REQUEST,
                    "CAPABILITY_UPLOAD_FAILED",
                    error.to_string(),
                );
            }
        }
    }
    let Some(bytes) = bytes else {
        return failure(
            StatusCode::BAD_REQUEST,
            "CAPABILITY_FILE_REQUIRED",
            tron_i18n::t!("capabilities.file_required", locale = &auth.locale).into_owned(),
        );
    };
    match state
        .agent_framework
        .import_pack(bytes.to_vec(), &auth.locale)
        .await
    {
        Ok(pack) => Json(ApiResponse::protocol_success(pack)).into_response(),
        Err(error) => failure(StatusCode::CONFLICT, "CAPABILITY_IMPORT_REJECTED", error),
    }
}
fn failure(status: StatusCode, code: &'static str, message: String) -> Response {
    (status, Json(ApiResponse::error(code, message))).into_response()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UninstallPackVo {
    id: String,
    uninstalled: bool,
}

async fn uninstall_pack(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.agent_framework.uninstall_pack(&id).await {
        Ok(()) => Json(ApiResponse::protocol_success(UninstallPackVo {
            id,
            uninstalled: true,
        }))
        .into_response(),
        Err(error) => {
            let (code, message_key) = if error.contains("bundled") {
                ("101031001", "capabilities.uninstall_bundled")
            } else if error.contains("referenced")
                || error.contains("Busy")
                || error.contains("busy")
                || error.contains("active")
            {
                ("101031002", "capabilities.uninstall_busy")
            } else {
                ("101031003", "capabilities.uninstall_incomplete")
            };
            (
                StatusCode::CONFLICT,
                Json(ApiResponse::protocol_error(
                    code,
                    format!(
                        "{}: {error}",
                        tron_i18n::t!(message_key, locale = &auth.locale)
                    ),
                )),
            )
                .into_response()
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GetPageVo {
    pack_id: String,
    version: String,
    digest: String,
    page_id: String,
    title: std::collections::BTreeMap<String, String>,
    entry: String,
    assets: std::collections::BTreeMap<String, String>,
}
async fn get_page(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(state): State<AppState>,
    Path((id, page)): Path<(String, String)>,
) -> Response {
    let host = state.agent_framework.clone();
    match tokio::task::spawn_blocking(move || host.capability_page(&id, &page)).await {
        Ok(Ok(p)) => (
            [(axum::http::header::CACHE_CONTROL, "no-store")],
            Json(ApiResponse::protocol_success(GetPageVo {
                pack_id: p.pack_id,
                version: p.version,
                digest: p.digest,
                page_id: p.page_id,
                title: p.title,
                entry: p.entry,
                assets: p.assets,
            })),
        )
            .into_response(),
        _ => failure(
            StatusCode::NOT_FOUND,
            "CAPABILITY_PAGE_UNAVAILABLE",
            tron_i18n::t!("capabilities.page_unavailable", locale = &auth.locale).into_owned(),
        ),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ListPagesVo {
    id: String,
    title: std::collections::BTreeMap<String, String>,
}
async fn list_pages(
    auth: crate::adapters::inbound::http::auth::RequestAuthContext,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let host = state.agent_framework.clone();
    match tokio::task::spawn_blocking(move || host.capability_page_catalog(&id)).await {
        Ok(Ok(pages)) => (
            [(axum::http::header::CACHE_CONTROL, "no-store")],
            Json(ApiResponse::protocol_success(
                pages
                    .into_iter()
                    .map(|p| ListPagesVo {
                        id: p.id,
                        title: p.title,
                    })
                    .collect::<Vec<_>>(),
            )),
        )
            .into_response(),
        _ => failure(
            StatusCode::NOT_FOUND,
            "CAPABILITY_PAGE_UNAVAILABLE",
            tron_i18n::t!("capabilities.page_unavailable", locale = &auth.locale).into_owned(),
        ),
    }
}
