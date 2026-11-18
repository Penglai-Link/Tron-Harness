use super::super::{auth::RequestAuthContext, person_ai::PersonAiApiError};
use crate::{
    app::AppState,
    application::capability::center::{self, CenterPack, CenterQuery, CenterStatus},
    response::ApiResponse,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/capabilities/center", get(page))
        .route("/api/v1/capabilities/center/categories", get(categories))
        .route("/api/v1/capabilities/center/packs/{id}/like", put(set_like))
        .route("/api/v1/capabilities/center/releases/{id}", get(detail))
        .route(
            "/api/v1/capabilities/center/releases/{id}/install",
            post(install),
        )
        .route(
            "/api/v1/capabilities/center/packs/{id}/versions",
            get(versions),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageRequest {
    #[serde(default = "one")]
    current: u64,
    #[serde(default = "twenty")]
    size: u64,
    keyword: Option<String>,
    category: Option<String>,
    source: Option<String>,
    #[serde(default)]
    status: CenterStatus,
}
fn one() -> u64 {
    1
}
fn twenty() -> u64 {
    20
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PageResponse {
    current: u64,
    size: u64,
    total: u64,
    records: Vec<CenterPack>,
    update_check_incomplete: bool,
}

async fn page(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    query: Result<Query<PageRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ApiResponse<PageResponse>>, PersonAiApiError> {
    let Query(q) = query.map_err(|_| invalid_request(&auth.locale))?;
    let (records, total, update_check_incomplete) = center::page(
        &state.model_providers,
        &state.agent_framework,
        &auth.user().user_code,
        &auth.locale,
        CenterQuery {
            current: q.current,
            size: q.size,
            keyword: q.keyword,
            category: q.category,
            source: q.source,
            status: q.status,
        },
    )
    .await
    .map_err(|e| PersonAiApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(PageResponse {
        current: q.current,
        size: q.size,
        total,
        records,
        update_check_incomplete,
    })))
}

#[derive(Serialize)]
struct CategoriesResponse {
    categories: Vec<String>,
}
async fn categories(
    auth: RequestAuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<CategoriesResponse>>, PersonAiApiError> {
    let categories = state
        .model_providers
        .capability_center_categories(&auth.user().user_code)
        .await
        .map_err(|e| PersonAiApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(CategoriesResponse {
        categories,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LikeRequest {
    liked: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LikeResponse {
    pack_id: String,
    like_count: u64,
    liked: bool,
}
async fn set_like(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(id): Path<String>,
    request: Result<Json<LikeRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ApiResponse<LikeResponse>>, PersonAiApiError> {
    let Json(request) = request.map_err(|_| invalid_request(&auth.locale))?;
    if id.trim().is_empty() || id.len() > 128 {
        return Err(invalid_request(&auth.locale));
    }
    let result = state
        .model_providers
        .capability_center_set_like(&auth.user().user_code, &id, request.liked)
        .await
        .map_err(|e| PersonAiApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(LikeResponse {
        pack_id: result.pack_id,
        like_count: result.like_count,
        liked: result.liked,
    })))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DetailResponse {
    pack: CenterPack,
    manifest: serde_json::Value,
    readme: Option<String>,
    release_notes: Option<String>,
    signing_key_id: Option<String>,
    installable: bool,
    install_block_reason: Option<String>,
}
async fn detail(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<DetailResponse>>, PersonAiApiError> {
    let detail = state
        .model_providers
        .capability_center_detail(&auth.user().user_code, &id)
        .await
        .map_err(|e| PersonAiApiError(e, auth.locale.clone()))?;
    let local = center::installed(&state.agent_framework, &auth.locale)
        .await
        .map_err(|e| PersonAiApiError(e, auth.locale.clone()))?;
    let install_block_reason = center::validate_release(&detail, &local)
        .err()
        .map(|e| block_message(e, &auth.locale));
    let pack = local.iter().find(|p| p.id == detail.summary.pack_id);
    Ok(Json(ApiResponse::protocol_success(DetailResponse {
        pack: CenterPack::from_release(detail.summary, pack),
        manifest: detail.manifest,
        readme: detail.readme,
        release_notes: detail.release_notes,
        signing_key_id: detail.signing_key_id,
        installable: install_block_reason.is_none(),
        install_block_reason,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VersionsRequest {
    #[serde(default = "one")]
    current: u64,
    #[serde(default = "twenty")]
    size: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionResponse {
    release_id: String,
    version: String,
}
#[derive(Serialize)]
struct VersionsResponse {
    current: u64,
    size: u64,
    total: u64,
    records: Vec<VersionResponse>,
}
async fn versions(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Result<Query<VersionsRequest>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<ApiResponse<VersionsResponse>>, PersonAiApiError> {
    let Query(q) = query.map_err(|_| invalid_request(&auth.locale))?;
    let result = state
        .model_providers
        .capability_center_page(
            &auth.user().user_code,
            &crab_serverapi::capability::CapabilityQuery {
                pack_id: Some(id),
                latest_only: Some(false),
                ..crab_serverapi::capability::CapabilityQuery::of(q.current, q.size)
            },
        )
        .await
        .map_err(|e| PersonAiApiError(e, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(VersionsResponse {
        current: result.current,
        size: result.size,
        total: result.total,
        records: result
            .records
            .into_iter()
            .map(|r| VersionResponse {
                release_id: r.id,
                version: r.version,
            })
            .collect(),
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallRequest {
    expected_installed_version: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallResponse {
    pack_id: String,
    version: String,
    enabled: bool,
}
async fn install(
    auth: RequestAuthContext,
    State(state): State<AppState>,
    Path(id): Path<String>,
    request: Result<Json<InstallRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(request)) => request,
        Err(_) => return invalid_request(&auth.locale).into_response(),
    };
    let detail = match state
        .model_providers
        .capability_center_detail(&auth.user().user_code, &id)
        .await
    {
        Ok(detail) => detail,
        Err(e) => return PersonAiApiError(e, auth.locale).into_response(),
    };
    let local = match center::installed(&state.agent_framework, &auth.locale).await {
        Ok(local) => local,
        Err(e) => return PersonAiApiError(e, auth.locale).into_response(),
    };
    if let Err(error) = center::validate_release(&detail, &local) {
        return rejected(block_message(error, &auth.locale), &auth.locale);
    }
    if local
        .iter()
        .find(|p| p.id == detail.summary.pack_id)
        .map(|p| &p.version)
        != request.expected_installed_version.as_ref()
    {
        return rejected(
            tron_i18n::t!("capabilities.center.version_changed", locale = &auth.locale)
                .into_owned(),
            &auth.locale,
        );
    }
    let bytes = match state
        .model_providers
        .capability_center_download(&auth.user().user_code, &detail)
        .await
    {
        Ok(bytes) => bytes,
        Err(e) => return PersonAiApiError(e, auth.locale).into_response(),
    };
    match state
        .agent_framework
        .import_center_pack(
            bytes,
            detail.manifest,
            request.expected_installed_version,
            &auth.locale,
        )
        .await
    {
        Ok(pack) => Json(ApiResponse::protocol_success(InstallResponse {
            pack_id: pack.id,
            version: pack.version,
            enabled: pack.effective,
        }))
        .into_response(),
        Err(error) => rejected(error, &auth.locale),
    }
}
fn rejected(error: String, locale: &str) -> Response {
    (
        StatusCode::CONFLICT,
        Json(ApiResponse::protocol_error(
            "101038003",
            format!(
                "{}: {error}",
                tron_i18n::t!("capabilities.center.install_incomplete", locale = locale)
            ),
        )),
    )
        .into_response()
}

fn invalid_request(locale: &str) -> PersonAiApiError {
    PersonAiApiError(
        crate::application::model_provider::ModelProviderError::InvalidSelection(
            tron_i18n::Message::new("capabilities.center.invalid_request"),
        ),
        locale.into(),
    )
}

fn block_message(block: center::InstallBlock, locale: &str) -> String {
    match block {
        center::InstallBlock::Manifest(detail) => format!(
            "{}: {detail}",
            tron_i18n::t!("capabilities.center.manifest_incompatible", locale = locale)
        ),
        center::InstallBlock::Identity => {
            tron_i18n::t!("capabilities.center.release_mismatch", locale = locale).into_owned()
        }
        center::InstallBlock::Dependency(value) => format!(
            "{}: {value}",
            tron_i18n::t!("capabilities.center.dependency_required", locale = locale)
        ),
        center::InstallBlock::Platform(value) => format!(
            "{}: {value}",
            tron_i18n::t!("capabilities.center.platform_unsupported", locale = locale)
        ),
        center::InstallBlock::Builtin => {
            tron_i18n::t!("capabilities.center.builtin", locale = locale).into_owned()
        }
        center::InstallBlock::NotNewer => {
            tron_i18n::t!("capabilities.center.not_newer", locale = locale).into_owned()
        }
    }
}
