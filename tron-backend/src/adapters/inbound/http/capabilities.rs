use crate::{
    app::AppState,
    application::agent::{installed_capability_packs, installed_extension_implementations},
    response::ApiResponse,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/capabilities/packs", get(list_capability_packs))
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
    let message = match (descriptor.message_key, locale.starts_with("zh")) {
        ("tron.capability.tool_policy_invalid", true) => "请提供有效的 Smart 开关状态",
        ("tron.capability.tool_policy_invalid", false) => "Provide a valid Smart setting.",
        (_, true) => "工具权限策略保存失败，请重试",
        (_, false) => "Unable to save tool policy settings. Please retry.",
    };
    (
        status,
        Json(ApiResponse::protocol_error(descriptor.code, message)),
    )
        .into_response()
}

async fn list_capability_packs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let locale = crate::shared::locale::resolve_request_locale(
        headers.get(axum::http::header::ACCEPT_LANGUAGE).and_then(|value| value.to_str().ok()),
    );
    let (packs, _) = state.agent_framework.capability_state();
    match installed_capability_packs(state.agent_framework.pack_root(), &locale) {
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
    }
}

async fn list_extension_implementations(
    State(state): State<AppState>,
) -> Response {
    let (packs, extensions) = state.agent_framework.capability_state();
    match installed_extension_implementations(state.agent_framework.pack_root(), &packs, &extensions) {
        Ok(records) => Json(ApiResponse::success(records)).into_response(),
        Err(error) => failure(StatusCode::INTERNAL_SERVER_ERROR, "CAPABILITY_LIST_FAILED", error),
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
                return failure(
                    StatusCode::NOT_FOUND,
                    "CAPABILITY_PACK_NOT_FOUND",
                    "capability pack is not installed".into(),
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
        Ok(()) => list_capability_packs(State(state), headers).await,
        Err(error) => failure(StatusCode::CONFLICT, "CAPABILITY_RECONFIGURE_FAILED", error),
    }
}
async fn set_extension_enabled(
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
            "extension point was not found".into(),
        );
    }
    match state
        .agent_framework
        .set_extension_enabled(&point, request.enabled)
        .await
    {
        Ok(()) => list_extension_implementations(State(state)).await,
        Err(error) => failure(StatusCode::CONFLICT, "CAPABILITY_RECONFIGURE_FAILED", error),
    }
}
async fn import_pack(State(state): State<AppState>, mut multipart: Multipart) -> Response {
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
                        "package exceeds 128 MiB".into(),
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
            "file field is required".into(),
        );
    };
    match state.agent_framework.import_pack(bytes.to_vec()).await {
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
            let zh = auth.locale.starts_with("zh");
            let (code, message) = if error.contains("bundled") {
                (
                    "101031001",
                    if zh {
                        "随程序发布的能力包不能单独卸载"
                    } else {
                        "Bundled capability packs cannot be uninstalled"
                    },
                )
            } else if error.contains("referenced")
                || error.contains("Busy")
                || error.contains("busy")
                || error.contains("active")
            {
                (
                    "101031002",
                    if zh {
                        "能力包仍被运行或持久续跑引用，请结束运行后重试"
                    } else {
                        "Capability pack is in use by a running or durable session"
                    },
                )
            } else {
                (
                    "101031003",
                    if zh {
                        "能力包卸载未完成，资源已停用，请重试"
                    } else {
                        "Capability pack cleanup is incomplete; retry the uninstall"
                    },
                )
            };
            (
                StatusCode::CONFLICT,
                Json(ApiResponse::protocol_error(
                    code,
                    format!("{message}: {error}"),
                )),
            )
                .into_response()
        }
    }
}
