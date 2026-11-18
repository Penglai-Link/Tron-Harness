use super::super::auth::RequestAuthContext;
use crate::{
    app::AppState, application::model::SpeechInputError, response::ApiResponse,
    shared::locale::SupportedLocale,
};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TranscribeAudioRequest {
    request_id: String,
    audio_base64: String,
    mime_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TranscribeAudioVo {
    text: String,
}

pub(super) struct SpeechError {
    error: SpeechInputError,
    locale: SupportedLocale,
}
impl IntoResponse for SpeechError {
    fn into_response(self) -> Response {
        let descriptor = self.error.descriptor();
        let (status, message_key) = match descriptor.message_key {
            "tron.speech.invalid_audio" => (StatusCode::BAD_REQUEST, "tron.speech.invalid_audio"),
            "tron.speech.unavailable" => {
                (StatusCode::SERVICE_UNAVAILABLE, "tron.speech.unavailable")
            }
            "tron.speech.provider_failed" => {
                (StatusCode::BAD_GATEWAY, "tron.speech.provider_failed")
            }
            "tron.speech.unsupported_selection" => {
                (StatusCode::BAD_REQUEST, "tron.speech.unsupported_selection")
            }
            "tron.speech.cancelled" => (StatusCode::CONFLICT, "tron.speech.cancelled"),
            "tron.speech.request_conflict" => {
                (StatusCode::CONFLICT, "tron.speech.request_conflict")
            }
            "tron.speech.too_many_requests" => (
                StatusCode::TOO_MANY_REQUESTS,
                "tron.speech.too_many_requests",
            ),
            "tron.speech.invalid_request_id" => {
                (StatusCode::BAD_REQUEST, "tron.speech.invalid_request_id")
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "tron.speech.storage_failed",
            ),
        };
        (
            status,
            Json(ApiResponse::protocol_error(
                descriptor.code,
                tron_i18n::t!(message_key, locale = self.locale),
            )),
        )
            .into_response()
    }
}

pub(super) async fn transcribe(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    headers: HeaderMap,
    request: Result<Json<TranscribeAudioRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<TranscribeAudioVo>>, SpeechError> {
    let locale =
        SupportedLocale::from_request(headers.get("accept-language").and_then(|v| v.to_str().ok()));
    let error = |error| SpeechError { error, locale };
    let Json(request) = request.map_err(|_| error(SpeechInputError::InvalidAudio))?;
    if request.audio_base64.len() > 12 * 1024 * 1024 {
        return Err(error(SpeechInputError::InvalidAudio));
    }
    let audio = base64::engine::general_purpose::STANDARD
        .decode(request.audio_base64)
        .map_err(|_| error(SpeechInputError::InvalidAudio))?;
    let text = state
        .models
        .transcribe_audio(
            &auth.user().user_code,
            &request.request_id,
            audio,
            &request.mime_type,
            &state.llm_runtime,
        )
        .await
        .map_err(error)?;
    Ok(Json(ApiResponse::protocol_success(TranscribeAudioVo {
        text,
    })))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CancelTranscriptionVo {
    cancelled: bool,
}

pub(super) async fn cancel(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(request_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<CancelTranscriptionVo>>, SpeechError> {
    let locale =
        SupportedLocale::from_request(headers.get("accept-language").and_then(|v| v.to_str().ok()));
    state
        .models
        .cancel_transcription(&auth.user().user_code, &request_id)
        .map_err(|error| SpeechError { error, locale })?;
    Ok(Json(ApiResponse::protocol_success(CancelTranscriptionVo {
        cancelled: true,
    })))
}
