use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthorizationSessionView {
    pub provider_code: ModelProviderCode,
    pub session_id: String,
    pub login_url: String,
    pub status: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct CreateProviderAuthorizationCommand {
    pub provider_code: ModelProviderCode,
    pub server_base_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompleteProviderAuthorizationCommand {
    pub session_id: String,
    pub expected_person_ai_user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteProviderAuthorizationResult {
    pub session: ProviderAuthorizationSessionView,
    pub connection: ProviderConnectionView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelProviderError {
    AuthorizationSessionNotFound,
    AuthorizationPending,
    AuthorizationEnded(String),
    AuthorizationAccountMismatch,
    InvalidServerBaseUrl,
    InvalidEncryptionKey,
    ConnectionNotReady,
    CatalogModelUnavailable(String),
    InvalidSelection(String),
    AuthenticationRequired,
    Unauthorized,
    DeviceUnbound,
    Remote(String),
    Storage,
}

impl std::fmt::Display for ModelProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthorizationSessionNotFound => {
                formatter.write_str("authorization session was not found")
            }
            Self::AuthorizationPending => formatter.write_str("authorization is still pending"),
            Self::AuthorizationEnded(status) => {
                write!(formatter, "authorization ended with status {status}")
            }
            Self::AuthorizationAccountMismatch => formatter.write_str(
                "Authorize with the current profile's PersonAI account; add another account from the profile page",
            ),
            Self::InvalidServerBaseUrl => {
                formatter.write_str("Person-AI serverBaseUrl must be an HTTP(S) URL")
            }
            Self::InvalidEncryptionKey => formatter.write_str("encryption key is invalid"),
            Self::ConnectionNotReady => formatter.write_str("Person-AI connection is not ready"),
            Self::CatalogModelUnavailable(model_code) => {
                write!(
                    formatter,
                    "Person-AI model `{model_code}` is no longer available"
                )
            }
            Self::InvalidSelection(message) => formatter.write_str(message),
            Self::AuthenticationRequired => {
                formatter.write_str("Person-AI authorization is required")
            }
            Self::Unauthorized => {
                formatter.write_str("Person-AI access token was rejected (HTTP 401)")
            }
            Self::DeviceUnbound => formatter.write_str("Person-AI device was unbound; login again"),
            Self::Remote(message) => formatter.write_str(message),
            Self::Storage => formatter.write_str("model provider storage failed"),
        }
    }
}

impl std::error::Error for ModelProviderError {}

/// The stable provider integration point. A later provider only needs to implement this
/// adapter; the connection/catalogue/default-selection storage stays shared.
pub trait ModelProviderAdapter: Send + Sync {
    fn code(&self) -> ModelProviderCode;

    fn create_authorization<'a>(
        &'a self,
        server_base_url: &'a str,
        client_name: &'a str,
        existing_client_id: Option<&'a str>,
        device_id: &'a str,
    ) -> ProviderFuture<'a, ExternalAuthorizationSession>;

    fn poll_authorization<'a>(
        &'a self,
        server_base_url: &'a str,
        session_id: &'a str,
    ) -> ProviderFuture<'a, ExternalAuthorizationPoll>;

    fn refresh_credentials<'a>(
        &'a self,
        server_base_url: &'a str,
        refresh_token: &'a str,
    ) -> ProviderFuture<'a, ProviderCredentials>;

    fn list_chat_models<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, Vec<ExternalCatalogModel>>;

    fn list_multimedia_models<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, Vec<ExternalMultimediaCatalogModel>>;
}

#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    pub(super) token: String,
    pub(super) refresh_token: String,
    pub(super) client_id: String,
    pub(super) public_key: String,
    pub(super) server_base_url: String,
    pub(super) authorization_expires_at: Option<i64>,
}

impl ProviderCredentials {
    pub(super) fn context(&self) -> AuthorizedClientContext {
        AuthorizedClientContext::default()
            .with_token(self.token.clone())
            .with_refresh_token(self.refresh_token.clone())
            .with_client_id(self.client_id.clone())
            .with_public_key(self.public_key.clone())
            .with_product_code(PERSON_AI_PRODUCT_CODE)
            .with_product_version(PERSON_AI_PRODUCT_VERSION)
    }
}

#[derive(Debug, Clone)]
pub struct ExternalAuthorizationSession {
    pub(super) session_id: String,
    pub(super) login_url: String,
    pub(super) status: String,
    pub(super) expires_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum ExternalAuthorizationPoll {
    Pending {
        status: String,
        expires_at: Option<i64>,
    },
    Completed(ProviderCredentials),
    Ended {
        status: String,
    },
}

#[derive(Debug, Clone)]
pub struct ExternalCatalogModel {
    pub(super) tags: Vec<String>,
    pub(super) model_code: String,
    pub(super) model_name: String,
    pub(super) protocols: Vec<AgentModelProtocol>,
    pub(super) upstream_vendor_code: Option<String>,
    pub(super) upstream_vendor_name: Option<String>,
    pub(super) context_window_tokens: Option<u32>,
    pub(super) quality_type: ProviderModelQuality,
    pub(super) input_coefficient: Option<f64>,
    pub(super) cache_hit_coefficient: Option<f64>,
    pub(super) output_coefficient: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ExternalMultimediaCatalogModel {
    pub(super) upstream_vendor_code: Option<String>,
    pub(super) upstream_vendor_name: Option<String>,
    pub(super) input_coefficient: Option<f64>,
    pub(super) cache_hit_coefficient: Option<f64>,
    pub(super) output_coefficient: Option<f64>,
    pub(super) model_id: String,
    pub(super) model_code: String,
    pub(super) model_name: String,
    pub(super) catalog_model_type: ProviderCatalogModelType,
    pub(super) dimension: Option<usize>,
    pub(super) tags: Vec<String>,
}
