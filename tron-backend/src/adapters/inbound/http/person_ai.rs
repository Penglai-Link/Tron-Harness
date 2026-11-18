use super::auth::RequestAuthContext;
use crate::{
    app::AppState,
    application::model_provider::ModelProviderError,
    response::{ApiResponse, EmptyData},
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;

pub struct PersonAiApiError(pub ModelProviderError, pub String);
impl IntoResponse for PersonAiApiError {
    fn into_response(self) -> Response {
        let descriptor = self.0.person_ai_service_descriptor();
        let (status, message_key) = match descriptor.message_key {
            "tron.capability.pack_unavailable" => {
                (StatusCode::NOT_FOUND, "tron.capability.pack_unavailable")
            }
            "tron.capability.release_unavailable" => {
                (StatusCode::NOT_FOUND, "tron.capability.release_unavailable")
            }
            "tron.personAi.device_unbound" => {
                (StatusCode::UNAUTHORIZED, "tron.personAi.device_unbound")
            }
            "tron.personAi.auth_required" => {
                (StatusCode::UNAUTHORIZED, "tron.personAi.auth_required")
            }
            "tron.personAi.invalid_request" => {
                (StatusCode::BAD_REQUEST, "tron.personAi.invalid_request")
            }
            "tron.personAi.storage_failed" => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "tron.personAi.storage_failed",
            ),
            _ => (StatusCode::BAD_GATEWAY, "tron.personAi.request_failed"),
        };
        let message = if let ModelProviderError::Remote(detail)
        | ModelProviderError::InvalidSelection(detail) = &self.0
        {
            detail.clone()
        } else {
            tron_i18n::t!(message_key, locale = &self.1).into_owned()
        };
        (
            status,
            Json(ApiResponse::<EmptyData>::protocol_error(
                descriptor.code,
                message,
            )),
        )
            .into_response()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckinStatusVo {
    today_checked_in: Option<bool>,
    today_reward_points: Option<f64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckinVo {
    points: Option<f64>,
    continuous_days: Option<i32>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/v1/account/checkin", get(status).post(checkin))
}
async fn status(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<CheckinStatusVo>>, PersonAiApiError> {
    let cached = state
        .profiles
        .cached_account(&auth.user().user_code)
        .map_err(|_| PersonAiApiError(ModelProviderError::Storage, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(CheckinStatusVo {
        today_checked_in: cached.as_ref().and_then(|account| account.today_checked_in),
        today_reward_points: cached.and_then(|account| account.today_reward_points),
    })))
}
async fn checkin(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<CheckinVo>>, PersonAiApiError> {
    let result = state
        .account_sync
        .checkin(&auth.user().user_code)
        .await
        .map_err(|error| PersonAiApiError(error, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(CheckinVo {
        points: result.points,
        continuous_days: result.continuous_days,
    })))
}
