use crate::{
    app::AppState,
    domain::model::{AutoRouteSourceMode, AutoRouteSources, AutoRouterKind},
    response::ApiResponse,
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RouteModelVo {
    provider_id: String,
    provider_name: String,
    model_code: String,
    model_name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RouteSourcesVo {
    mode: AutoRouteSourceMode,
    provider_ids: Vec<String>,
    model_codes: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListAutomaticRouterVo {
    kind: AutoRouterKind,
    sources: RouteSourcesVo,
    candidates: Vec<RouteModelVo>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateAutomaticRouterRequest {
    mode: AutoRouteSourceMode,
    provider_ids: Vec<String>,
    model_codes: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateAutomaticRouterVo {
    kind: AutoRouterKind,
    mode: AutoRouteSourceMode,
    provider_ids: Vec<String>,
    model_codes: Vec<String>,
}

pub(super) struct RouteError {
    storage: bool,
    english: bool,
}
impl RouteError {
    fn new(storage: bool, headers: &HeaderMap) -> Self {
        Self {
            storage,
            english: crate::shared::locale::SupportedLocale::from_request(
                headers.get("accept-language").and_then(|h| h.to_str().ok()),
            ) == crate::shared::locale::SupportedLocale::EnUs,
        }
    }
}
impl IntoResponse for RouteError {
    fn into_response(self) -> Response {
        let descriptor = if self.storage {
            crate::application::model::AutomaticRoutingSettingsError::Storage
        } else {
            crate::application::model::AutomaticRoutingSettingsError::InvalidSources
        }
        .descriptor();
        let (status, message) = match (self.storage, self.english) {
            (true, true) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not load or save automatic router settings.",
            ),
            (true, false) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "无法读取或保存自动路由器配置。",
            ),
            (false, true) => (StatusCode::BAD_REQUEST, "Invalid automatic router sources."),
            (false, false) => (StatusCode::BAD_REQUEST, "自动路由器的数据来源无效。"),
        };
        (
            status,
            Json(ApiResponse::protocol_error(descriptor.code, message)),
        )
            .into_response()
    }
}
pub(super) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ListAutomaticRouterVo>>>, RouteError> {
    let catalog = state
        .models
        .automatic_route_catalog()
        .map_err(|_| RouteError::new(true, &headers))?;
    let mut policies = state
        .models
        .automatic_route_sources()
        .map_err(|_| RouteError::new(true, &headers))?;
    Ok(Json(ApiResponse::protocol_success(
        AutoRouterKind::ALL
            .into_iter()
            .map(|kind| {
                let sources = policies.remove(kind.code()).unwrap_or_default();
                ListAutomaticRouterVo {
                    kind,
                    sources: RouteSourcesVo {
                        mode: sources.mode,
                        provider_ids: sources.provider_ids,
                        model_codes: sources.model_codes,
                    },
                    candidates: catalog
                        .iter()
                        .filter(|m| m.eligible_for(kind))
                        .map(|m| RouteModelVo {
                            provider_id: m.provider_id.clone(),
                            provider_name: m.provider_name.clone(),
                            model_code: m.model_code.clone(),
                            model_name: m.model_name.clone(),
                        })
                        .collect(),
                }
            })
            .collect(),
    )))
}
pub(super) async fn update(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    headers: HeaderMap,
    Json(request): Json<UpdateAutomaticRouterRequest>,
) -> Result<Json<ApiResponse<UpdateAutomaticRouterVo>>, RouteError> {
    let kind = AutoRouterKind::ALL
        .into_iter()
        .find(|value| value.code() == kind)
        .ok_or_else(|| RouteError::new(false, &headers))?;
    let sources = AutoRouteSources {
        mode: request.mode,
        provider_ids: request.provider_ids,
        model_codes: request.model_codes,
    };
    state
        .models
        .save_automatic_route_sources(kind, sources.clone())
        .map_err(|error| {
            RouteError::new(
                matches!(
                    error,
                    crate::application::model::AutomaticRoutingSettingsError::Storage
                ),
                &headers,
            )
        })?;
    Ok(Json(ApiResponse::protocol_success(
        UpdateAutomaticRouterVo {
            kind,
            mode: sources.mode,
            provider_ids: sources.provider_ids,
            model_codes: sources.model_codes,
        },
    )))
}
