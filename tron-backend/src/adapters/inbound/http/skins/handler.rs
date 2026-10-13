use super::response::{ActivateSkinVo, ActiveSkinVo, InstallSkinVo, SkinVo};
use super::{error_mapper::SkinApiError, request::ActivateSkinRequest};
use crate::adapters::inbound::http::auth::RequestAuthContext;
use crate::{
    app::AppState,
    application::skin::SkinError,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json,
    body::Bytes,
    extract::{
        Path, State,
        rejection::{BytesRejection, JsonRejection},
    },
    http::header,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
};
use std::{convert::Infallible, time::Duration};

pub(super) async fn blocking<T: Send + 'static>(
    locale: String,
    run: impl FnOnce() -> Result<T, SkinError> + Send + 'static,
) -> Result<T, SkinApiError> {
    crate::app::telemetry::spawn_blocking_traced(run)
        .await
        .map_err(|_| SkinApiError(SkinError::Storage, locale.clone()))?
        .map_err(|error| SkinApiError(error, locale))
}

pub async fn list(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<Vec<SkinVo>>>, SkinApiError> {
    let skins = blocking(auth.locale, move || state.skins.list()).await?;
    Ok(Json(ApiResponse::protocol_success(
        skins.into_iter().map(Into::into).collect(),
    )))
}
pub async fn active(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<ActiveSkinVo>>, SkinApiError> {
    let skin = blocking(auth.locale, move || state.skins.active()).await?;
    Ok(Json(ApiResponse::protocol_success(ActiveSkinVo {
        skin: skin.into(),
    })))
}
pub async fn activate(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    request: Result<Json<ActivateSkinRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<ActivateSkinVo>>, SkinApiError> {
    let Json(request) =
        request.map_err(|_| SkinApiError(SkinError::Invalid, auth.locale.clone()))?;
    let skin = blocking(auth.locale, move || state.skins.activate(&request.id)).await?;
    Ok(Json(ApiResponse::protocol_success(ActivateSkinVo {
        skin: skin.into(),
    })))
}
pub async fn install(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<ApiResponse<InstallSkinVo>>, SkinApiError> {
    let body = body.map_err(|_| SkinApiError(SkinError::TooLarge, auth.locale.clone()))?;
    let skin = blocking(auth.locale, move || state.skins.install(&body)).await?;
    Ok(Json(ApiResponse::protocol_success(InstallSkinVo {
        skin: skin.into(),
    })))
}
pub async fn uninstall(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmptyData>>, SkinApiError> {
    blocking(auth.locale, move || state.skins.uninstall(&id)).await?;
    Ok(Json(ApiResponse::protocol_success(EmptyData {})))
}
pub async fn resource(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((id, path)): Path<(String, String)>,
) -> Result<Response, SkinApiError> {
    let mime = crate::domain::skin::resource_mime(&path)
        .ok_or_else(|| SkinApiError(SkinError::Invalid, auth.locale.clone()))?;
    let bytes = blocking(auth.locale, move || state.skins.resource(&id, &path)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "private, max-age=3600"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    )
        .into_response())
}
pub async fn events(State(state): State<AppState>) -> Response {
    let mut receiver = state.skins.subscribe();
    let stream = async_stream::stream! {
        // Refresh on every connection, including reconnects after a missed change.
        yield Ok::<_, Infallible>(Event::default().event("skin-changed").data("{}"));
        loop {
            match receiver.recv().await {
                Ok(()) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("skin-changed").data("{}"));
                },
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(20)))
        .into_response()
}
