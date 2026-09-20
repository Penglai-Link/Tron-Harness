use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    future::Future,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
};

use chrono::DateTime;
use crab_serverapi::{
    AuthorizedClientApi, AuthorizedClientContext, ServerApiError,
    auth::{AccountClient, LoginClient, SessionRequest},
    gateway::{
        ChatCompletionsClient, EmbeddingsClient, ProtocolType, QualityType, ResponsesClient,
    },
    multimedia::MultimediaClient,
};
use reqwest::Method;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

#[path = "model_provider/chat_auto_routes.rs"]
mod chat_auto_routes;

use crate::{
    adapters::outbound::crypto::Sm4GcmVariableCipher,
    application::{
        llm::LlmRuntimeModelResolver,
        model::{ModelService, RuntimeModelConfig},
        variable::VariableCipher,
    },
    config::VariableCryptoConfig,
    domain::{
        agent::{AgentModelConfig, AgentModelProtocol},
        model::{ModelError, ModelProtocol, ModelType},
        model_provider::{
            ModelProviderCode, ModelSelection, ModelSelectionKind, ProviderAccountBalanceView,
            ProviderCatalogModelStatus, ProviderCatalogModelType, ProviderCatalogModelView,
            ProviderConnectionStatus, ProviderConnectionView, ProviderModelQuality,
            ProviderModelRunSnapshotView, ProviderModelSelectOption, ProviderRouteCode,
            ProviderSyncRunView, ProviderSyncStatus,
        },
    },
};

const MODEL_PROVIDER_MIGRATION: &str =
    include_str!("../../migrations/20260915_0001_create_model_provider_management.sql");
const PERSON_AI_DEFAULT_SERVER_BASE_URL: &str = "https://personai.chat/person-ai-index";
const PERSON_AI_CLIENT_NAME: &str = "Tron";
const PROVIDER_RUNTIME_PREFIX: &str = "PERSON_AI:";
const TOKEN_REFRESH_LEAD_MILLIS: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_PROVIDER_SYNC_RUNS: i64 = 10;

pub(crate) fn migrate_provider_catalog_model_schema(
    connection: &Connection,
) -> rusqlite::Result<()> {
    ensure_sqlite_column(
        connection,
        "provider_catalog_models",
        "quality_type",
        "TEXT NOT NULL DEFAULT 'High'",
    )?;
    for (column, definition) in [
        ("person_ai_user_id", "TEXT"),
        ("person_ai_username", "TEXT"),
    ] {
        ensure_sqlite_column(connection, "provider_connections", column, definition)?;
    }
    for (column, definition) in [
        ("protocols", "TEXT NOT NULL DEFAULT '[\"ChatCompletions\"]'"),
        ("input_coefficient", "REAL"),
        ("cache_hit_coefficient", "REAL"),
        ("output_coefficient", "REAL"),
    ] {
        ensure_sqlite_column(connection, "provider_catalog_models", column, definition)?;
    }
    if sqlite_table_exists_raw(connection, "default_model_selections")? {
        ensure_sqlite_column(
            connection,
            "default_model_selections",
            "quality_type",
            "TEXT",
        )?;
    }
    if sqlite_table_exists_raw(connection, "provider_model_run_snapshots")? {
        ensure_sqlite_column(
            connection,
            "provider_model_run_snapshots",
            "quality_type",
            "TEXT",
        )?;
    }
    Ok(())
}

fn ensure_sqlite_column(
    connection: &Connection,
    table_name: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    let columns = {
        let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
        statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<HashSet<_>, _>>()?
    };
    if !columns.contains(column) {
        connection.execute(
            &format!("ALTER TABLE {table_name} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ModelProviderError>> + Send + 'a>>;

fn provider_quality_from_sdk(quality: QualityType) -> ProviderModelQuality {
    match quality {
        QualityType::High => ProviderModelQuality::High,
        QualityType::Middle => ProviderModelQuality::Middle,
        QualityType::Low => ProviderModelQuality::Low,
    }
}

fn provider_quality_to_sdk(quality: ProviderModelQuality) -> QualityType {
    match quality {
        ProviderModelQuality::High => QualityType::High,
        ProviderModelQuality::Middle => QualityType::Middle,
        ProviderModelQuality::Low => QualityType::Low,
    }
}

fn agent_protocol_from_sdk(protocol: ProtocolType) -> AgentModelProtocol {
    match protocol {
        ProtocolType::ChatCompletions => AgentModelProtocol::ChatCompletions,
        ProtocolType::Responses => AgentModelProtocol::Responses,
    }
}

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
    /// The setup wizard supplies the key before the process has reloaded its
    /// persisted variable-crypto configuration. Normal administrator
    /// re-authorization deliberately leaves this empty: it must use the
    /// platform's existing credential cipher instead of asking an operator to
    /// handle the master key again.
    pub encryption_key_base64: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteProviderAuthorizationResult {
    pub session: ProviderAuthorizationSessionView,
    pub connection: ProviderConnectionView,
    pub text_verified: bool,
    pub embedding_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelProviderError {
    AuthorizationSessionNotFound,
    AuthorizationPending,
    AuthorizationEnded(String),
    InvalidServerBaseUrl,
    InvalidEncryptionKey,
    ConnectionNotReady,
    CatalogModelUnavailable(String),
    InvalidSelection(String),
    AuthenticationRequired,
    Unauthorized,
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

    fn verify_chat_auto<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, ()>;

    fn verify_embedding<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, ()>;

    fn chat<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
        quality_type: ProviderModelQuality,
        conversation_id: Option<&'a str>,
    ) -> ProviderFuture<'a, Value>;

    fn chat_stream<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
        quality_type: ProviderModelQuality,
        conversation_id: Option<&'a str>,
        sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
    ) -> ProviderFuture<'a, ()>;

    fn embed<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
    ) -> ProviderFuture<'a, Value>;
}

#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    token: String,
    refresh_token: String,
    client_id: String,
    public_key: String,
    server_base_url: String,
    authorization_expires_at: Option<i64>,
}

impl ProviderCredentials {
    fn context(&self) -> AuthorizedClientContext {
        AuthorizedClientContext::default()
            .with_token(self.token.clone())
            .with_refresh_token(self.refresh_token.clone())
            .with_client_id(self.client_id.clone())
            .with_public_key(self.public_key.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ExternalAuthorizationSession {
    session_id: String,
    login_url: String,
    status: String,
    expires_at: Option<i64>,
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
    model_code: String,
    model_name: String,
    protocols: Vec<AgentModelProtocol>,
    upstream_vendor_code: Option<String>,
    upstream_vendor_name: Option<String>,
    context_window_tokens: Option<u32>,
    quality_type: ProviderModelQuality,
    input_coefficient: Option<f64>,
    cache_hit_coefficient: Option<f64>,
    output_coefficient: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ExternalMultimediaCatalogModel {
    model_id: String,
    model_code: String,
    model_name: String,
    catalog_model_type: ProviderCatalogModelType,
}

#[derive(Debug, Clone, Default)]
pub struct PersonAiAdapter;

impl ModelProviderAdapter for PersonAiAdapter {
    fn code(&self) -> ModelProviderCode {
        ModelProviderCode::PersonAi
    }

    fn create_authorization<'a>(
        &'a self,
        server_base_url: &'a str,
        client_name: &'a str,
        existing_client_id: Option<&'a str>,
    ) -> ProviderFuture<'a, ExternalAuthorizationSession> {
        Box::pin(async move {
            let client = LoginClient::new(server_base_url)
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let session = client
                .create_session(&SessionRequest {
                    client_id: existing_client_id.map(str::to_owned),
                    client_name: client_name.to_owned(),
                    callback_url: None,
                })
                .await
                .map_err(remote_error)?;
            let login_url = session.login_url.unwrap_or_else(|| {
                client.build_login_url(
                    &session.session_id,
                    Some(LoginClient::detect_os_type()),
                    None,
                )
            });
            Ok(ExternalAuthorizationSession {
                session_id: session.session_id,
                login_url,
                status: session.status,
                expires_at: session
                    .expires_at
                    .as_deref()
                    .and_then(parse_remote_timestamp),
            })
        })
    }

    fn poll_authorization<'a>(
        &'a self,
        server_base_url: &'a str,
        session_id: &'a str,
    ) -> ProviderFuture<'a, ExternalAuthorizationPoll> {
        Box::pin(async move {
            let client = LoginClient::new(server_base_url)
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let session = client.get_session(session_id).await.map_err(remote_error)?;
            let expires_at = session
                .expires_at
                .as_deref()
                .and_then(parse_remote_timestamp);
            if session.is_completed() {
                let credentials = ProviderCredentials {
                    token: required_remote_value(session.token, "token")?,
                    refresh_token: required_remote_value(session.refresh_token, "refreshToken")?,
                    client_id: required_remote_value(session.client_id, "clientId")?,
                    public_key: required_remote_value(session.public_key, "publicKey")?,
                    server_base_url: server_base_url.to_owned(),
                    authorization_expires_at: session
                        .expires_in
                        .map(|seconds| now_millis().saturating_add(seconds.saturating_mul(1000))),
                };
                return Ok(ExternalAuthorizationPoll::Completed(credentials));
            }
            if session.is_finished() {
                return Ok(ExternalAuthorizationPoll::Ended {
                    status: session.status,
                });
            }
            Ok(ExternalAuthorizationPoll::Pending {
                status: session.status,
                expires_at,
            })
        })
    }

    fn refresh_credentials<'a>(
        &'a self,
        server_base_url: &'a str,
        refresh_token: &'a str,
    ) -> ProviderFuture<'a, ProviderCredentials> {
        Box::pin(async move {
            let client = LoginClient::new(server_base_url)
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let refreshed = client
                .refresh_token(refresh_token)
                .await
                .map_err(remote_error)?;
            Ok(ProviderCredentials {
                token: refreshed.token,
                refresh_token: refreshed
                    .refresh_token
                    .unwrap_or_else(|| refresh_token.to_owned()),
                client_id: String::new(),
                public_key: String::new(),
                server_base_url: server_base_url.to_owned(),
                authorization_expires_at: refreshed
                    .expires_in
                    .map(|seconds| now_millis().saturating_add(seconds.saturating_mul(1000))),
            })
        })
    }

    fn list_chat_models<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, Vec<ExternalCatalogModel>> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let context = credentials.context();
            let mut distinct = BTreeMap::<String, ExternalCatalogModel>::new();
            for quality in [QualityType::High, QualityType::Middle, QualityType::Low] {
                for model in client
                    .list_models(&context, Some(quality))
                    .await
                    .map_err(remote_error)?
                {
                    distinct.entry(model.model_code.clone()).or_insert_with(|| {
                        ExternalCatalogModel {
                            model_code: model.model_code.clone(),
                            model_name: model.model_name.unwrap_or(model.model_code),
                            protocols: model
                                .protocol_types
                                .unwrap_or_default()
                                .into_iter()
                                .map(agent_protocol_from_sdk)
                                .collect(),
                            upstream_vendor_code: model.provider_code,
                            upstream_vendor_name: model.provider_name,
                            context_window_tokens: model
                                .context_length
                                .and_then(|value| u32::try_from(value).ok()),
                            quality_type: provider_quality_from_sdk(quality),
                            input_coefficient: model.input_coefficient,
                            cache_hit_coefficient: model.cache_hit_coefficient,
                            output_coefficient: model.output_coefficient,
                        }
                    });
                }
            }
            Ok(distinct.into_values().collect())
        })
    }

    fn list_multimedia_models<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, Vec<ExternalMultimediaCatalogModel>> {
        Box::pin(async move {
            let client = MultimediaClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let context = credentials.context();
            let mut distinct = BTreeMap::<String, ExternalMultimediaCatalogModel>::new();
            for model in client
                .list_models(&context, None)
                .await
                .map_err(remote_error)?
            {
                let Some(catalog_model_type) =
                    ProviderCatalogModelType::from_person_ai_media_type(model.media_type)
                else {
                    continue;
                };
                // Embeddings are a Person-AI managed capability, not a
                // selectable remote model. Keep them out of the provider
                // catalogue even if a future upstream list includes type 5.
                if catalog_model_type == ProviderCatalogModelType::Embedding {
                    continue;
                }
                distinct.entry(model.model_id.clone()).or_insert_with(|| {
                    ExternalMultimediaCatalogModel {
                        model_id: model.model_id,
                        model_name: model.model_name.unwrap_or_else(|| model.model_code.clone()),
                        model_code: model.model_code,
                        catalog_model_type,
                    }
                });
            }
            Ok(distinct.into_values().collect())
        })
    }

    fn verify_chat_auto<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, ()> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let response = client
                .create(
                    &credentials.context(),
                    &json!({
                        "messages": [{
                            "role": "user",
                            "content": "Reply with OK."
                        }],
                        "max_tokens": 8,
                    })
                    .to_string(),
                    Some(QualityType::High),
                    None,
                )
                .await
                .map_err(remote_error)?;
            if response.get("error").is_some() {
                return Err(ModelProviderError::Remote(
                    "Person-AI high quality Auto chat verification failed".into(),
                ));
            }
            Ok(())
        })
    }

    fn verify_embedding<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
    ) -> ProviderFuture<'a, ()> {
        Box::pin(async move {
            let client = EmbeddingsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            let response = client
                .create(
                    &credentials.context(),
                    &json!({ "input": "Tron Person-AI embedding connectivity check" }).to_string(),
                )
                .await
                .map_err(remote_error)?;
            let valid = response
                .get("data")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|item| item.get("embedding"))
                .and_then(Value::as_array)
                .is_some_and(|embedding| !embedding.is_empty());
            if valid {
                // Person-AI owns the active embedding model and routes this
                // request internally. This only verifies the required
                // capability; it must never synthesize a catalog model.
                Ok(())
            } else {
                Err(ModelProviderError::Remote(
                    "Person-AI embedding verification returned no vector".into(),
                ))
            }
        })
    }

    fn chat<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
        quality_type: ProviderModelQuality,
        conversation_id: Option<&'a str>,
    ) -> ProviderFuture<'a, Value> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            client
                .create(
                    &credentials.context(),
                    &body.to_string(),
                    Some(provider_quality_to_sdk(quality_type)),
                    conversation_id,
                )
                .await
                .map_err(person_ai_runtime_error)
        })
    }

    fn chat_stream<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
        quality_type: ProviderModelQuality,
        conversation_id: Option<&'a str>,
        sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
    ) -> ProviderFuture<'a, ()> {
        Box::pin(async move {
            let client = ChatCompletionsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            client
                .create_stream(
                    &credentials.context(),
                    &body.to_string(),
                    provider_quality_to_sdk(quality_type),
                    conversation_id,
                    |data| {
                        if data.trim().is_empty() || data.trim() == "[DONE]" {
                            return Ok(());
                        }
                        let event = serde_json::from_str(data).map_err(|error| {
                            crab_serverapi::ServerApiError::Validation(format!(
                                "Person-AI stream returned invalid JSON: {error}"
                            ))
                        })?;
                        sink(event).map_err(crab_serverapi::ServerApiError::Validation)
                    },
                )
                .await
                .map_err(person_ai_runtime_error)
        })
    }

    fn embed<'a>(
        &'a self,
        credentials: &'a ProviderCredentials,
        body: &'a Value,
    ) -> ProviderFuture<'a, Value> {
        Box::pin(async move {
            let client = EmbeddingsClient::new(credentials.server_base_url.clone())
                .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
            client
                .create(&credentials.context(), &body.to_string())
                .await
                .map_err(remote_error)
        })
    }
}

#[derive(Clone)]
pub struct ModelProviderService {
    connection: Arc<Mutex<Connection>>,
    token_refresh_lock: Arc<tokio::sync::Mutex<()>>,
    cipher: Arc<dyn VariableCipher>,
    adapters: Arc<HashMap<ModelProviderCode, Arc<dyn ModelProviderAdapter>>>,
}

impl ModelProviderService {
    pub fn connect(
        database_url: &str,
        cipher: Arc<dyn VariableCipher>,
    ) -> Result<Self, ModelProviderError> {
        let connection = open_connection(database_url)?;
        connection
            .execute_batch(MODEL_PROVIDER_MIGRATION)
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute_batch(include_str!(
                "../../migrations/20260917_0001_create_chat_auto_model_routes.sql"
            ))
            .map_err(|_| ModelProviderError::Storage)?;
        migrate_provider_catalog_model_schema(&connection)
            .map_err(|_| ModelProviderError::Storage)?;
        remove_legacy_person_ai_embedding_catalog_models(&connection)?;
        trim_provider_sync_runs(&connection, ModelProviderCode::PersonAi)?;
        migrate_legacy_default_model_selections(&connection)?;
        if sqlite_table_exists(&connection, "default_model_selections")? {
            connection
                .execute(
                    "DELETE FROM default_model_selections WHERE provider_code = 'PersonAi' AND route_code = 'ChatAuto'",
                    [],
                )
                .map_err(|_| ModelProviderError::Storage)?;
        }
        let adapter: Arc<dyn ModelProviderAdapter> = Arc::new(PersonAiAdapter);
        let mut adapters = HashMap::new();
        adapters.insert(adapter.code(), adapter);
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            token_refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
            cipher,
            adapters: Arc::new(adapters),
        })
    }

    async fn fetch_catalog_snapshot(
        &self,
        provider_code: ModelProviderCode,
        credentials: &ProviderCredentials,
    ) -> Result<
        (
            Vec<ExternalCatalogModel>,
            Vec<ExternalMultimediaCatalogModel>,
        ),
        ModelProviderError,
    > {
        let adapter = self.adapter(provider_code)?;
        let chat_models = adapter.list_chat_models(credentials).await?;
        let multimedia_models = adapter.list_multimedia_models(credentials).await?;
        Ok((chat_models, multimedia_models))
    }

    pub fn list_connections(&self) -> Result<Vec<ProviderConnectionView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let existing = read_connection(&connection, ModelProviderCode::PersonAi)?;
        Ok(vec![connection_view(
            existing,
            catalog_model_count(&connection, ModelProviderCode::PersonAi)?,
        )])
    }

    pub fn set_person_ai_identity(
        &self,
        person_ai_user_id: &str,
        person_ai_username: &str,
    ) -> Result<(), ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "UPDATE provider_connections
                 SET person_ai_user_id = ?1, person_ai_username = ?2, updated_at = ?3
                 WHERE provider_code = 'PersonAi'",
                params![
                    person_ai_user_id.trim(),
                    person_ai_username.trim(),
                    now_millis()
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    pub fn get_connection(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderConnectionView, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let row = read_connection(&connection, provider_code)?;
        Ok(connection_view(
            row,
            catalog_model_count(&connection, provider_code)?,
        ))
    }

    pub async fn get_account_balance(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderAccountBalanceView, ModelProviderError> {
        let credentials = self.active_credentials(provider_code).await?;
        match provider_code {
            ModelProviderCode::PersonAi => {
                let client = AuthorizedClientApi::new(credentials.server_base_url.clone())
                    .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
                let context = credentials.context();
                let request = client
                    .new_gateway_request(Method::GET, "/account/info", &context)
                    .map_err(remote_error)?;
                let response = client
                    .execute_for_raw_json(request, Some(&context), "获取账户信息")
                    .await
                    .map_err(|error| {
                        eprintln!(
                            "[person-ai] account profile request failed: {}",
                            account_info_error_kind(&error)
                        );
                        account_info_error(error)
                    })?;
                parse_person_ai_account_info(response)
            }
        }
    }

    pub fn list_catalog_models(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        read_catalog_models(&connection, provider_code)
    }

    pub fn list_sync_runs(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<Vec<ProviderSyncRunView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        read_sync_runs(&connection, provider_code)
    }

    pub fn get_run_snapshot(
        &self,
        run_code: &str,
    ) -> Result<Option<ProviderModelRunSnapshotView>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        read_run_snapshot(&connection, run_code)
    }

    pub fn get_authorization_session(
        &self,
        session_id: &str,
    ) -> Result<ProviderAuthorizationSessionView, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(read_authorization_session(&connection, session_id)?.to_view())
    }

    pub async fn create_authorization_session(
        &self,
        command: CreateProviderAuthorizationCommand,
    ) -> Result<ProviderAuthorizationSessionView, ModelProviderError> {
        let (adapter, existing_client_id, server_base_url) = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            let row = read_connection(&connection, command.provider_code)?;
            let server_base_url = match command
                .server_base_url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                Some(value) => normalize_server_base_url(Some(value))?,
                None => row
                    .as_ref()
                    .map(|row| row.server_base_url.clone())
                    .unwrap_or_else(|| PERSON_AI_DEFAULT_SERVER_BASE_URL.into()),
            };
            let existing_client_id = row
                .as_ref()
                .and_then(|row| row.credentials_encrypted.as_deref())
                .and_then(|encrypted| self.decrypt_credentials(encrypted).ok())
                .map(|credentials| credentials.client_id);
            (
                self.adapter(command.provider_code)?,
                existing_client_id,
                server_base_url,
            )
        };
        let remote = adapter
            .create_authorization(
                &server_base_url,
                PERSON_AI_CLIENT_NAME,
                existing_client_id.as_deref(),
            )
            .await?;
        let now = now_millis();
        let view = ProviderAuthorizationSessionView {
            provider_code: command.provider_code,
            session_id: remote.session_id.clone(),
            login_url: remote.login_url.clone(),
            status: remote.status.clone(),
            expires_at: remote.expires_at,
            created_at: now,
            updated_at: now,
        };
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO provider_connections (
                    provider_code, server_base_url, status, credentials_encrypted, authorized_at,
                    authorization_expires_at, last_sync_at, last_error, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, NULL, NULL, NULL, NULL, NULL, ?4, ?4)
                 ON CONFLICT(provider_code) DO UPDATE SET
                    server_base_url = CASE
                        WHEN provider_connections.credentials_encrypted IS NULL THEN excluded.server_base_url
                        ELSE provider_connections.server_base_url
                    END,
                    status = CASE
                        WHEN provider_connections.credentials_encrypted IS NULL THEN excluded.status
                        ELSE provider_connections.status
                    END,
                    updated_at = excluded.updated_at",
                params![
                    command.provider_code.storage_value(),
                    server_base_url,
                    ProviderConnectionStatus::PendingAuthorization.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO provider_authorization_sessions (
                    session_id, provider_code, server_base_url, client_name, login_url, status,
                    expires_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                 ON CONFLICT(session_id) DO UPDATE SET status = excluded.status, updated_at = excluded.updated_at",
                params![
                    view.session_id,
                    command.provider_code.storage_value(),
                    server_base_url,
                    PERSON_AI_CLIENT_NAME,
                    view.login_url,
                    view.status,
                    view.expires_at,
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(view)
    }

    pub async fn poll_authorization_session(
        &self,
        command: CompleteProviderAuthorizationCommand,
    ) -> Result<CompleteProviderAuthorizationResult, ModelProviderError> {
        let stored = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            read_authorization_session(&connection, &command.session_id)?
        };
        let adapter = self.adapter(stored.provider_code)?;
        match adapter
            .poll_authorization(&stored.server_base_url, &stored.session_id)
            .await?
        {
            ExternalAuthorizationPoll::Pending { status, expires_at } => {
                let now = now_millis();
                let connection = self
                    .connection
                    .lock()
                    .map_err(|_| ModelProviderError::Storage)?;
                connection
                    .execute(
                        "UPDATE provider_authorization_sessions SET status = ?2, expires_at = ?3, updated_at = ?4 WHERE session_id = ?1",
                        params![stored.session_id, status, expires_at, now],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                Err(ModelProviderError::AuthorizationPending)
            }
            ExternalAuthorizationPoll::Ended { status } => {
                let now = now_millis();
                let connection = self
                    .connection
                    .lock()
                    .map_err(|_| ModelProviderError::Storage)?;
                connection
                    .execute(
                        "UPDATE provider_authorization_sessions SET status = ?2, updated_at = ?3 WHERE session_id = ?1",
                        params![stored.session_id, status, now],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                Err(ModelProviderError::AuthorizationEnded(status))
            }
            ExternalAuthorizationPoll::Completed(credentials) => {
                let (catalog, multimedia_catalog) = self
                    .fetch_catalog_snapshot(stored.provider_code, &credentials)
                    .await?;
                if catalog.is_empty() {
                    return Err(ModelProviderError::Remote(
                        "Person-AI returned no selectable chat models".into(),
                    ));
                }
                adapter.verify_chat_auto(&credentials).await?;
                adapter.verify_embedding(&credentials).await?;
                let (person_ai_user_id, person_ai_username) =
                    self.fetch_person_ai_identity(&credentials).await?;
                let supplied_encryption_key = command
                    .encryption_key_base64
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned);
                let setup_cipher = supplied_encryption_key
                    .as_ref()
                    .map(|key| {
                        Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
                            key_base64: key.clone(),
                        })
                    })
                    .transpose()
                    .map_err(|_| ModelProviderError::InvalidEncryptionKey)?;
                let encrypted = match setup_cipher.as_ref() {
                    Some(cipher) => encrypt_credentials(cipher, &credentials)?,
                    None => self.encrypt_credentials(&credentials)?,
                };
                let now = now_millis();
                let run_code = sync_run_code(now);
                let mut connection = self
                    .connection
                    .lock()
                    .map_err(|_| ModelProviderError::Storage)?;
                let transaction = connection
                    .transaction()
                    .map_err(|_| ModelProviderError::Storage)?;
                replace_authorized_connection(
                    &transaction,
                    stored.provider_code,
                    &credentials,
                    &person_ai_user_id,
                    &person_ai_username,
                    &encrypted,
                    &stored.session_id,
                    &catalog,
                    &multimedia_catalog,
                    &run_code,
                    now,
                )?;
                transaction
                    .commit()
                    .map_err(|_| ModelProviderError::Storage)?;
                // Setup persists this same key in application.yml immediately
                // after the provider handshake. Configure the shared cipher
                // now as well, so the freshly authorized Person-AI connection
                // is usable before the process is restarted.
                if let Some(key) = supplied_encryption_key {
                    self.cipher
                        .configure_key_base64(&key)
                        .map_err(|_| ModelProviderError::InvalidEncryptionKey)?;
                }
                let session =
                    read_authorization_session(&connection, &stored.session_id)?.to_view();
                let row = read_connection(&connection, stored.provider_code)?;
                let count = catalog_model_count(&connection, stored.provider_code)?;
                Ok(CompleteProviderAuthorizationResult {
                    session,
                    connection: connection_view(row, count),
                    text_verified: true,
                    embedding_verified: true,
                })
            }
        }
    }

    async fn fetch_person_ai_identity(
        &self,
        credentials: &ProviderCredentials,
    ) -> Result<(String, String), ModelProviderError> {
        let login = LoginClient::new(credentials.server_base_url.clone())
            .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
        let token_status = login
            .get_token_status(&credentials.token)
            .await
            .map_err(remote_error)?;
        let person_ai_user_id = token_status
            .subject
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ModelProviderError::Remote(
                    "Person-AI token status did not contain a stable subject".into(),
                )
            })?;
        let account = AccountClient::new(credentials.server_base_url.clone())
            .map_err(|_| ModelProviderError::InvalidServerBaseUrl)?
            .get_info(&credentials.context())
            .await
            .map_err(remote_error)?;
        let person_ai_username = account
            .username
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ModelProviderError::Remote(
                    "Person-AI account info did not contain a username".into(),
                )
            })?;
        Ok((person_ai_user_id, person_ai_username))
    }

    pub async fn sync_now(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderSyncRunView, ModelProviderError> {
        let now = now_millis();
        let sync_run_code = sync_run_code(now);
        {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            let row = read_connection(&connection, provider_code)?
                .ok_or(ModelProviderError::ConnectionNotReady)?;
            if matches!(
                row.status,
                ProviderConnectionStatus::Disconnected
                    | ProviderConnectionStatus::PendingAuthorization
            ) {
                return Err(ModelProviderError::ConnectionNotReady);
            }
            connection
                .execute(
                    "INSERT INTO provider_sync_runs (
                        sync_run_code, provider_code, status, started_at, completed_at, model_count, error_summary
                     ) VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL)",
                    params![
                        sync_run_code,
                        provider_code.storage_value(),
                        ProviderSyncStatus::Running.storage_value(),
                        now,
                    ],
                )
                .map_err(|_| ModelProviderError::Storage)?;
            trim_provider_sync_runs(&connection, provider_code)?;
        }
        let credentials = match self.active_credentials(provider_code).await {
            Ok(credentials) => credentials,
            Err(error) => {
                self.finish_sync_failure(provider_code, &sync_run_code, &error)
                    .await?;
                return Err(error);
            }
        };
        let result = self
            .fetch_catalog_snapshot(provider_code, &credentials)
            .await;
        match result {
            Ok((catalog, multimedia_catalog)) => {
                let completed_at = now_millis();
                let connection = self
                    .connection
                    .lock()
                    .map_err(|_| ModelProviderError::Storage)?;
                let transaction = connection
                    .unchecked_transaction()
                    .map_err(|_| ModelProviderError::Storage)?;
                apply_catalog_snapshot(&transaction, provider_code, &catalog, completed_at)?;
                apply_multimedia_catalog_snapshot(
                    &transaction,
                    provider_code,
                    &multimedia_catalog,
                    completed_at,
                )?;
                transaction
                    .execute(
                        "UPDATE provider_connections SET status = ?2, last_sync_at = ?3, last_error = NULL, updated_at = ?3 WHERE provider_code = ?1",
                        params![
                            provider_code.storage_value(),
                            ProviderConnectionStatus::Connected.storage_value(),
                            completed_at,
                        ],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                transaction
                    .execute(
                        "UPDATE provider_sync_runs SET status = ?2, completed_at = ?3, model_count = ?4 WHERE sync_run_code = ?1",
                        params![
                            sync_run_code,
                            ProviderSyncStatus::Succeeded.storage_value(),
                            completed_at,
                            i64::try_from(catalog.len().saturating_add(multimedia_catalog.len()))
                                .unwrap_or(i64::MAX),
                        ],
                    )
                    .map_err(|_| ModelProviderError::Storage)?;
                transaction
                    .commit()
                    .map_err(|_| ModelProviderError::Storage)?;
                Ok(ProviderSyncRunView {
                    sync_run_code,
                    provider_code,
                    status: ProviderSyncStatus::Succeeded,
                    started_at: now,
                    completed_at: Some(completed_at),
                    model_count: Some(catalog.len().saturating_add(multimedia_catalog.len())),
                    error_summary: None,
                })
            }
            Err(error) => {
                self.finish_sync_failure(provider_code, &sync_run_code, &error)
                    .await?;
                Err(error)
            }
        }
    }

    pub async fn refresh_due_connections(&self) {
        let should_refresh = self
            .get_connection(ModelProviderCode::PersonAi)
            .ok()
            .is_some_and(|connection| {
                matches!(
                    connection.status,
                    ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
                ) && connection
                    .authorization_expires_at
                    .is_some_and(|expires_at| {
                        expires_at <= now_millis().saturating_add(TOKEN_REFRESH_LEAD_MILLIS)
                    })
            });
        if should_refresh {
            let _ = self.active_credentials(ModelProviderCode::PersonAi).await;
        }
    }

    pub async fn sync_all_due_connections(&self) {
        self.refresh_due_connections().await;
        let connection = self.get_connection(ModelProviderCode::PersonAi);
        if connection.is_ok_and(|view| {
            matches!(
                view.status,
                ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
            )
        }) {
            let _ = self.sync_now(ModelProviderCode::PersonAi).await;
        }
    }

    pub fn start_background_sync(&self) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let service = self.clone();
        handle.spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
            interval.tick().await;
            loop {
                interval.tick().await;
                service.sync_all_due_connections().await;
            }
        });
    }

    pub fn disconnect(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderConnectionView, ModelProviderError> {
        let now = now_millis();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let transaction = connection
            .transaction()
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_connections SET status = ?2, credentials_encrypted = NULL,
                 authorization_expires_at = NULL, last_error = NULL, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderConnectionStatus::Disconnected.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_catalog_models SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderCatalogModelStatus::Unavailable.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "UPDATE provider_multimedia_catalog_models
                 SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderCatalogModelStatus::Unavailable.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .execute(
                "DELETE FROM default_model_selections WHERE provider_code = ?1",
                params![provider_code.storage_value()],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        transaction
            .commit()
            .map_err(|_| ModelProviderError::Storage)?;
        let row = read_connection(&connection, provider_code)?;
        Ok(connection_view(
            row,
            catalog_model_count(&connection, provider_code)?,
        ))
    }

    pub fn list_text_select_options(
        &self,
        workspace_code: &str,
    ) -> Result<Vec<ProviderModelSelectOption>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let default_selection =
            effective_default_selection(&connection, workspace_code, ModelType::Text)?;
        let Some(provider) = read_connection(&connection, ModelProviderCode::PersonAi)? else {
            return Ok(Vec::new());
        };
        let active = matches!(
            provider.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        );
        let mut options = Vec::new();
        if active {
            let selection = ModelSelection::person_ai_route(ProviderRouteCode::ChatAuto);
            options.push(ProviderModelSelectOption {
                is_default: default_selection.as_ref() == Some(&selection),
                selection,
                option_code: "provider:PersonAi:route:ChatAuto".into(),
                display_name: "Auto".into(),
                provider_code: ModelProviderCode::PersonAi,
                access_platform: ModelProviderCode::PersonAi.display_name().into(),
                upstream_vendor_name: None,
                model_type: ModelType::Text,
                protocols: vec![AgentModelProtocol::ChatCompletions],
                enabled: true,
                remote_status: ProviderCatalogModelStatus::Available,
                context_window_tokens: None,
                quality_type: Some(ProviderModelQuality::High),
                input_coefficient: None,
                cache_hit_coefficient: None,
                output_coefficient: None,
            });
        }
        for model in read_chat_catalog_models(&connection, ModelProviderCode::PersonAi)? {
            let enabled = active && model.status == ProviderCatalogModelStatus::Available;
            let selection = ModelSelection::person_ai_fixed(
                model.model_code.clone(),
                model
                    .quality_type
                    .expect("chat catalog models always have a quality"),
            );
            options.push(ProviderModelSelectOption {
                is_default: default_selection.as_ref() == Some(&selection),
                selection,
                option_code: format!(
                    "provider:PersonAi:model:{}:{}",
                    model
                        .quality_type
                        .expect("chat catalog models always have a quality")
                        .storage_value(),
                    model.model_code,
                ),
                display_name: model.model_name.clone(),
                provider_code: ModelProviderCode::PersonAi,
                access_platform: ModelProviderCode::PersonAi.display_name().into(),
                upstream_vendor_name: model.upstream_vendor_name.clone(),
                model_type: ModelType::Text,
                protocols: model.protocols.clone(),
                enabled,
                remote_status: model.status,
                context_window_tokens: model.context_window_tokens,
                quality_type: model.quality_type,
                input_coefficient: model.input_coefficient,
                cache_hit_coefficient: model.cache_hit_coefficient,
                output_coefficient: model.output_coefficient,
            });
        }
        Ok(options)
    }

    pub fn resolve_chat_selection(
        &self,
        workspace_code: &str,
        selection: Option<&ModelSelection>,
        legacy_model_code: Option<&str>,
        reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    ) -> Result<Option<AgentModelConfig>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let selection = match selection {
            Some(selection) => Some(selection.clone()),
            None => legacy_model_code
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .and_then(|model_code| {
                    read_catalog_model(&connection, ModelProviderCode::PersonAi, model_code, None)
                        .ok()
                        .flatten()
                        .map(|model| {
                            ModelSelection::person_ai_fixed(
                                model_code,
                                model
                                    .quality_type
                                    .expect("chat catalog models always have a quality"),
                            )
                        })
                })
                .or_else(|| {
                    effective_default_selection(&connection, workspace_code, ModelType::Text)
                        .ok()
                        .flatten()
                }),
        };
        let Some(selection) = selection else {
            return Ok(None);
        };
        if selection.provider_code != Some(ModelProviderCode::PersonAi) {
            return Ok(None);
        }
        let provider = read_connection(&connection, ModelProviderCode::PersonAi)?
            .ok_or(ModelProviderError::ConnectionNotReady)?;
        if !matches!(
            provider.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        ) {
            return Err(
                if provider.status == ProviderConnectionStatus::AuthRequired {
                    ModelProviderError::AuthenticationRequired
                } else {
                    ModelProviderError::ConnectionNotReady
                },
            );
        }
        match selection.kind {
            ModelSelectionKind::Auto => Err(ModelProviderError::InvalidSelection(
                "Tron Auto must be resolved by the chat session router".into(),
            )),
            ModelSelectionKind::ProviderRoute => {
                if selection.route_code != Some(ProviderRouteCode::ChatAuto)
                    || selection.model_code.is_some()
                    || selection.quality_type.is_some()
                {
                    return Err(ModelProviderError::InvalidSelection(
                        "Person-AI chat route must be ChatAuto without modelCode".into(),
                    ));
                }
                Ok(Some(person_ai_agent_config(
                    "PERSON_AI:ROUTE:ChatAuto",
                    "",
                    None,
                    reasoning_effort,
                    AgentModelProtocol::ChatCompletions,
                )))
            }
            ModelSelectionKind::Fixed => {
                let model_code = selection
                    .model_code
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ModelProviderError::InvalidSelection(
                            "Person-AI Fixed selection requires modelCode".into(),
                        )
                    })?;
                let model = read_catalog_model(
                    &connection,
                    ModelProviderCode::PersonAi,
                    model_code,
                    selection.quality_type,
                )?
                .ok_or_else(|| ModelProviderError::CatalogModelUnavailable(model_code.into()))?;
                if model.status != ProviderCatalogModelStatus::Available {
                    return Err(ModelProviderError::CatalogModelUnavailable(
                        model_code.into(),
                    ));
                }
                Ok(Some(person_ai_agent_config(
                    person_ai_fixed_runtime_code(
                        model
                            .quality_type
                            .expect("chat catalog models always have a quality"),
                        model_code,
                    ),
                    model_code,
                    model.context_window_tokens,
                    reasoning_effort,
                    preferred_protocol(&model.protocols),
                )))
            }
        }
    }

    pub fn text_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelProviderError> {
        let selection = match model_code.map(str::trim).filter(|value| !value.is_empty()) {
            Some(model_code) => self.person_ai_selection_from_legacy_code(model_code)?,
            None => None,
        };
        let selection = selection.or_else(|| {
            self.default_selection(workspace_code, ModelType::Text)
                .ok()
                .flatten()
        });
        let Some(selection) = selection else {
            return Ok(None);
        };
        let config = self.resolve_chat_selection(workspace_code, Some(&selection), None, None)?;
        Ok(config.map(runtime_config_from_agent))
    }

    pub fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelProviderError> {
        if model_code.is_some_and(|value| !value.trim().is_empty()) {
            return Ok(None);
        }
        let Some(selection) = self.default_selection(workspace_code, ModelType::Embedding)? else {
            return Ok(None);
        };
        if selection.provider_code != Some(ModelProviderCode::PersonAi)
            || selection.kind != ModelSelectionKind::ProviderRoute
            || selection.route_code != Some(ProviderRouteCode::EmbeddingAuto)
        {
            return Ok(None);
        }
        self.require_active_connection(ModelProviderCode::PersonAi)?;
        Ok(Some(RuntimeModelConfig {
            model_code: "PERSON_AI:ROUTE:EmbeddingAuto".into(),
            protocol: ModelProtocol::OpenAiCompatibleEmbedding,
            model_type: ModelType::Embedding,
            base_url: "https://person-ai.invalid".into(),
            api_path: Some("embeddings".into()),
            api_key: String::new(),
            model_name: String::new(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
            max_retries: Some(0),
        }))
    }

    pub fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelProviderError> {
        let explicit_llm_fallback = model_code
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value == "PERSON_AI:ROUTE:LlmFallback");
        if model_code.is_some_and(|value| !value.trim().is_empty()) && !explicit_llm_fallback {
            return Ok(None);
        }
        if !explicit_llm_fallback {
            let Some(selection) = self.default_selection(workspace_code, ModelType::Rerank)? else {
                return Ok(None);
            };
            if selection.provider_code != Some(ModelProviderCode::PersonAi)
                || selection.kind != ModelSelectionKind::ProviderRoute
                || selection.route_code != Some(ProviderRouteCode::LlmFallback)
            {
                return Ok(None);
            }
        }
        self.require_active_connection(ModelProviderCode::PersonAi)?;
        let Some(text_selection) = self.default_selection(workspace_code, ModelType::Text)? else {
            return Ok(None);
        };
        let Some(text_config) =
            self.resolve_chat_selection(workspace_code, Some(&text_selection), None, None)?
        else {
            return Ok(None);
        };
        let mut config = runtime_config_from_agent(text_config);
        // This is a RERANK runtime contract, but the LLM invocation must use
        // the workspace's effective TEXT selection (Auto or a fixed remote
        // Person-AI model), not a pretend native rerank endpoint.
        config.protocol = ModelProtocol::Rerank;
        config.model_type = ModelType::Rerank;
        config.max_tokens = Some(512);
        config.temperature = Some(0.0);
        Ok(Some(config))
    }

    pub fn is_person_ai_config(&self, model_code: &str) -> bool {
        model_code.starts_with(PROVIDER_RUNTIME_PREFIX)
    }

    pub async fn chat_completion(
        &self,
        config: &AgentModelConfig,
        body: &Value,
        conversation_id: Option<&str>,
        run_code: Option<&str>,
    ) -> Result<Value, String> {
        let credentials = self
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .map_err(|error| error.to_string())?;
        let quality_type = person_ai_runtime_quality(config)
            .ok_or_else(|| "Person-AI runtime model has no quality type".to_string())?;
        if config.protocol == AgentModelProtocol::Responses {
            let client = ResponsesClient::new(credentials.server_base_url.clone())
                .map_err(|e| e.to_string())?;
            return client
                .create(&credentials.context(), &body.to_string())
                .await
                .map_err(|e| e.to_string());
        }
        let adapter = self
            .adapter(ModelProviderCode::PersonAi)
            .map_err(|error| error.to_string())?;
        let result = adapter
            .chat(&credentials, body, quality_type, conversation_id)
            .await;
        let result = if matches!(&result, Err(ModelProviderError::Unauthorized)) {
            let refreshed = self
                .refresh_credentials_after_unauthorized(
                    ModelProviderCode::PersonAi,
                    &credentials.token,
                )
                .await
                .map_err(|error| error.to_string())?;
            adapter
                .chat(&refreshed, body, quality_type, conversation_id)
                .await
        } else {
            result
        };
        let response = result.map_err(|error| {
            self.mark_auth_required_after_rejected_credentials(&error);
            error.to_string()
        })?;
        self.record_runtime_snapshot(run_code, "", config, &response)
            .map_err(|error| error.to_string())?;
        Ok(response)
    }

    pub async fn chat_completion_stream(
        &self,
        config: &AgentModelConfig,
        body: &Value,
        conversation_id: Option<&str>,
        run_code: Option<&str>,
        sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + '_>,
    ) -> Result<(), String> {
        let credentials = self
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .map_err(|error| error.to_string())?;
        let quality_type = person_ai_runtime_quality(config)
            .ok_or_else(|| "Person-AI runtime model has no quality type".to_string())?;
        if config.protocol == AgentModelProtocol::Responses {
            let client = ResponsesClient::new(credentials.server_base_url.clone())
                .map_err(|e| e.to_string())?;
            let response = client
                .create_stream(&credentials.context(), &body.to_string(), |data| {
                    if data.trim().is_empty() || data.trim() == "[DONE]" {
                        return Ok(());
                    }
                    let event = serde_json::from_str(data)
                        .map_err(|e| crab_serverapi::ServerApiError::Validation(e.to_string()))?;
                    sink(event).map_err(crab_serverapi::ServerApiError::Validation)
                })
                .await
                .map_err(|e| e.to_string());
            return response;
        }
        let snapshot_service = self.clone();
        let snapshot_config = config.clone();
        let snapshot_run_code = run_code.map(ToOwned::to_owned);
        let snapshot_sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + '_> =
            Arc::new(move |event: Value| {
                snapshot_service
                    .record_runtime_snapshot(
                        snapshot_run_code.as_deref(),
                        "",
                        &snapshot_config,
                        &event,
                    )
                    .map_err(|error| error.to_string())?;
                sink(event)
            });
        let adapter = self
            .adapter(ModelProviderCode::PersonAi)
            .map_err(|error| error.to_string())?;
        let result = adapter
            .chat_stream(
                &credentials,
                body,
                quality_type,
                conversation_id,
                Arc::clone(&snapshot_sink),
            )
            .await;
        let result = if matches!(&result, Err(ModelProviderError::Unauthorized)) {
            let refreshed = self
                .refresh_credentials_after_unauthorized(
                    ModelProviderCode::PersonAi,
                    &credentials.token,
                )
                .await
                .map_err(|error| error.to_string())?;
            adapter
                .chat_stream(
                    &refreshed,
                    body,
                    quality_type,
                    conversation_id,
                    snapshot_sink,
                )
                .await
        } else {
            result
        };
        result.map_err(|error| {
            self.mark_auth_required_after_rejected_credentials(&error);
            error.to_string()
        })
    }

    pub async fn embedding(&self, body: &Value) -> Result<Value, String> {
        let credentials = self
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .map_err(|error| error.to_string())?;
        self.adapter(ModelProviderCode::PersonAi)
            .map_err(|error| error.to_string())?
            .embed(&credentials, body)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn llm_rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<(usize, f64)>, String> {
        self.llm_rerank_with_config(
            &person_ai_agent_config(
                "PERSON_AI:ROUTE:ChatAuto",
                "",
                None,
                None,
                AgentModelProtocol::ChatCompletions,
            ),
            query,
            documents,
            top_n,
        )
        .await
    }

    pub async fn llm_rerank_with_config(
        &self,
        config: &AgentModelConfig,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> Result<Vec<(usize, f64)>, String> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }
        let numbered = documents
            .iter()
            .enumerate()
            .map(|(index, document)| format!("[{index}] {document}"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let body = json!({
            "messages": [
                {
                    "role": "system",
                    "content": "Rank the candidate documents by relevance. Return only a JSON array of zero-based candidate indexes, with no markdown or explanation."
                },
                {
                    "role": "user",
                    "content": format!("Query:\n{query}\n\nCandidates:\n{numbered}")
                }
            ],
            "temperature": 0
        });
        let response = self.chat_completion(config, &body, None, None).await?;
        let content = response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| "Person-AI LLM rerank returned no text".to_string())?;
        let indexes = serde_json::from_str::<Vec<usize>>(content)
            .map_err(|_| "Person-AI LLM rerank returned invalid JSON indexes".to_string())?;
        let mut seen = HashSet::new();
        let mut ranked = indexes
            .into_iter()
            .filter(|index| *index < documents.len() && seen.insert(*index))
            .collect::<Vec<_>>();
        if ranked.is_empty() {
            return Err("Person-AI LLM rerank returned no valid candidate indexes".into());
        }
        if let Some(limit) = top_n.filter(|limit| *limit > 0) {
            ranked.truncate(limit);
        }
        let total = ranked.len().max(1) as f64;
        Ok(ranked
            .into_iter()
            .enumerate()
            .map(|(rank, index)| (index, 1.0 - rank as f64 / total))
            .collect())
    }

    fn default_selection(
        &self,
        workspace_code: &str,
        model_type: ModelType,
    ) -> Result<Option<ModelSelection>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        effective_default_selection(&connection, workspace_code, model_type)
    }

    fn person_ai_selection_from_legacy_code(
        &self,
        model_code: &str,
    ) -> Result<Option<ModelSelection>, ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(
            read_catalog_model(&connection, ModelProviderCode::PersonAi, model_code, None)?.map(
                |model| {
                    ModelSelection::person_ai_fixed(
                        model_code,
                        model
                            .quality_type
                            .expect("chat catalog models always have a quality"),
                    )
                },
            ),
        )
    }

    fn require_active_connection(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<(), ModelProviderError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let row = read_connection(&connection, provider_code)?
            .ok_or(ModelProviderError::ConnectionNotReady)?;
        if matches!(
            row.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        ) {
            Ok(())
        } else if row.status == ProviderConnectionStatus::AuthRequired {
            Err(ModelProviderError::AuthenticationRequired)
        } else {
            Err(ModelProviderError::ConnectionNotReady)
        }
    }

    async fn active_credentials(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        let row = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            read_connection(&connection, provider_code)?
                .ok_or(ModelProviderError::ConnectionNotReady)?
        };
        if row.status == ProviderConnectionStatus::AuthRequired {
            return Err(ModelProviderError::AuthenticationRequired);
        }
        if !matches!(
            row.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        ) {
            return Err(ModelProviderError::ConnectionNotReady);
        }
        let encrypted = row
            .credentials_encrypted
            .as_deref()
            .ok_or(ModelProviderError::AuthenticationRequired)?;
        let credentials = match self.decrypt_credentials(encrypted) {
            Ok(credentials) => credentials,
            Err(_) => {
                self.mark_auth_required(provider_code, "credential decryption failed")?;
                return Err(ModelProviderError::AuthenticationRequired);
            }
        };
        if credentials
            .authorization_expires_at
            .is_some_and(|expires_at| {
                expires_at > now_millis().saturating_add(TOKEN_REFRESH_LEAD_MILLIS)
            })
        {
            return Ok(credentials);
        }
        self.refresh_credentials_for_request(provider_code, &credentials.token, false)
            .await
    }

    async fn refresh_credentials_after_unauthorized(
        &self,
        provider_code: ModelProviderCode,
        rejected_token: &str,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        self.refresh_credentials_for_request(provider_code, rejected_token, true)
            .await
    }

    async fn refresh_credentials_for_request(
        &self,
        provider_code: ModelProviderCode,
        expected_token: &str,
        force: bool,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        let _refresh_guard = self.token_refresh_lock.lock().await;
        let row = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            read_connection(&connection, provider_code)?
                .ok_or(ModelProviderError::ConnectionNotReady)?
        };
        if !matches!(
            row.status,
            ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded
        ) {
            return Err(if row.status == ProviderConnectionStatus::AuthRequired {
                ModelProviderError::AuthenticationRequired
            } else {
                ModelProviderError::ConnectionNotReady
            });
        }
        let encrypted = row
            .credentials_encrypted
            .as_deref()
            .ok_or(ModelProviderError::AuthenticationRequired)?;
        let mut credentials = self.decrypt_credentials(encrypted).map_err(|_| {
            let _ = self.mark_auth_required(provider_code, "credential decryption failed");
            ModelProviderError::AuthenticationRequired
        })?;

        // Another request may already have refreshed this token while we were
        // waiting for the lock. Reuse its persisted credentials rather than
        // rotating the same refresh token a second time.
        if credentials.token != expected_token {
            return Ok(credentials);
        }
        if !force
            && credentials
                .authorization_expires_at
                .is_some_and(|expires_at| {
                    expires_at > now_millis().saturating_add(TOKEN_REFRESH_LEAD_MILLIS)
                })
        {
            return Ok(credentials);
        }

        let refreshed = match self
            .adapter(provider_code)?
            .refresh_credentials(&credentials.server_base_url, &credentials.refresh_token)
            .await
        {
            Ok(refreshed) if !refreshed.token.trim().is_empty() => refreshed,
            Ok(_) | Err(_) => {
                self.mark_auth_required(provider_code, "token refresh failed")?;
                return Err(ModelProviderError::AuthenticationRequired);
            }
        };
        credentials.token = refreshed.token;
        credentials.refresh_token = refreshed.refresh_token;
        credentials.authorization_expires_at = refreshed.authorization_expires_at;
        self.persist_refreshed_credentials(provider_code, &credentials)?;
        Ok(credentials)
    }

    fn persist_refreshed_credentials(
        &self,
        provider_code: ModelProviderCode,
        credentials: &ProviderCredentials,
    ) -> Result<(), ModelProviderError> {
        let encrypted = self.encrypt_credentials(credentials)?;
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "UPDATE provider_connections SET credentials_encrypted = ?2, authorization_expires_at = ?3,
                 status = ?4, last_error = NULL, updated_at = ?5 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    encrypted,
                    credentials.authorization_expires_at,
                    ProviderConnectionStatus::Connected.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    fn mark_auth_required_after_rejected_credentials(&self, error: &ModelProviderError) {
        if matches!(error, ModelProviderError::Unauthorized)
            || is_person_ai_authorization_key_error(error)
        {
            let reason = if matches!(error, ModelProviderError::Unauthorized) {
                "Person-AI rejected the refreshed access token; reauthorization required"
            } else {
                "Person-AI rejected the authorization key; reauthorization required"
            };
            let _ = self.mark_auth_required(ModelProviderCode::PersonAi, reason);
        }
    }

    async fn finish_sync_failure(
        &self,
        provider_code: ModelProviderCode,
        sync_run_code: &str,
        error: &ModelProviderError,
    ) -> Result<(), ModelProviderError> {
        let now = now_millis();
        let summary = safe_error_summary(error);
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        let status = if matches!(error, ModelProviderError::AuthenticationRequired) {
            ProviderConnectionStatus::AuthRequired
        } else {
            ProviderConnectionStatus::Degraded
        };
        connection
            .execute(
                "UPDATE provider_connections SET status = ?2, last_error = ?3, updated_at = ?4 WHERE provider_code = ?1",
                params![provider_code.storage_value(), status.storage_value(), summary, now],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "UPDATE provider_sync_runs SET status = ?2, completed_at = ?3, error_summary = ?4 WHERE sync_run_code = ?1",
                params![sync_run_code, ProviderSyncStatus::Failed.storage_value(), now, summary],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    fn mark_auth_required(
        &self,
        provider_code: ModelProviderCode,
        message: &str,
    ) -> Result<(), ModelProviderError> {
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "UPDATE provider_connections SET status = ?2, last_error = ?3, updated_at = ?4 WHERE provider_code = ?1",
                params![
                    provider_code.storage_value(),
                    ProviderConnectionStatus::AuthRequired.storage_value(),
                    message,
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    fn adapter(
        &self,
        provider_code: ModelProviderCode,
    ) -> Result<Arc<dyn ModelProviderAdapter>, ModelProviderError> {
        self.adapters
            .get(&provider_code)
            .cloned()
            .ok_or(ModelProviderError::ConnectionNotReady)
    }

    fn encrypt_credentials(
        &self,
        credentials: &ProviderCredentials,
    ) -> Result<String, ModelProviderError> {
        encrypt_credentials(self.cipher.as_ref(), credentials)
    }

    fn decrypt_credentials(
        &self,
        encrypted: &str,
    ) -> Result<ProviderCredentials, ModelProviderError> {
        decrypt_credentials(self.cipher.as_ref(), encrypted)
    }

    pub fn record_runtime_snapshot(
        &self,
        run_code: Option<&str>,
        workspace_code: &str,
        config: &AgentModelConfig,
        response: &Value,
    ) -> Result<(), ModelProviderError> {
        let Some(run_code) = run_code.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };
        let (selection_kind, requested_model_code, route_code) =
            if config.model_code == "PERSON_AI:ROUTE:ChatAuto" {
                (
                    ModelSelectionKind::ProviderRoute,
                    None,
                    Some(ProviderRouteCode::ChatAuto),
                )
            } else {
                (
                    ModelSelectionKind::Fixed,
                    Some(config.model_name.clone()),
                    None,
                )
            };
        let quality_type = person_ai_runtime_quality(config);
        let actual_model_code = response
            .get("model")
            .or_else(|| response.get("model_code"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let actual_model_name = response
            .get("model_name")
            .or_else(|| response.pointer("/model/name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let actual_vendor_code = response
            .get("provider_code")
            .or_else(|| response.pointer("/provider/code"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let actual_vendor_name = response
            .get("provider_name")
            .or_else(|| response.pointer("/provider/name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO provider_model_run_snapshots (
                    run_code, session_code, workspace_code, selection_kind, provider_code,
                    requested_model_code, quality_type, route_code, resolved_model_code, resolved_model_name,
                    resolved_upstream_vendor_code, resolved_upstream_vendor_name, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
                 ON CONFLICT(run_code) DO UPDATE SET
                    resolved_model_code = COALESCE(excluded.resolved_model_code, provider_model_run_snapshots.resolved_model_code),
                    resolved_model_name = COALESCE(excluded.resolved_model_name, provider_model_run_snapshots.resolved_model_name),
                    resolved_upstream_vendor_code = COALESCE(excluded.resolved_upstream_vendor_code, provider_model_run_snapshots.resolved_upstream_vendor_code),
                    resolved_upstream_vendor_name = COALESCE(excluded.resolved_upstream_vendor_name, provider_model_run_snapshots.resolved_upstream_vendor_name),
                    updated_at = excluded.updated_at",
                params![
                    run_code,
                    Option::<String>::None,
                    workspace_code,
                    selection_kind.storage_value(),
                    ModelProviderCode::PersonAi.storage_value(),
                    requested_model_code,
                    quality_type.map(ProviderModelQuality::storage_value),
                    route_code.map(ProviderRouteCode::storage_value),
                    actual_model_code,
                    actual_model_name,
                    actual_vendor_code,
                    actual_vendor_name,
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }

    /// Persist the choice before invocation. Stream callbacks later enrich this
    /// row with the actual Person-AI response model, so catalog changes cannot
    /// erase the model lineage of a finished run.
    pub fn record_requested_runtime_snapshot(
        &self,
        run_code: Option<&str>,
        session_code: &str,
        workspace_code: &str,
        config: &AgentModelConfig,
    ) -> Result<(), ModelProviderError> {
        let Some(run_code) = run_code.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };
        if !self.is_person_ai_config(&config.model_code) {
            return Ok(());
        }
        let (selection_kind, requested_model_code, route_code) =
            if config.model_code == "PERSON_AI:ROUTE:ChatAuto" {
                (
                    ModelSelectionKind::ProviderRoute,
                    None,
                    Some(ProviderRouteCode::ChatAuto),
                )
            } else {
                (
                    ModelSelectionKind::Fixed,
                    Some(config.model_name.clone()),
                    None,
                )
            };
        let quality_type = person_ai_runtime_quality(config);
        let fixed_catalog_model = requested_model_code
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|model_code| {
                let connection = self
                    .connection
                    .lock()
                    .map_err(|_| ModelProviderError::Storage)?;
                read_catalog_model(
                    &connection,
                    ModelProviderCode::PersonAi,
                    model_code,
                    quality_type,
                )
            })
            .transpose()?
            .flatten();
        let now = now_millis();
        let connection = self
            .connection
            .lock()
            .map_err(|_| ModelProviderError::Storage)?;
        connection
            .execute(
                "INSERT INTO provider_model_run_snapshots (
                    run_code, session_code, workspace_code, selection_kind, provider_code,
                    requested_model_code, quality_type, route_code, resolved_model_code, resolved_model_name,
                    resolved_upstream_vendor_code, resolved_upstream_vendor_name, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
                 ON CONFLICT(run_code) DO UPDATE SET
                    session_code = excluded.session_code,
                    workspace_code = excluded.workspace_code,
                    selection_kind = excluded.selection_kind,
                    provider_code = excluded.provider_code,
                    requested_model_code = excluded.requested_model_code,
                    quality_type = excluded.quality_type,
                    route_code = excluded.route_code,
                    resolved_model_code = COALESCE(excluded.resolved_model_code, provider_model_run_snapshots.resolved_model_code),
                    resolved_model_name = COALESCE(excluded.resolved_model_name, provider_model_run_snapshots.resolved_model_name),
                    resolved_upstream_vendor_code = COALESCE(excluded.resolved_upstream_vendor_code, provider_model_run_snapshots.resolved_upstream_vendor_code),
                    resolved_upstream_vendor_name = COALESCE(excluded.resolved_upstream_vendor_name, provider_model_run_snapshots.resolved_upstream_vendor_name),
                    updated_at = excluded.updated_at",
                params![
                    run_code,
                    session_code,
                    workspace_code,
                    selection_kind.storage_value(),
                    ModelProviderCode::PersonAi.storage_value(),
                    requested_model_code,
                    quality_type.map(ProviderModelQuality::storage_value),
                    route_code.map(ProviderRouteCode::storage_value),
                    fixed_catalog_model.as_ref().map(|model| model.model_code.clone()),
                    fixed_catalog_model.as_ref().map(|model| model.model_name.clone()),
                    fixed_catalog_model.as_ref().and_then(|model| model.upstream_vendor_code.clone()),
                    fixed_catalog_model.as_ref().and_then(|model| model.upstream_vendor_name.clone()),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ProviderAwareLlmRuntimeResolver {
    models: ModelService,
    providers: ModelProviderService,
}

impl ProviderAwareLlmRuntimeResolver {
    pub fn new(models: ModelService, providers: ModelProviderService) -> Self {
        Self { models, providers }
    }
}

impl LlmRuntimeModelResolver for ProviderAwareLlmRuntimeResolver {
    fn embedding_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        match self
            .providers
            .embedding_runtime_model_config(workspace_code, model_code)
        {
            Ok(Some(config)) => Ok(Some(config)),
            Ok(None) => self
                .models
                .embedding_runtime_model_config(workspace_code, model_code),
            Err(_) => Err(ModelError::StorageFailure),
        }
    }

    fn rerank_runtime_model_config(
        &self,
        workspace_code: &str,
        model_code: Option<&str>,
    ) -> Result<Option<RuntimeModelConfig>, ModelError> {
        match self
            .providers
            .rerank_runtime_model_config(workspace_code, model_code)
        {
            Ok(Some(config)) => Ok(Some(config)),
            Ok(None) => self
                .models
                .rerank_runtime_model_config(workspace_code, model_code),
            Err(_) => Err(ModelError::StorageFailure),
        }
    }
}

fn person_ai_agent_config(
    runtime_model_code: impl Into<String>,
    model_name: impl Into<String>,
    context_window_tokens: Option<u32>,
    reasoning_effort: Option<crate::domain::reasoning::ReasoningEffort>,
    protocol: AgentModelProtocol,
) -> AgentModelConfig {
    AgentModelConfig {
        model_code: runtime_model_code.into(),
        protocol,
        base_url: "https://person-ai.invalid".into(),
        api_path: Some(
            if protocol == AgentModelProtocol::Responses {
                "responses"
            } else {
                "chat/completions"
            }
            .into(),
        ),
        api_key: String::new(),
        model_name: model_name.into(),
        thinking_enabled: reasoning_effort.is_some(),
        reasoning_effort,
        responses_stateful_enabled: false,
        temperature: None,
        max_tokens: None,
        context_window_tokens,
        top_p: None,
        max_retries: Some(0),
    }
}

fn preferred_protocol(protocols: &[AgentModelProtocol]) -> AgentModelProtocol {
    if protocols.contains(&AgentModelProtocol::Responses) {
        AgentModelProtocol::Responses
    } else {
        AgentModelProtocol::ChatCompletions
    }
}

fn person_ai_fixed_runtime_code(quality_type: ProviderModelQuality, model_code: &str) -> String {
    format!(
        "PERSON_AI:FIXED:{}:{model_code}",
        quality_type.storage_value(),
    )
}

fn person_ai_runtime_quality(config: &AgentModelConfig) -> Option<ProviderModelQuality> {
    if config.model_code == "PERSON_AI:ROUTE:ChatAuto" {
        return Some(ProviderModelQuality::High);
    }
    let encoded = config.model_code.strip_prefix("PERSON_AI:FIXED:")?;
    let (quality, _) = encoded.split_once(':')?;
    ProviderModelQuality::from_storage(quality)
}

fn runtime_config_from_agent(config: AgentModelConfig) -> RuntimeModelConfig {
    RuntimeModelConfig {
        model_code: config.model_code,
        protocol: match config.protocol {
            AgentModelProtocol::Responses => ModelProtocol::Responses,
            AgentModelProtocol::ChatCompletions => ModelProtocol::OpenAiCompatible,
            AgentModelProtocol::Embeddings => ModelProtocol::OpenAiCompatibleEmbedding,
        },
        model_type: ModelType::Text,
        base_url: config.base_url,
        api_path: config.api_path,
        api_key: config.api_key,
        model_name: config.model_name,
        thinking_enabled: config.thinking_enabled,
        reasoning_effort: config.reasoning_effort,
        responses_stateful_enabled: config.responses_stateful_enabled,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        context_window_tokens: config.context_window_tokens,
        top_p: config.top_p,
        max_retries: config.max_retries,
    }
}

#[derive(Debug, Clone)]
struct ProviderConnectionRow {
    provider_code: ModelProviderCode,
    person_ai_user_id: Option<String>,
    person_ai_username: Option<String>,
    server_base_url: String,
    status: ProviderConnectionStatus,
    credentials_encrypted: Option<String>,
    authorized_at: Option<i64>,
    authorization_expires_at: Option<i64>,
    last_sync_at: Option<i64>,
    last_error: Option<String>,
    updated_at: i64,
}

#[derive(Debug, Clone)]
struct ProviderAuthorizationSessionRow {
    session_id: String,
    provider_code: ModelProviderCode,
    server_base_url: String,
    login_url: String,
    status: String,
    expires_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

impl ProviderAuthorizationSessionRow {
    fn to_view(&self) -> ProviderAuthorizationSessionView {
        ProviderAuthorizationSessionView {
            provider_code: self.provider_code,
            session_id: self.session_id.clone(),
            login_url: self.login_url.clone(),
            status: self.status.clone(),
            expires_at: self.expires_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn open_connection(database_url: &str) -> Result<Connection, ModelProviderError> {
    if database_url != ":memory:"
        && !database_url.starts_with("file:")
        && let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| ModelProviderError::Storage)?;
    }
    if database_url == ":memory:" || database_url.starts_with("file:") {
        Connection::open_with_flags(
            database_url,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
    } else {
        Connection::open(database_url)
    }
    .map_err(|_| ModelProviderError::Storage)
}

// `models` remains the source of truth for manual-model runtime configuration.
// This one-time copy gives existing default pointers the unified selection shape
// without moving models or duplicating their credentials.
fn migrate_legacy_default_model_selections(
    connection: &Connection,
) -> Result<(), ModelProviderError> {
    if sqlite_table_exists(connection, "platform_default_models")? {
        connection
            .execute(
                "INSERT OR IGNORE INTO default_model_selections (
                    scope_key, workspace_code, model_type, selection_kind, provider_code,
                    model_code, quality_type, route_code, created_at, updated_at
                 )
                 SELECT 'platform', NULL, model_type, 'Fixed', NULL, model_code, NULL, NULL,
                        updated_at, updated_at
                 FROM platform_default_models",
                [],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    if sqlite_table_exists(connection, "workspace_default_models")? {
        connection
            .execute(
                "INSERT OR IGNORE INTO default_model_selections (
                    scope_key, workspace_code, model_type, selection_kind, provider_code,
                    model_code, quality_type, route_code, created_at, updated_at
                 )
                 SELECT workspace_code, workspace_code, model_type, 'Fixed', NULL, model_code,
                        NULL, NULL, updated_at, updated_at
                 FROM workspace_default_models",
                [],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    Ok(())
}

fn sqlite_table_exists(
    connection: &Connection,
    table_name: &str,
) -> Result<bool, ModelProviderError> {
    sqlite_table_exists_raw(connection, table_name).map_err(|_| ModelProviderError::Storage)
}

fn sqlite_table_exists_raw(connection: &Connection, table_name: &str) -> rusqlite::Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            params![table_name],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists != 0)
}

fn normalize_server_base_url(raw: Option<&str>) -> Result<String, ModelProviderError> {
    let candidate = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(PERSON_AI_DEFAULT_SERVER_BASE_URL);
    let parsed = Url::parse(candidate).map_err(|_| ModelProviderError::InvalidServerBaseUrl)?;
    if !matches!(parsed.scheme(), "https" | "http") || parsed.host_str().is_none() {
        return Err(ModelProviderError::InvalidServerBaseUrl);
    }
    Ok(candidate.trim_end_matches('/').to_owned())
}

fn read_connection(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Option<ProviderConnectionRow>, ModelProviderError> {
    connection
        .query_row(
            "SELECT provider_code, server_base_url, status, credentials_encrypted, authorized_at,
                    authorization_expires_at, last_sync_at, last_error, updated_at,
                    person_ai_user_id, person_ai_username
             FROM provider_connections WHERE provider_code = ?1",
            params![provider_code.storage_value()],
            |row| {
                let stored_code = row.get::<_, String>(0)?;
                let status = row.get::<_, String>(2)?;
                Ok(ProviderConnectionRow {
                    provider_code: ModelProviderCode::from_storage(&stored_code)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    person_ai_user_id: row.get(9)?,
                    person_ai_username: row.get(10)?,
                    server_base_url: row.get(1)?,
                    status: ProviderConnectionStatus::from_storage(&status)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    credentials_encrypted: row.get(3)?,
                    authorized_at: row.get(4)?,
                    authorization_expires_at: row.get(5)?,
                    last_sync_at: row.get(6)?,
                    last_error: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|_| ModelProviderError::Storage)
}

fn read_authorization_session(
    connection: &Connection,
    session_id: &str,
) -> Result<ProviderAuthorizationSessionRow, ModelProviderError> {
    connection
        .query_row(
            "SELECT session_id, provider_code, server_base_url, login_url, status, expires_at, created_at, updated_at
             FROM provider_authorization_sessions WHERE session_id = ?1",
            params![session_id],
            |row| {
                let provider_code = row.get::<_, String>(1)?;
                Ok(ProviderAuthorizationSessionRow {
                    session_id: row.get(0)?,
                    provider_code: ModelProviderCode::from_storage(&provider_code)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                    server_base_url: row.get(2)?,
                    login_url: row.get(3)?,
                    status: row.get(4)?,
                    expires_at: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|_| ModelProviderError::Storage)?
        .ok_or(ModelProviderError::AuthorizationSessionNotFound)
}

fn catalog_model_count(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<usize, ModelProviderError> {
    let count = connection
        .query_row(
            "SELECT
                 (SELECT COUNT(*) FROM provider_catalog_models WHERE provider_code = ?1) +
                 (SELECT COUNT(*) FROM provider_multimedia_catalog_models WHERE provider_code = ?1)",
            params![provider_code.storage_value()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| ModelProviderError::Storage)?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

/// Older builds represented Person-AI's managed embedding route as a remote
/// multimedia catalogue entry (`media_type = 5`). The route remains a default
/// selection, but it is not a physical model and must not appear in the
/// synchronised catalogue.
fn remove_legacy_person_ai_embedding_catalog_models(
    connection: &Connection,
) -> Result<(), ModelProviderError> {
    connection
        .execute(
            "DELETE FROM provider_multimedia_catalog_models
             WHERE provider_code = ?1 AND media_type = 5",
            params![ModelProviderCode::PersonAi.storage_value()],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    Ok(())
}

/// Sync history is operational diagnostics, not an audit log. Retain a small,
/// provider-scoped rolling window so scheduled synchronization cannot grow the
/// local database indefinitely.
fn trim_provider_sync_runs(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<(), ModelProviderError> {
    connection
        .execute(
            "DELETE FROM provider_sync_runs
             WHERE provider_code = ?1
               AND sync_run_code NOT IN (
                   SELECT sync_run_code
                   FROM provider_sync_runs
                   WHERE provider_code = ?1
                   ORDER BY started_at DESC, sync_run_code DESC
                   LIMIT ?2
               )",
            params![provider_code.storage_value(), MAX_PROVIDER_SYNC_RUNS],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    Ok(())
}

fn connection_view(
    row: Option<ProviderConnectionRow>,
    catalog_model_count: usize,
) -> ProviderConnectionView {
    let row = row.unwrap_or_else(|| ProviderConnectionRow {
        provider_code: ModelProviderCode::PersonAi,
        person_ai_user_id: None,
        person_ai_username: None,
        server_base_url: PERSON_AI_DEFAULT_SERVER_BASE_URL.into(),
        status: ProviderConnectionStatus::Disconnected,
        credentials_encrypted: None,
        authorized_at: None,
        authorization_expires_at: None,
        last_sync_at: None,
        last_error: None,
        updated_at: 0,
    });
    ProviderConnectionView {
        provider_code: row.provider_code,
        person_ai_user_id: row.person_ai_user_id,
        person_ai_username: row.person_ai_username,
        display_name: row.provider_code.display_name().into(),
        status: row.status,
        server_base_url: row.server_base_url,
        authorized_at: row.authorized_at,
        authorization_expires_at: row.authorization_expires_at,
        last_sync_at: row.last_sync_at,
        last_error: row.last_error,
        catalog_model_count,
        updated_at: row.updated_at,
    }
}

fn read_catalog_models(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
    let mut models = read_chat_catalog_models(connection, provider_code)?;
    models.extend(read_multimedia_catalog_models(connection, provider_code)?);
    models.sort_by(|left, right| {
        left.status
            .storage_value()
            .cmp(right.status.storage_value())
            .then_with(|| {
                left.catalog_model_type
                    .person_ai_media_type()
                    .unwrap_or(-1)
                    .cmp(
                        &right
                            .catalog_model_type
                            .person_ai_media_type()
                            .unwrap_or(-1),
                    )
            })
            .then_with(|| left.model_name.cmp(&right.model_name))
            .then_with(|| left.model_code.cmp(&right.model_code))
    });
    Ok(models)
}

fn read_chat_catalog_models(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
    let mut statement = connection
        .prepare(
            "SELECT provider_code, model_code, model_name, upstream_vendor_code, upstream_vendor_name,
                    context_window_tokens, protocols, quality_type, input_coefficient, cache_hit_coefficient,
                    output_coefficient, status, first_seen_at, last_seen_at, updated_at
             FROM provider_catalog_models WHERE provider_code = ?1
             ORDER BY CASE status WHEN 'Available' THEN 0 ELSE 1 END,
                      CASE quality_type WHEN 'High' THEN 0 WHEN 'Middle' THEN 1 ELSE 2 END,
                      upstream_vendor_name COLLATE NOCASE, model_name COLLATE NOCASE, model_code",
        )
        .map_err(|_| ModelProviderError::Storage)?;
    statement
        .query_map(params![provider_code.storage_value()], |row| {
            let provider_code = row.get::<_, String>(0)?;
            let quality_type = row.get::<_, String>(7)?;
            let status = row.get::<_, String>(11)?;
            Ok(ProviderCatalogModelView {
                provider_code: ModelProviderCode::from_storage(&provider_code)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                catalog_model_type: ProviderCatalogModelType::Chat,
                model_id: None,
                model_code: row.get(1)?,
                model_name: row.get(2)?,
                protocols: serde_json::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or_else(|_| vec![AgentModelProtocol::ChatCompletions]),
                upstream_vendor_code: row.get(3)?,
                upstream_vendor_name: row.get(4)?,
                context_window_tokens: row
                    .get::<_, Option<i64>>(5)?
                    .and_then(|value| u32::try_from(value).ok()),
                quality_type: Some(
                    ProviderModelQuality::from_storage(&quality_type)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                ),
                input_coefficient: row.get(8)?,
                cache_hit_coefficient: row.get(9)?,
                output_coefficient: row.get(10)?,
                status: ProviderCatalogModelStatus::from_storage(&status)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                first_seen_at: row.get(12)?,
                last_seen_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })
        .map_err(|_| ModelProviderError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ModelProviderError::Storage)
}

fn read_multimedia_catalog_models(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderCatalogModelView>, ModelProviderError> {
    let mut statement = connection
        .prepare(
            "SELECT provider_code, model_id, model_code, model_name, media_type, status,
                    first_seen_at, last_seen_at, updated_at
             FROM provider_multimedia_catalog_models WHERE provider_code = ?1 AND media_type != 5
             ORDER BY CASE status WHEN 'Available' THEN 0 ELSE 1 END,
                      media_type, model_name COLLATE NOCASE, model_code",
        )
        .map_err(|_| ModelProviderError::Storage)?;
    statement
        .query_map(params![provider_code.storage_value()], |row| {
            let provider_code = row.get::<_, String>(0)?;
            let media_type = row.get::<_, i32>(4)?;
            let status = row.get::<_, String>(5)?;
            Ok(ProviderCatalogModelView {
                provider_code: ModelProviderCode::from_storage(&provider_code)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                catalog_model_type: ProviderCatalogModelType::from_person_ai_media_type(media_type)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                model_id: Some(row.get(1)?),
                model_code: row.get(2)?,
                model_name: row.get(3)?,
                protocols: Vec::new(),
                upstream_vendor_code: None,
                upstream_vendor_name: None,
                context_window_tokens: None,
                quality_type: None,
                input_coefficient: None,
                cache_hit_coefficient: None,
                output_coefficient: None,
                status: ProviderCatalogModelStatus::from_storage(&status)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                first_seen_at: row.get(6)?,
                last_seen_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|_| ModelProviderError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ModelProviderError::Storage)
}

fn read_catalog_model(
    connection: &Connection,
    provider_code: ModelProviderCode,
    model_code: &str,
    quality_type: Option<ProviderModelQuality>,
) -> Result<Option<ProviderCatalogModelView>, ModelProviderError> {
    read_chat_catalog_models(connection, provider_code).map(|models| {
        models.into_iter().find(|model| {
            model.model_code == model_code
                && quality_type.is_none_or(|quality| model.quality_type == Some(quality))
        })
    })
}

fn read_sync_runs(
    connection: &Connection,
    provider_code: ModelProviderCode,
) -> Result<Vec<ProviderSyncRunView>, ModelProviderError> {
    let mut statement = connection
        .prepare(
            "SELECT sync_run_code, provider_code, status, started_at, completed_at, model_count, error_summary
             FROM provider_sync_runs WHERE provider_code = ?1
             ORDER BY started_at DESC, sync_run_code DESC LIMIT 10",
        )
        .map_err(|_| ModelProviderError::Storage)?;
    statement
        .query_map(params![provider_code.storage_value()], |row| {
            let provider_code = row.get::<_, String>(1)?;
            let status = row.get::<_, String>(2)?;
            Ok(ProviderSyncRunView {
                sync_run_code: row.get(0)?,
                provider_code: ModelProviderCode::from_storage(&provider_code)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                status: ProviderSyncStatus::from_storage(&status)
                    .ok_or(rusqlite::Error::InvalidQuery)?,
                started_at: row.get(3)?,
                completed_at: row.get(4)?,
                model_count: row
                    .get::<_, Option<i64>>(5)?
                    .and_then(|value| usize::try_from(value).ok()),
                error_summary: row.get(6)?,
            })
        })
        .map_err(|_| ModelProviderError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ModelProviderError::Storage)
}

fn read_run_snapshot(
    connection: &Connection,
    run_code: &str,
) -> Result<Option<ProviderModelRunSnapshotView>, ModelProviderError> {
    connection
        .query_row(
            "SELECT run_code, session_code, workspace_code, selection_kind, provider_code,
                    requested_model_code, quality_type, route_code, resolved_model_code, resolved_model_name,
                    resolved_upstream_vendor_code, resolved_upstream_vendor_name, created_at, updated_at
             FROM provider_model_run_snapshots WHERE run_code = ?1",
            params![run_code],
            |row| {
                let selection_kind = row.get::<_, String>(3)?;
                let provider_code = row.get::<_, String>(4)?;
                let quality_type = row.get::<_, Option<String>>(6)?;
                let route_code = row.get::<_, Option<String>>(7)?;
                let route_code = match route_code.as_deref() {
                    Some(value) => Some(
                        ProviderRouteCode::from_storage(value)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                    ),
                    None => None,
                };
                let quality_type = match quality_type.as_deref() {
                    Some(value) => Some(
                        ProviderModelQuality::from_storage(value)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                    ),
                    None => None,
                };
                Ok(ProviderModelRunSnapshotView {
                    run_code: row.get(0)?,
                    session_code: row.get(1)?,
                    workspace_code: row.get(2)?,
                    model_selection: ModelSelection {
                        kind: ModelSelectionKind::from_storage(&selection_kind)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                        provider_code: Some(
                            ModelProviderCode::from_storage(&provider_code)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        model_code: row.get(5)?,
                        quality_type,
                        route_code,
                    },
                    resolved_model_code: row.get(8)?,
                    resolved_model_name: row.get(9)?,
                    resolved_upstream_vendor_code: row.get(10)?,
                    resolved_upstream_vendor_name: row.get(11)?,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            },
        )
        .optional()
        .map_err(|_| ModelProviderError::Storage)
}

fn apply_catalog_snapshot(
    transaction: &Transaction<'_>,
    provider_code: ModelProviderCode,
    catalog: &[ExternalCatalogModel],
    now: i64,
) -> Result<(), ModelProviderError> {
    transaction
        .execute(
            "UPDATE provider_catalog_models SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
            params![
                provider_code.storage_value(),
                ProviderCatalogModelStatus::Unavailable.storage_value(),
                now,
            ],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    for model in catalog {
        transaction
            .execute(
                "INSERT INTO provider_catalog_models (
                    provider_code, model_code, model_name, upstream_vendor_code, upstream_vendor_name,
                    context_window_tokens, protocols, quality_type, input_coefficient, cache_hit_coefficient,
                    output_coefficient, status, first_seen_at, last_seen_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?13)
                 ON CONFLICT(provider_code, model_code) DO UPDATE SET
                    model_name = excluded.model_name,
                    upstream_vendor_code = excluded.upstream_vendor_code,
                    upstream_vendor_name = excluded.upstream_vendor_name,
                    context_window_tokens = excluded.context_window_tokens,
                    protocols = excluded.protocols,
                    quality_type = excluded.quality_type,
                    input_coefficient = excluded.input_coefficient,
                    cache_hit_coefficient = excluded.cache_hit_coefficient,
                    output_coefficient = excluded.output_coefficient,
                    status = excluded.status,
                    last_seen_at = excluded.last_seen_at,
                    updated_at = excluded.updated_at",
                params![
                    provider_code.storage_value(),
                    model.model_code,
                    model.model_name,
                    model.upstream_vendor_code,
                    model.upstream_vendor_name,
                    model.context_window_tokens.map(i64::from),
                    serde_json::to_string(&model.protocols).unwrap_or_else(|_| "[\"ChatCompletions\"]".into()),
                    model.quality_type.storage_value(),
                    model.input_coefficient,
                    model.cache_hit_coefficient,
                    model.output_coefficient,
                    ProviderCatalogModelStatus::Available.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    Ok(())
}

fn apply_multimedia_catalog_snapshot(
    transaction: &Transaction<'_>,
    provider_code: ModelProviderCode,
    catalog: &[ExternalMultimediaCatalogModel],
    now: i64,
) -> Result<(), ModelProviderError> {
    remove_legacy_person_ai_embedding_catalog_models(transaction)?;
    transaction
        .execute(
            "UPDATE provider_multimedia_catalog_models
             SET status = ?2, updated_at = ?3 WHERE provider_code = ?1",
            params![
                provider_code.storage_value(),
                ProviderCatalogModelStatus::Unavailable.storage_value(),
                now,
            ],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    for model in catalog {
        let media_type = model
            .catalog_model_type
            .person_ai_media_type()
            .expect("multimedia catalog entries cannot use the Chat type");
        transaction
            .execute(
                "INSERT INTO provider_multimedia_catalog_models (
                    provider_code, model_id, model_code, model_name, media_type, status,
                    first_seen_at, last_seen_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7)
                 ON CONFLICT(provider_code, model_id) DO UPDATE SET
                    model_code = excluded.model_code,
                    model_name = excluded.model_name,
                    media_type = excluded.media_type,
                    status = excluded.status,
                    last_seen_at = excluded.last_seen_at,
                    updated_at = excluded.updated_at",
                params![
                    provider_code.storage_value(),
                    model.model_id,
                    model.model_code,
                    model.model_name,
                    media_type,
                    ProviderCatalogModelStatus::Available.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    Ok(())
}

fn replace_authorized_connection(
    transaction: &Transaction<'_>,
    provider_code: ModelProviderCode,
    credentials: &ProviderCredentials,
    person_ai_user_id: &str,
    person_ai_username: &str,
    encrypted: &str,
    authorization_session_id: &str,
    catalog: &[ExternalCatalogModel],
    multimedia_catalog: &[ExternalMultimediaCatalogModel],
    sync_run_code: &str,
    now: i64,
) -> Result<(), ModelProviderError> {
    transaction
        .execute(
            "INSERT INTO provider_connections (
                provider_code, person_ai_user_id, person_ai_username, server_base_url, status, credentials_encrypted, authorized_at,
                authorization_expires_at, last_sync_at, last_error, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?7, NULL, ?7, ?7)
             ON CONFLICT(provider_code) DO UPDATE SET
                person_ai_user_id = excluded.person_ai_user_id,
                person_ai_username = excluded.person_ai_username,
                server_base_url = excluded.server_base_url,
                status = excluded.status,
                credentials_encrypted = excluded.credentials_encrypted,
                authorized_at = excluded.authorized_at,
                authorization_expires_at = excluded.authorization_expires_at,
                last_sync_at = excluded.last_sync_at,
                last_error = NULL,
                updated_at = excluded.updated_at",
            params![
                provider_code.storage_value(),
                person_ai_user_id,
                person_ai_username,
                credentials.server_base_url,
                ProviderConnectionStatus::Connected.storage_value(),
                encrypted,
                now,
                credentials.authorization_expires_at,
            ],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    apply_catalog_snapshot(transaction, provider_code, catalog, now)?;
    apply_multimedia_catalog_snapshot(transaction, provider_code, multimedia_catalog, now)?;
    transaction
        .execute(
            "INSERT INTO provider_sync_runs (
                sync_run_code, provider_code, status, started_at, completed_at, model_count, error_summary
             ) VALUES (?1, ?2, ?3, ?4, ?4, ?5, NULL)",
            params![
                sync_run_code,
                provider_code.storage_value(),
                ProviderSyncStatus::Succeeded.storage_value(),
                now,
                i64::try_from(catalog.len().saturating_add(multimedia_catalog.len()))
                    .unwrap_or(i64::MAX),
            ],
    )
    .map_err(|_| ModelProviderError::Storage)?;
    trim_provider_sync_runs(transaction, provider_code)?;
    transaction
        .execute(
            "UPDATE provider_authorization_sessions SET status = 'Completed', updated_at = ?2 WHERE session_id = ?1",
            params![authorization_session_id, now],
        )
        .map_err(|_| ModelProviderError::Storage)?;
    for (model_type, route_code) in [
        (ModelType::Text, ProviderRouteCode::ChatAuto),
        (ModelType::Embedding, ProviderRouteCode::EmbeddingAuto),
    ] {
        transaction
            .execute(
                "INSERT INTO default_model_selections (
                    scope_key, workspace_code, model_type, selection_kind, provider_code, model_code,
                    quality_type, route_code, created_at, updated_at
                 ) VALUES ('platform', NULL, ?1, ?2, ?3, NULL, NULL, ?4, ?5, ?5)
                 ON CONFLICT(scope_key, model_type) DO UPDATE SET
                    selection_kind = excluded.selection_kind,
                    provider_code = excluded.provider_code,
                    model_code = NULL,
                    quality_type = NULL,
                    route_code = excluded.route_code,
                    updated_at = excluded.updated_at",
                params![
                    model_type_storage(model_type),
                    ModelSelectionKind::ProviderRoute.storage_value(),
                    provider_code.storage_value(),
                    route_code.storage_value(),
                    now,
                ],
            )
            .map_err(|_| ModelProviderError::Storage)?;
    }
    Ok(())
}

fn effective_default_selection(
    connection: &Connection,
    workspace_code: &str,
    model_type: ModelType,
) -> Result<Option<ModelSelection>, ModelProviderError> {
    let workspace_code = workspace_code.trim();
    let workspace = if workspace_code.is_empty() {
        None
    } else {
        Some(workspace_code)
    };
    let rows = [workspace, Some("platform")];
    for scope_key in rows.into_iter().flatten() {
        let row = connection
            .query_row(
                "SELECT selection_kind, provider_code, model_code, quality_type, route_code
                 FROM default_model_selections WHERE scope_key = ?1 AND model_type = ?2",
                params![scope_key, model_type_storage(model_type)],
                |row| {
                    let kind = row.get::<_, String>(0)?;
                    let provider_code = row.get::<_, Option<String>>(1)?;
                    let quality_type = row.get::<_, Option<String>>(3)?;
                    let route_code = row.get::<_, Option<String>>(4)?;
                    let provider_code = match provider_code {
                        Some(provider_code) => Some(
                            ModelProviderCode::from_storage(&provider_code)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        None => None,
                    };
                    let route_code = match route_code {
                        Some(route_code) => Some(
                            ProviderRouteCode::from_storage(&route_code)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        None => None,
                    };
                    let quality_type = match quality_type {
                        Some(quality_type) => Some(
                            ProviderModelQuality::from_storage(&quality_type)
                                .ok_or(rusqlite::Error::InvalidQuery)?,
                        ),
                        None => None,
                    };
                    Ok(ModelSelection {
                        kind: ModelSelectionKind::from_storage(&kind)
                            .ok_or(rusqlite::Error::InvalidQuery)?,
                        provider_code,
                        model_code: row.get(2)?,
                        quality_type,
                        route_code,
                    })
                },
            )
            .optional()
            .map_err(|_| ModelProviderError::Storage)?;
        if row.is_some() {
            return Ok(row);
        }
    }
    Ok(None)
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedCredentialsPayload {
    token: String,
    refresh_token: String,
    client_id: String,
    public_key: String,
    server_base_url: String,
    authorization_expires_at: Option<i64>,
}

fn encrypt_credentials(
    cipher: &(impl VariableCipher + ?Sized),
    credentials: &ProviderCredentials,
) -> Result<String, ModelProviderError> {
    let payload = serde_json::to_string(&EncryptedCredentialsPayload {
        token: credentials.token.clone(),
        refresh_token: credentials.refresh_token.clone(),
        client_id: credentials.client_id.clone(),
        public_key: credentials.public_key.clone(),
        server_base_url: credentials.server_base_url.clone(),
        authorization_expires_at: credentials.authorization_expires_at,
    })
    .map_err(|_| ModelProviderError::Storage)?;
    let encrypted = cipher
        .encrypt(&payload, b"model_provider.credentials")
        .map_err(|_| ModelProviderError::Storage)?;
    serde_json::to_string(&encrypted)
        .map(|value| format!("enc:v1:{value}"))
        .map_err(|_| ModelProviderError::Storage)
}

fn decrypt_credentials(
    cipher: &(impl VariableCipher + ?Sized),
    encrypted: &str,
) -> Result<ProviderCredentials, ModelProviderError> {
    let payload = encrypted
        .trim()
        .strip_prefix("enc:v1:")
        .ok_or(ModelProviderError::AuthenticationRequired)?;
    let encrypted = serde_json::from_str(payload).map_err(|_| ModelProviderError::Storage)?;
    let plain = cipher
        .decrypt(&encrypted, b"model_provider.credentials")
        .map_err(|_| ModelProviderError::Storage)?;
    let payload: EncryptedCredentialsPayload =
        serde_json::from_str(&plain).map_err(|_| ModelProviderError::Storage)?;
    Ok(ProviderCredentials {
        token: payload.token,
        refresh_token: payload.refresh_token,
        client_id: payload.client_id,
        public_key: payload.public_key,
        server_base_url: payload.server_base_url,
        authorization_expires_at: payload.authorization_expires_at,
    })
}

fn required_remote_value(value: Option<String>, field: &str) -> Result<String, ModelProviderError> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ModelProviderError::Remote(format!("Person-AI authorization response omitted {field}"))
        })
}

fn remote_error(error: impl std::fmt::Display) -> ModelProviderError {
    ModelProviderError::Remote(format!("Person-AI request failed: {error}"))
}

fn person_ai_runtime_error(error: ServerApiError) -> ModelProviderError {
    remote_error(error)
}

fn is_person_ai_authorization_key_error(error: &ModelProviderError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    [
        "decrypt_error",
        "decryptfailed",
        "服务端解密失败",
        "授权密钥已失效",
    ]
    .iter()
    .any(|marker| message.contains(marker))
}

fn account_info_error(error: ServerApiError) -> ModelProviderError {
    match error {
        ServerApiError::DecryptFailed => ModelProviderError::AuthenticationRequired,
        ServerApiError::Business { code, .. } => ModelProviderError::Remote(format!(
            "Person-AI account information request was rejected (code {code})"
        )),
        _ => ModelProviderError::Remote("Person-AI account information request failed".into()),
    }
}

fn account_info_error_kind(error: &ServerApiError) -> &'static str {
    match error {
        ServerApiError::Business { .. } => "business-response",
        ServerApiError::Validation(_) => "request-validation",
        ServerApiError::Http(_) => "http-transport",
        ServerApiError::Json(_) | ServerApiError::Other(_) => "invalid-response",
        ServerApiError::Crypto(_) => "crypto",
        ServerApiError::DecryptFailed => "authorization-key",
        ServerApiError::WebSocket(_) => "websocket",
        ServerApiError::Io(_) => "io",
        ServerApiError::Url(_) => "url",
    }
}

fn parse_person_ai_account_info(
    response: Value,
) -> Result<ProviderAccountBalanceView, ModelProviderError> {
    if let Some(code) = response.get("code") {
        let successful = matches!(code, Value::Number(value) if value.as_i64() == Some(0))
            || matches!(code, Value::String(value) if value == "0" || value.eq_ignore_ascii_case("success"));
        if !successful {
            return Err(ModelProviderError::Remote(
                "Person-AI account information request was rejected".into(),
            ));
        }
    }

    let data = response.get("data").unwrap_or(&response);
    let fields = data.as_object().ok_or_else(|| {
        ModelProviderError::Remote("Person-AI account information response has invalid data".into())
    })?;
    Ok(ProviderAccountBalanceView {
        username: optional_account_string(fields, &["username"]),
        vip_level: optional_account_number(fields, &["vip_level", "vipLevel"])
            .and_then(|value| i32::try_from(value).ok()),
        // Person-AI defines remaining_tokens as the account's remaining points balance.
        remaining_points: optional_account_number(fields, &["remaining_tokens", "remainingTokens"]),
    })
}

fn optional_account_string(
    fields: &serde_json::Map<String, Value>,
    names: &[&str],
) -> Option<String> {
    names.iter().find_map(|name| {
        fields
            .get(*name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn optional_account_number(fields: &serde_json::Map<String, Value>, names: &[&str]) -> Option<i64> {
    names.iter().find_map(|name| {
        fields.get(*name).and_then(|value| match value {
            Value::Number(value) => value
                .as_i64()
                .or_else(|| value.as_f64().and_then(account_number_from_float)),
            Value::String(value) => value.trim().parse::<i64>().ok().or_else(|| {
                value
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .and_then(account_number_from_float)
            }),
            _ => None,
        })
    })
}

fn account_number_from_float(value: f64) -> Option<i64> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i64::MIN as f64
        || value > i64::MAX as f64
    {
        return None;
    }
    Some(value as i64)
}

fn safe_error_summary(error: &ModelProviderError) -> String {
    match error {
        ModelProviderError::AuthenticationRequired => "Person-AI authorization is required".into(),
        ModelProviderError::Remote(_) => "Person-AI remote request failed".into(),
        other => other.to_string(),
    }
}

fn parse_remote_timestamp(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn sync_run_code(now: i64) -> String {
    format!("PROVIDER-SYNC-{now}-{:08x}", rand::random::<u32>())
}

fn model_type_storage(model_type: ModelType) -> &'static str {
    match model_type {
        ModelType::Text => "TEXT",
        ModelType::Embedding => "EMBEDDING",
        ModelType::Rerank => "RERANK",
        ModelType::Image => "IMAGE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct UnauthorizedOnceAdapter {
        refresh_count: AtomicUsize,
        refresh_tokens: Mutex<Vec<String>>,
        chat_count: AtomicUsize,
        chat_tokens: Mutex<Vec<String>>,
        stream_count: AtomicUsize,
        stream_tokens: Mutex<Vec<String>>,
    }

    impl ModelProviderAdapter for UnauthorizedOnceAdapter {
        fn code(&self) -> ModelProviderCode {
            ModelProviderCode::PersonAi
        }

        fn create_authorization<'a>(
            &'a self,
            _server_base_url: &'a str,
            _client_name: &'a str,
            _existing_client_id: Option<&'a str>,
        ) -> ProviderFuture<'a, ExternalAuthorizationSession> {
            Box::pin(async { Err(ModelProviderError::Remote("unused in test".into())) })
        }

        fn poll_authorization<'a>(
            &'a self,
            _server_base_url: &'a str,
            _session_id: &'a str,
        ) -> ProviderFuture<'a, ExternalAuthorizationPoll> {
            Box::pin(async { Err(ModelProviderError::Remote("unused in test".into())) })
        }

        fn refresh_credentials<'a>(
            &'a self,
            server_base_url: &'a str,
            refresh_token: &'a str,
        ) -> ProviderFuture<'a, ProviderCredentials> {
            let server_base_url = server_base_url.to_owned();
            let refresh_token = refresh_token.to_owned();
            Box::pin(async move {
                self.refresh_tokens
                    .lock()
                    .expect("lock refresh tokens")
                    .push(refresh_token);
                let refresh_number = self.refresh_count.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(ProviderCredentials {
                    token: format!("access-{refresh_number}"),
                    refresh_token: format!("refresh-{refresh_number}"),
                    client_id: String::new(),
                    public_key: String::new(),
                    server_base_url,
                    authorization_expires_at: Some(now_millis() + 60 * 24 * 60 * 60 * 1000),
                })
            })
        }

        fn list_chat_models<'a>(
            &'a self,
            _credentials: &'a ProviderCredentials,
        ) -> ProviderFuture<'a, Vec<ExternalCatalogModel>> {
            Box::pin(async { Err(ModelProviderError::Remote("unused in test".into())) })
        }

        fn list_multimedia_models<'a>(
            &'a self,
            _credentials: &'a ProviderCredentials,
        ) -> ProviderFuture<'a, Vec<ExternalMultimediaCatalogModel>> {
            Box::pin(async { Err(ModelProviderError::Remote("unused in test".into())) })
        }

        fn verify_chat_auto<'a>(
            &'a self,
            _credentials: &'a ProviderCredentials,
        ) -> ProviderFuture<'a, ()> {
            Box::pin(async { Err(ModelProviderError::Remote("unused in test".into())) })
        }

        fn verify_embedding<'a>(
            &'a self,
            _credentials: &'a ProviderCredentials,
        ) -> ProviderFuture<'a, ()> {
            Box::pin(async { Err(ModelProviderError::Remote("unused in test".into())) })
        }

        fn chat<'a>(
            &'a self,
            credentials: &'a ProviderCredentials,
            _body: &'a Value,
            _quality_type: ProviderModelQuality,
            _conversation_id: Option<&'a str>,
        ) -> ProviderFuture<'a, Value> {
            let token = credentials.token.clone();
            Box::pin(async move {
                self.chat_tokens
                    .lock()
                    .expect("lock chat tokens")
                    .push(token);
                if self.chat_count.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(ModelProviderError::Unauthorized)
                } else {
                    Ok(json!({"model": "mock-model"}))
                }
            })
        }

        fn chat_stream<'a>(
            &'a self,
            credentials: &'a ProviderCredentials,
            _body: &'a Value,
            _quality_type: ProviderModelQuality,
            _conversation_id: Option<&'a str>,
            _sink: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync + 'a>,
        ) -> ProviderFuture<'a, ()> {
            let token = credentials.token.clone();
            Box::pin(async move {
                self.stream_tokens
                    .lock()
                    .expect("lock stream tokens")
                    .push(token);
                if self.stream_count.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(ModelProviderError::Unauthorized)
                } else {
                    Ok(())
                }
            })
        }

        fn embed<'a>(
            &'a self,
            _credentials: &'a ProviderCredentials,
            _body: &'a Value,
        ) -> ProviderFuture<'a, Value> {
            Box::pin(async { Err(ModelProviderError::Remote("unused in test".into())) })
        }
    }

    #[tokio::test]
    async fn chat_and_stream_refresh_401_rebuild_requests_and_persist_rotated_tokens() {
        let cipher: Arc<dyn VariableCipher> = Arc::new(
            Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
                key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
            })
            .expect("create test cipher"),
        );
        let connection = Connection::open_in_memory().expect("open provider test database");
        connection
            .execute_batch(MODEL_PROVIDER_MIGRATION)
            .expect("create provider schema");
        let adapter = Arc::new(UnauthorizedOnceAdapter::default());
        let service = ModelProviderService {
            connection: Arc::new(Mutex::new(connection)),
            token_refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
            cipher,
            adapters: Arc::new(HashMap::from([(
                ModelProviderCode::PersonAi,
                adapter.clone() as Arc<dyn ModelProviderAdapter>,
            )])),
        };
        let initial_credentials = ProviderCredentials {
            token: "expired-access".into(),
            refresh_token: "initial-refresh".into(),
            client_id: "client-id".into(),
            public_key: "public-key".into(),
            server_base_url: "https://person-ai.invalid".into(),
            authorization_expires_at: Some(now_millis() + 60 * 24 * 60 * 60 * 1000),
        };
        let encrypted = service
            .encrypt_credentials(&initial_credentials)
            .expect("encrypt initial credentials");
        service
            .connection
            .lock()
            .expect("lock provider database")
            .execute(
                "INSERT INTO provider_connections (
                    provider_code, server_base_url, status, credentials_encrypted, authorized_at,
                    authorization_expires_at, last_sync_at, last_error, created_at, updated_at
                 ) VALUES ('PersonAi', ?1, 'Connected', ?2, ?3, ?4, NULL, NULL, ?3, ?3)",
                params![
                    initial_credentials.server_base_url,
                    encrypted,
                    now_millis(),
                    initial_credentials.authorization_expires_at,
                ],
            )
            .expect("seed provider credentials");

        let config = person_ai_agent_config(
            "PERSON_AI:ROUTE:ChatAuto",
            "Auto",
            Some(256_000),
            None,
            AgentModelProtocol::ChatCompletions,
        );
        service
            .chat_completion(&config, &json!({"messages": []}), None, None)
            .await
            .expect("chat should retry with refreshed credentials");
        service
            .chat_completion_stream(
                &config,
                &json!({"messages": []}),
                None,
                None,
                Arc::new(|_| Ok(())),
            )
            .await
            .expect("stream should retry with refreshed credentials");

        assert_eq!(
            *adapter.chat_tokens.lock().expect("read chat tokens"),
            ["expired-access", "access-1"]
        );
        assert_eq!(
            *adapter.stream_tokens.lock().expect("read stream tokens"),
            ["access-1", "access-2"]
        );
        assert_eq!(
            *adapter.refresh_tokens.lock().expect("read refresh tokens"),
            ["initial-refresh", "refresh-1"]
        );
        let stored = {
            let connection = service.connection.lock().expect("lock provider database");
            read_connection(&connection, ModelProviderCode::PersonAi)
                .expect("read provider connection")
                .expect("provider connection exists")
        };
        let stored_credentials = service
            .decrypt_credentials(
                stored
                    .credentials_encrypted
                    .as_deref()
                    .expect("stored encrypted credentials"),
            )
            .expect("decrypt stored credentials");
        assert_eq!(stored_credentials.token, "access-2");
        assert_eq!(stored_credentials.refresh_token, "refresh-2");
        assert_eq!(stored_credentials.client_id, "client-id");
        assert_eq!(stored_credentials.public_key, "public-key");
    }

    #[test]
    fn person_ai_invalid_authorization_key_is_detected_from_remote_error() {
        let error = ModelProviderError::Remote(
            "Person-AI request failed: 服务端解密失败: 授权密钥已失效，请重新登录后再试".into(),
        );
        assert!(is_person_ai_authorization_key_error(&error));

        let ordinary_error = ModelProviderError::Remote("Person-AI request failed: timeout".into());
        assert!(!is_person_ai_authorization_key_error(&ordinary_error));
    }

    fn catalog_model(
        code: &str,
        name: &str,
        vendor: &str,
        context_window_tokens: Option<u32>,
    ) -> ExternalCatalogModel {
        ExternalCatalogModel {
            model_code: code.into(),
            model_name: name.into(),
            protocols: vec![AgentModelProtocol::ChatCompletions],
            upstream_vendor_code: Some(vendor.to_ascii_lowercase()),
            upstream_vendor_name: Some(vendor.into()),
            context_window_tokens,
            quality_type: ProviderModelQuality::High,
            input_coefficient: Some(0.8),
            cache_hit_coefficient: Some(0.1),
            output_coefficient: Some(1.2),
        }
    }

    #[test]
    fn catalog_schema_adds_point_coefficients_without_recreating_existing_catalogs() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE provider_catalog_models (
                    provider_code TEXT NOT NULL,
                    model_code TEXT NOT NULL,
                    model_name TEXT NOT NULL,
                    upstream_vendor_code TEXT,
                    upstream_vendor_name TEXT,
                    context_window_tokens INTEGER,
                    status TEXT NOT NULL,
                    first_seen_at INTEGER NOT NULL,
                    last_seen_at INTEGER,
                    updated_at INTEGER NOT NULL,
                    PRIMARY KEY(provider_code, model_code)
                );",
            )
            .expect("create legacy catalog table");

        migrate_provider_catalog_model_schema(&connection).expect("migrate catalog table");

        let columns = connection
            .prepare("PRAGMA table_info(provider_catalog_models)")
            .expect("prepare table metadata")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("read table metadata")
            .collect::<Result<HashSet<_>, _>>()
            .expect("collect table metadata");
        assert!(columns.contains("input_coefficient"));
        assert!(columns.contains("cache_hit_coefficient"));
        assert!(columns.contains("output_coefficient"));
        assert!(columns.contains("quality_type"));
    }

    #[test]
    fn full_catalog_snapshot_updates_and_marks_missing_models_unavailable() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(MODEL_PROVIDER_MIGRATION)
            .expect("create provider tables");

        let transaction = connection.transaction().expect("start first snapshot");
        apply_catalog_snapshot(
            &transaction,
            ModelProviderCode::PersonAi,
            &[
                catalog_model("deepseek-v4", "DeepSeek V4", "DeepSeek", Some(64_000)),
                catalog_model("retired", "Retired", "Qwen", Some(32_000)),
            ],
            100,
        )
        .expect("apply first snapshot");
        transaction.commit().expect("commit first snapshot");

        let transaction = connection.transaction().expect("start second snapshot");
        let mut updated_deepseek =
            catalog_model("deepseek-v4", "DeepSeek V4 Pro", "DeepSeek", Some(128_000));
        updated_deepseek.input_coefficient = Some(1.6);
        updated_deepseek.cache_hit_coefficient = Some(0.2);
        updated_deepseek.output_coefficient = Some(2.4);
        apply_catalog_snapshot(
            &transaction,
            ModelProviderCode::PersonAi,
            &[
                updated_deepseek,
                catalog_model("glm-5", "GLM 5", "GLM", Some(96_000)),
            ],
            200,
        )
        .expect("apply complete second snapshot");
        transaction.commit().expect("commit second snapshot");

        let catalog = read_catalog_models(&connection, ModelProviderCode::PersonAi)
            .expect("read synchronized catalog");
        let deepseek = catalog
            .iter()
            .find(|model| model.model_code == "deepseek-v4")
            .expect("updated model");
        assert_eq!(deepseek.model_name, "DeepSeek V4 Pro");
        assert_eq!(deepseek.context_window_tokens, Some(128_000));
        assert_eq!(deepseek.input_coefficient, Some(1.6));
        assert_eq!(deepseek.cache_hit_coefficient, Some(0.2));
        assert_eq!(deepseek.output_coefficient, Some(2.4));
        assert_eq!(deepseek.status, ProviderCatalogModelStatus::Available);
        assert!(catalog.iter().any(|model| {
            model.model_code == "glm-5" && model.status == ProviderCatalogModelStatus::Available
        }));
        assert!(catalog.iter().any(|model| {
            model.model_code == "retired" && model.status == ProviderCatalogModelStatus::Unavailable
        }));
    }

    #[test]
    fn legacy_default_pointers_migrate_into_fixed_selections() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(MODEL_PROVIDER_MIGRATION)
            .expect("create provider tables");
        connection
            .execute_batch(
                "CREATE TABLE platform_default_models (
                    model_type TEXT NOT NULL PRIMARY KEY,
                    model_code TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE workspace_default_models (
                    workspace_code TEXT NOT NULL,
                    model_type TEXT NOT NULL,
                    model_code TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    PRIMARY KEY (workspace_code, model_type)
                 );
                 INSERT INTO platform_default_models VALUES
                    ('TEXT', 'MODEL-PLATFORM-TEXT', 100),
                    ('EMBEDDING', 'MODEL-PLATFORM-EMBEDDING', 100);
                 INSERT INTO workspace_default_models VALUES
                    ('workspace-1', 'TEXT', 'MODEL-WORKSPACE-TEXT', 200);",
            )
            .expect("seed legacy defaults");

        migrate_legacy_default_model_selections(&connection).expect("migrate defaults");

        assert_eq!(
            effective_default_selection(&connection, "workspace-1", ModelType::Text)
                .expect("read workspace default"),
            Some(ModelSelection::fixed("MODEL-WORKSPACE-TEXT"))
        );
        assert_eq!(
            effective_default_selection(&connection, "workspace-1", ModelType::Embedding)
                .expect("read platform fallback"),
            Some(ModelSelection::fixed("MODEL-PLATFORM-EMBEDDING"))
        );
    }

    #[test]
    fn run_snapshot_keeps_requested_policy_and_actual_person_ai_model() {
        let cipher: Arc<dyn VariableCipher> = Arc::new(
            Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
                key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
            })
            .expect("create test cipher"),
        );
        let service =
            ModelProviderService::connect(":memory:", cipher).expect("create provider service");
        {
            let connection = service.connection.lock().expect("lock provider database");
            connection
                .execute(
                    "INSERT INTO provider_catalog_models (
                        provider_code, model_code, model_name, upstream_vendor_code,
                        upstream_vendor_name, context_window_tokens, status, first_seen_at,
                        last_seen_at, updated_at
                     ) VALUES ('PersonAi', 'deepseek-v4', 'DeepSeek V4', 'deepseek',
                        'DeepSeek', 64000, 'Available', 1, 1, 1)",
                    [],
                )
                .expect("seed catalog model");
        }
        let config = person_ai_agent_config(
            person_ai_fixed_runtime_code(ProviderModelQuality::High, "deepseek-v4"),
            "deepseek-v4",
            Some(64_000),
            None,
            AgentModelProtocol::ChatCompletions,
        );
        service
            .record_requested_runtime_snapshot(Some("RUN-1"), "SESSION-1", "WS-1", &config)
            .expect("record requested model");
        service
            .record_runtime_snapshot(
                Some("RUN-1"),
                "",
                &config,
                &json!({
                    "model": "deepseek-v4-0731",
                    "model_name": "DeepSeek V4 0731",
                    "provider_name": "DeepSeek"
                }),
            )
            .expect("record actual model");

        let snapshot = service
            .get_run_snapshot("RUN-1")
            .expect("read model snapshot")
            .expect("stored model snapshot");
        assert_eq!(snapshot.session_code.as_deref(), Some("SESSION-1"));
        assert_eq!(snapshot.workspace_code, "WS-1");
        assert_eq!(
            snapshot.model_selection,
            ModelSelection::person_ai_fixed("deepseek-v4", ProviderModelQuality::High)
        );
        assert_eq!(
            snapshot.resolved_model_code.as_deref(),
            Some("deepseek-v4-0731")
        );
        assert_eq!(
            snapshot.resolved_model_name.as_deref(),
            Some("DeepSeek V4 0731")
        );
        assert_eq!(
            snapshot.resolved_upstream_vendor_name.as_deref(),
            Some("DeepSeek")
        );
    }
}
