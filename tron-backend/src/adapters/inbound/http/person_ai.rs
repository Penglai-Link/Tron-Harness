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
        let (status, zh, en) = match descriptor.message_key {
            "tron.personAi.device_unbound" => (
                StatusCode::UNAUTHORIZED,
                "设备已解绑，请重新授权",
                "Device unbound; authorize again",
            ),
            "tron.personAi.auth_required" => (
                StatusCode::UNAUTHORIZED,
                "请先完成 Person-AI 授权",
                "Person-AI authorization required",
            ),
            "tron.personAi.invalid_request" => {
                (StatusCode::BAD_REQUEST, "请求参数无效", "Invalid request")
            }
            "tron.personAi.storage_failed" => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "账户数据读写失败",
                "Could not access account storage",
            ),
            _ => (
                StatusCode::BAD_GATEWAY,
                "Person-AI 请求失败",
                "Person-AI request failed",
            ),
        };
        let message = if let ModelProviderError::Remote(detail)
        | ModelProviderError::InvalidSelection(detail) = &self.0
        {
            detail.clone()
        } else if self.1.starts_with("en") {
            en.to_owned()
        } else {
            zh.to_owned()
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
    today_checked_in: bool,
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
    let result = state
        .model_providers
        .checkin_status()
        .await
        .map_err(|error| PersonAiApiError(error, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(CheckinStatusVo {
        today_checked_in: result.today_checked_in,
        today_reward_points: result.today_reward_points,
    })))
}
async fn checkin(
    State(state): State<AppState>,
    auth: RequestAuthContext,
) -> Result<Json<ApiResponse<CheckinVo>>, PersonAiApiError> {
    let result = state
        .model_providers
        .checkin()
        .await
        .map_err(|error| PersonAiApiError(error, auth.locale))?;
    Ok(Json(ApiResponse::protocol_success(CheckinVo {
        points: result.points,
        continuous_days: result.continuous_days,
    })))
}
