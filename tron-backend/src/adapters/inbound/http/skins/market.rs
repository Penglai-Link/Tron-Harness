use super::{error_mapper::SkinApiError, handler::blocking, response::InstallSkinVo};
use crate::adapters::inbound::http::{auth::RequestAuthContext, person_ai::PersonAiApiError};
use crate::{app::AppState, response::ApiResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct MarketQuery {
    current: Option<i32>,
    size: Option<i32>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSkinVo {
    id: String,
    name: String,
    author: Option<String>,
    preview_url: Option<String>,
    download_count: Option<i64>,
}
#[derive(Serialize)]
pub struct MarketPageVo {
    current: i32,
    size: i32,
    total: i64,
    records: Vec<MarketSkinVo>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Query(query): Query<MarketQuery>,
) -> Result<Json<ApiResponse<MarketPageVo>>, PersonAiApiError> {
    let current = query.current.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);
    let result = state
        .model_providers
        .skin_market(current, size)
        .await
        .map_err(|error| PersonAiApiError(error, auth.locale))?;
    let records = result
        .items
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            Some(MarketSkinVo {
                id: item.id?,
                name: item.name.unwrap_or_default(),
                author: item.author,
                preview_url: item.preview_url.filter(|url| url.starts_with("https://")),
                download_count: item.download_count,
            })
        })
        .collect();
    Ok(Json(ApiResponse::protocol_success(MarketPageVo {
        current,
        size,
        total: result.total.unwrap_or(0),
        records,
    })))
}

pub enum MarketInstallApiError {
    Remote(PersonAiApiError),
    Local(SkinApiError),
}
impl IntoResponse for MarketInstallApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Remote(error) => error.into_response(),
            Self::Local(error) => error.into_response(),
        }
    }
}

pub async fn install(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<InstallSkinVo>>, MarketInstallApiError> {
    let bytes = state
        .model_providers
        .download_market_skin(&id)
        .await
        .map_err(|error| {
            MarketInstallApiError::Remote(PersonAiApiError(error, auth.locale.clone()))
        })?;
    let skin = blocking(auth.locale, move || state.skins.install_market(&bytes, &id))
        .await
        .map_err(MarketInstallApiError::Local)?;
    Ok(Json(ApiResponse::protocol_success(InstallSkinVo {
        skin: skin.into(),
    })))
}
