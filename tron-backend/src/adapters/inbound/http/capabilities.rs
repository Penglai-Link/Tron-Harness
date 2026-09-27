use crate::{
    app::AppState,
    application::agent::{
        extension_implementation_catalog_with_state, install_capability_pack,
        installed_capability_packs,
    },
    response::ApiResponse,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/capabilities/packs", get(list_capability_packs))
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
}

async fn list_capability_packs(State(state): State<AppState>) -> Response {
    let (packs, _) = state.agent_framework.capability_state();
    match installed_capability_packs(state.agent_framework.pack_root()) {
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
) -> Json<ApiResponse<Vec<crate::application::agent::ExtensionImplementationInfo>>> {
    let (packs, mut extensions) = state.agent_framework.capability_state();
    if let Some(enabled) = packs.get("framework.local.scoped") {
        extensions.entry("tool".into()).or_insert(*enabled);
    }
    if let Some(enabled) = packs.get("tron.harness") {
        extensions.entry("context".into()).or_insert(*enabled);
    }
    if let Some(enabled) = packs.get("framework.external-agent") {
        extensions.entry("runtime".into()).or_insert(*enabled);
    }
    Json(ApiResponse::success(
        extension_implementation_catalog_with_state(&extensions),
    ))
}

#[derive(Deserialize)]
struct EnabledRequest {
    enabled: bool,
}
async fn set_pack_enabled(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<EnabledRequest>,
) -> Response {
    let is_builtin = matches!(
        id.as_str(),
        "framework.external-agent" | "framework.local.scoped" | "tron.harness"
    );
    if !is_builtin {
        let root = state.agent_framework.pack_root().to_path_buf();
        let package = id.clone();
        let installed = tokio::task::spawn_blocking(move || {
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
        Ok(()) => list_capability_packs(State(state)).await,
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
        Ok(()) => list_extension_implementations(State(state))
            .await
            .into_response(),
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
    let root = state.agent_framework.pack_root().to_path_buf();
    match tokio::task::spawn_blocking(move || install_capability_pack(&root, &bytes)).await {
        Ok(Ok(pack)) => Json(ApiResponse::success(pack)).into_response(),
        Ok(Err(error)) => failure(StatusCode::BAD_REQUEST, "CAPABILITY_IMPORT_REJECTED", error),
        Err(error) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CAPABILITY_IMPORT_FAILED",
            error.to_string(),
        ),
    }
}
fn failure(status: StatusCode, code: &'static str, message: String) -> Response {
    (status, Json(ApiResponse::error(code, message))).into_response()
}
