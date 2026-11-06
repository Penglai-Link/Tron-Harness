use super::*;
use crate::application::llm::LlmRuntimeService;
use crate::domain::model::AutoRouterKind;

#[derive(Clone, Copy, Debug)]
pub enum SpeechInputError {
    InvalidAudio,
    Unavailable,
    ProviderFailed,
    Storage,
    UnsupportedSelection,
    Cancelled,
    RequestConflict,
    TooManyRequests,
    InvalidRequestId,
}

impl SpeechInputError {
    pub fn descriptor(self) -> ApplicationErrorDescriptor {
        let (code, message_key) = match self {
            Self::InvalidAudio => ("101032014", "tron.speech.invalid_audio"),
            Self::Unavailable => ("101032015", "tron.speech.unavailable"),
            Self::ProviderFailed => ("101032016", "tron.speech.provider_failed"),
            Self::Storage => ("101032017", "tron.speech.storage_failed"),
            Self::UnsupportedSelection => ("101032018", "tron.speech.unsupported_selection"),
            Self::Cancelled => ("101032019", "tron.speech.cancelled"),
            Self::RequestConflict => ("101032020", "tron.speech.request_conflict"),
            Self::TooManyRequests => ("101032021", "tron.speech.too_many_requests"),
            Self::InvalidRequestId => ("101032022", "tron.speech.invalid_request_id"),
        };
        ApplicationErrorDescriptor { code, message_key }
    }
}

impl ModelService {
    /// Media routes use the catalogue's stable provider/model order. Empty
    /// explicit selections stay empty; they never fall back to all providers.
    pub fn select_automatic_media_model(
        &self,
        kind: AutoRouterKind,
    ) -> Result<AutomaticRouteModel, ModelError> {
        if !matches!(
            kind,
            AutoRouterKind::SpeechRecognition
                | AutoRouterKind::Realtime
                | AutoRouterKind::ImageGeneration
        ) {
            return Err(ModelError::InvalidProtocolModelType);
        }
        self.automatic_route_models(kind)?
            .into_iter()
            .next()
            .ok_or_else(|| {
                ModelError::RuntimeResolutionFailed(
                    "automatic media route has no available models".into(),
                )
            })
    }

    pub async fn transcribe_audio(
        &self,
        user: &str,
        request_id: &str,
        audio: Vec<u8>,
        mime: &str,
        runtime: &LlmRuntimeService,
    ) -> Result<String, SpeechInputError> {
        let session = self.speech_sessions.start(user, request_id)?;
        tracing::info!(
            speech_request_id = request_id,
            "Speech input request started"
        );
        tokio::select! {
            biased;
            _ = session.cancelled() => {
                // Dropping the operation also drops both WebSocket halves. No
                // detached upload task or reusable socket can outlive this request.
                tracing::info!(speech_request_id = request_id, "Speech input cancelled; closing upstream transport");
                Err(SpeechInputError::Cancelled)
            }
            result = self.transcribe_audio_inner(user, audio, mime, runtime) => {
                tracing::info!(speech_request_id = request_id, succeeded = result.is_ok(), "Speech input request finished");
                result
            }
        }
    }

    pub fn cancel_transcription(
        &self,
        user: &str,
        request_id: &str,
    ) -> Result<(), SpeechInputError> {
        self.speech_sessions.cancel(user, request_id)?;
        tracing::info!(
            speech_request_id = request_id,
            "Speech input cancellation requested"
        );
        Ok(())
    }

    async fn transcribe_audio_inner(
        &self,
        user: &str,
        audio: Vec<u8>,
        mime: &str,
        runtime: &LlmRuntimeService,
    ) -> Result<String, SpeechInputError> {
        let file_name = match mime {
            "audio/wav" => "recording.wav",
            "audio/webm" => "recording.webm",
            "audio/mp4" => "recording.m4a",
            "audio/ogg" => "recording.ogg",
            "audio/mpeg" => "recording.mp3",
            _ => return Err(SpeechInputError::InvalidAudio),
        };
        if audio.is_empty() || audio.len() > 8 * 1024 * 1024 {
            return Err(SpeechInputError::InvalidAudio);
        }
        let sources = self
            .automatic_route_sources()
            .map_err(|_| SpeechInputError::Storage)?
            .remove(AutoRouterKind::SpeechRecognition.code())
            .unwrap_or_default();
        let model = self
            .automatic_route_catalog()
            .map_err(|_| SpeechInputError::Storage)?
            .into_iter()
            .find(|model| {
                model.eligible_for(AutoRouterKind::SpeechRecognition) && model.allowed_by(&sources)
            })
            .ok_or(SpeechInputError::Unavailable)?;
        let operation = async {
            if model.provider_id == "PersonAi" {
                self.providers
                    .transcribe_profile_audio(user, file_name, audio, &sources)
                    .await
                    .map_err(|error| match error {
                        crate::application::model_provider::ModelProviderError::InvalidSelection(_) => {
                            SpeechInputError::UnsupportedSelection
                        }
                        _ => SpeechInputError::ProviderFailed,
                    })
            } else {
                let config = self
                    .auxiliary_runtime_model(Some(&model.binding_id), ModelType::SpeechRecognition)
                    .map_err(|_| SpeechInputError::Storage)?
                    .ok_or(SpeechInputError::Unavailable)?;
                runtime
                    .transcribe_with_config(&config, audio, mime, file_name)
                    .await
                    .map_err(|_| SpeechInputError::ProviderFailed)
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(120), operation)
            .await
            .map_err(|error| {
                tracing::error!(provider = %model.provider_id, model = %model.model_code, error = %error,
                    "Speech input exceeded 120 second deadline; closing upstream transport");
                SpeechInputError::ProviderFailed
            })?
            .map(|text| text.trim().to_owned())
    }
}
