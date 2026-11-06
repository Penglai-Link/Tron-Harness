use super::storage_read::read_connection;
use super::{ModelProviderError, ModelProviderService, server_api_error};
use crate::domain::model_provider::ModelProviderCode;
use crab_serverapi::{
    AuthorizedClientContext, ServerApiError,
    base::PageResult,
    checkin::{CheckinClient, CheckinRecord, CheckinStatus},
    skin::{SkinMarketClient, SkinMarketItem, SkinMarketQuery},
};

impl ModelProviderService {
    pub async fn transcribe_profile_audio(
        &self, user: &str, _file_name: &str, audio: Vec<u8>,
        sources: &crate::domain::model::AutoRouteSources,
    ) -> Result<String, ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        // The upstream WebSocket endpoint chooses from its active STT pool and does
        // not accept model_id. A restricted selection is valid only when that
        // entire pool is allowed. Read the live pool before uploading audio.
        if sources.mode == crate::domain::model::AutoRouteSourceMode::SelectedModels {
            let catalog = crab_serverapi::multimedia::MultimediaClient::new(base.clone())
                .map_err(server_api_error)?;
            let models = self.person_ai_service_result(catalog.list_models(&context, Some(2)).await)?;
            if models.is_empty() || models.iter().any(|model| {
                model.media_type != 2
                    || !sources.allows("PersonAi", &format!("PERSON_AI:SPEECH:{}", model.model_id))
            }) {
                return Err(ModelProviderError::InvalidSelection(
                    "Person-AI transcription cannot restrict its active speech model pool".into(),
                ));
            }
        }
        super::person_ai_speech::transcribe(&base, &context, &audio)
            .await
            .map_err(ModelProviderError::Remote)
    }

    /// The durable knowledge task belongs to one Person-AI account. A profile
    /// switch must pause its remote work instead of submitting under another user.
    pub async fn document_parse_session(
        &self,
        user: &str,
    ) -> Result<(AuthorizedClientContext, String), ModelProviderError> {
        self.person_ai_profile_session(user).await
    }

    async fn person_ai_profile_session(
        &self,
        user: &str,
    ) -> Result<(AuthorizedClientContext, String), ModelProviderError> {
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let active = self.connection.call(
            move |connection| read_connection(connection, ModelProviderCode::PersonAi),
            || ModelProviderError::Storage,
        )?;
        let active = active.ok_or(ModelProviderError::AuthenticationRequired)?;
        let current = active
            .credentials_encrypted
            .as_deref()
            .and_then(|value| self.decrypt_credentials(value).ok())
            .ok_or(ModelProviderError::AuthenticationRequired)?;
        if active.person_ai_user_id.as_deref() != Some(user)
            || current.token != credentials.token
            || current.client_id != credentials.client_id
        {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        Ok((credentials.context(), credentials.server_base_url))
    }

    /// Both reads use the same captured account credentials, even if the active
    /// profile changes while either request is in flight.
    pub async fn profile_account_snapshot(
        &self,
        user: &str,
    ) -> Result<(crab_serverapi::auth::AccountInfo, CheckinStatus), ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        let account =
            crab_serverapi::auth::AccountClient::new(base.clone()).map_err(server_api_error)?;
        let checkin = CheckinClient::new(base).map_err(server_api_error)?;
        let (info, status) = tokio::join!(account.get_info(&context), checkin.status(&context));
        Ok((
            info.map_err(server_api_error)?,
            status.map_err(server_api_error)?,
        ))
    }

    pub async fn checkin_profile(&self, user: &str) -> Result<CheckinRecord, ModelProviderError> {
        let (context, base) = self.person_ai_profile_session(user).await?;
        let client = CheckinClient::new(base).map_err(server_api_error)?;
        self.person_ai_service_result(client.checkin(&context).await)
    }
    fn person_ai_service_result<T>(
        &self,
        result: Result<T, ServerApiError>,
    ) -> Result<T, ModelProviderError> {
        result.map_err(|error| {
            let mapped = match error {
                ServerApiError::Business { code: 401, .. }
                | ServerApiError::Gateway { status: 401, .. } => ModelProviderError::Unauthorized,
                ServerApiError::Business { code: 400, message }
                | ServerApiError::Validation(message) => {
                    ModelProviderError::InvalidSelection(message)
                }
                error => server_api_error(error),
            };
            self.mark_auth_required_after_rejected_credentials(&mapped);
            mapped
        })
    }

    pub async fn checkin_status(&self) -> Result<CheckinStatus, ModelProviderError> {
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let client =
            CheckinClient::new(credentials.server_base_url.clone()).map_err(server_api_error)?;
        self.person_ai_service_result(client.status(&credentials.context()).await)
    }

    pub async fn checkin(&self) -> Result<CheckinRecord, ModelProviderError> {
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let client =
            CheckinClient::new(credentials.server_base_url.clone()).map_err(server_api_error)?;
        self.person_ai_service_result(client.checkin(&credentials.context()).await)
    }

    pub async fn skin_market(
        &self,
        page: i32,
        size: i32,
    ) -> Result<PageResult<SkinMarketItem>, ModelProviderError> {
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let client =
            SkinMarketClient::new(credentials.server_base_url.clone()).map_err(server_api_error)?;
        // The SDK sends X-Product-Code from credentials.context(); tags describe style, not the product.
        let query = SkinMarketQuery::of(page, size);
        self.person_ai_service_result(client.list(&credentials.context(), &query).await)
    }

    pub async fn download_market_skin(&self, id: &str) -> Result<Vec<u8>, ModelProviderError> {
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(ModelProviderError::InvalidSelection(
                "Invalid market skin ID".into(),
            ));
        }
        let credentials = self.active_credentials(ModelProviderCode::PersonAi).await?;
        let client =
            SkinMarketClient::new(credentials.server_base_url.clone()).map_err(server_api_error)?;
        let download =
            self.person_ai_service_result(client.download(&credentials.context(), id).await)?;
        let url = download
            .file_url
            .ok_or_else(|| ModelProviderError::Remote("Missing theme download URL".into()))?;
        client.download_zip(&url).await.map_err(server_api_error)
    }
}

pub struct PersonAiServiceErrorDescriptor {
    pub code: &'static str,
    pub message_key: &'static str,
}

impl ModelProviderError {
    pub fn person_ai_service_descriptor(&self) -> PersonAiServiceErrorDescriptor {
        let (code, message_key) = match self {
            Self::DeviceUnbound => ("101026001", "tron.personAi.device_unbound"),
            Self::Unauthorized | Self::AuthenticationRequired | Self::ConnectionNotReady => {
                ("101026002", "tron.personAi.auth_required")
            }
            Self::InvalidSelection(_) => ("101026003", "tron.personAi.invalid_request"),
            Self::Storage => ("101026005", "tron.personAi.storage_failed"),
            _ => ("101026004", "tron.personAi.request_failed"),
        };
        PersonAiServiceErrorDescriptor { code, message_key }
    }
}
