#[cfg(test)]
use std::sync::Mutex;
use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use chrono::DateTime;
use crab_serverapi::{
    AuthorizedClientApi, AuthorizedClientContext, ServerApiError,
    auth::{AccountClient, AccountInfo, LoginClient, SessionRequest},
    gateway::{ChatCompletionsClient, ProtocolType, QualityType},
    multimedia::MultimediaClient,
};
use reqwest::Method;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
use tokio::sync::broadcast;
use url::Url;

mod chat_auto_routes;
mod route_sources;
mod person_ai_services;
mod person_ai_tools;
mod sdk_models;

use crate::{
    application::{
        llm::LlmRuntimeModelResolver,
        model::{ModelService, RuntimeModelConfig},
        variable::VariableCipher,
    },
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

const PERSON_AI_DEFAULT_SERVER_BASE_URL: &str = "https://personai.chat/person-ai-index";
const PERSON_AI_CLIENT_NAME: &str = "Tron";
const PERSON_AI_PRODUCT_CODE: &str = "tron";
const PERSON_AI_PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROVIDER_RUNTIME_PREFIX: &str = "PERSON_AI:";
const PERSON_AI_RERANK_PREFIX: &str = "PERSON_AI:RERANK:";
const PERSON_AI_EMBEDDING_PREFIX: &str = "PERSON_AI:EMBEDDING:";
const TOKEN_REFRESH_LEAD_MILLIS: i64 = 7 * 24 * 60 * 60 * 1000;
const MAX_PROVIDER_SYNC_RUNS: i64 = 10;

#[derive(Clone)]
pub struct ModelProviderService {
    auto_routers: eliteforge_agent_core::AutoRouters,
    model_executor: Arc<eliteforge_agent_framework::ModelClient>,
    model_attempt_salt: Arc<String>,
    connection: crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase,
    token_refresh_lock: Arc<tokio::sync::Mutex<()>>,
    server_auth_events: broadcast::Sender<()>,
    device_unbound: Arc<AtomicBool>,
    embedding_route_counter: Arc<AtomicU64>,
    device_id: String,
    cipher: Arc<dyn VariableCipher>,
    adapters: Arc<HashMap<ModelProviderCode, Arc<dyn ModelProviderAdapter>>>,
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
struct PersonAiProfileCredentialsRow {
    server_base_url: String,
    credentials_encrypted: String,
    status: ProviderConnectionStatus,
    authorized_at: i64,
    authorization_expires_at: Option<i64>,
    last_error: Option<String>,
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

mod contracts;
pub use contracts::*;

mod adapter;
pub use adapter::*;

mod schema;
use schema::*;

mod runtime;
pub use runtime::*;

mod storage_read;
use storage_read::*;

mod storage_write;
use storage_write::*;

mod utilities;
use utilities::*;
mod authorization;
mod auto_selection;
mod credentials;
mod encryption_keys;
mod lifecycle;
mod selection;
mod snapshots;
mod sync;
#[cfg(test)]
mod tests {
    use crate::adapters::outbound::crypto::Sm4GcmVariableCipher;
    use crate::config::VariableCryptoConfig;

    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct UnauthorizedOnceAdapter {
        refresh_count: AtomicUsize,
        refresh_tokens: Mutex<Vec<String>>,
        block: AtomicBool,
        fail: AtomicBool,
        remote_failure: AtomicBool,
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
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
            _device_id: &'a str,
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
                if self.block.load(Ordering::SeqCst) {
                    self.entered.notify_one();
                    self.release.notified().await;
                }
                if self.remote_failure.load(Ordering::SeqCst) {
                    return Err(ModelProviderError::Remote(
                        "refresh temporarily unavailable".into(),
                    ));
                }
                if self.fail.load(Ordering::SeqCst) {
                    return Err(ModelProviderError::DeviceUnbound);
                }
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
    }

    #[tokio::test]
    async fn credential_refresh_policy_persists_rotated_tokens_without_model_resubmission() {
        let cipher: Arc<dyn VariableCipher> = Arc::new(
            Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
                key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
            })
            .expect("create test cipher"),
        );
        let connection = crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(":memory:").unwrap();
        let adapter = Arc::new(UnauthorizedOnceAdapter::default());
        let service = ModelProviderService {
            auto_routers: auto_selection::default_auto_routers(),
            model_executor: Default::default(),
            model_attempt_salt: Arc::new(eliteforge_agent_core::RunId::new_v4().to_string()),
            connection,
            token_refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
            server_auth_events: broadcast::channel(16).0,
            device_unbound: Arc::new(AtomicBool::new(false)),
            embedding_route_counter: Arc::new(AtomicU64::new(0)),
            device_id: "test-device-id".into(),
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
            .call(
                move |connection| {
                    connection
                        .execute(
                            "INSERT INTO provider_connections (
                    provider_code, server_base_url, status, credentials_encrypted, authorized_at,
                    authorization_expires_at, last_sync_at, last_error, created_at, updated_at
                 ) VALUES ('PersonAi', ?1, 'Connected', ?2, ?3, ?4, NULL, NULL, ?3, ?3)",
                            params![
                                initial_credentials.server_base_url.clone(),
                                encrypted,
                                now_millis(),
                                initial_credentials.authorization_expires_at,
                            ],
                        )
                        .expect("seed provider credentials");
                    Ok::<_, rusqlite::Error>(())
                },
                || rusqlite::Error::InvalidQuery,
            )
            .unwrap();

        let identity = service.sdk_credential_attempt("expired-access");
        let rejected = eliteforge_agent_core::ModelError::Provider(Box::new(
            eliteforge_agent_core::ProviderModelError {
                authorization_attempt: Some(identity),
                provider: "personai".into(),
                category: eliteforge_agent_core::ProviderErrorCategory::Authentication,
                http_status: Some(401),
                business_code: None,
                error_type: None,
                model: None,
                message: "expired".into(),
            },
        ));
        // An upstream outage must leave the same credentials retryable.
        adapter.remote_failure.store(true, Ordering::SeqCst);
        let failure = service
            .sdk_recover_model_authentication(&rejected)
            .await
            .unwrap_err();
        assert!(
            failure
                .to_string()
                .contains("refresh temporarily unavailable")
        );
        service.sdk_final_model_failure(&failure);
        let retained = service
            .connection
            .call(
                |connection| read_connection(connection, ModelProviderCode::PersonAi),
                || ModelProviderError::Storage,
            )
            .unwrap()
            .unwrap();
        assert_eq!(retained.status, ProviderConnectionStatus::Connected);
        assert!(retained.last_error.is_none());
        let retained_credentials = service
            .decrypt_credentials(retained.credentials_encrypted.as_deref().unwrap())
            .unwrap();
        assert_eq!(retained_credentials.token, "expired-access");
        assert_eq!(retained_credentials.refresh_token, "initial-refresh");
        adapter.remote_failure.store(false, Ordering::SeqCst);
        adapter.refresh_count.store(0, Ordering::SeqCst);
        adapter.refresh_tokens.lock().unwrap().clear();

        let (first, concurrent) = tokio::join!(
            service.sdk_recover_model_authentication(&rejected),
            service.sdk_recover_model_authentication(&rejected)
        );
        assert!(first.unwrap() && concurrent.unwrap());
        assert_eq!(adapter.refresh_count.load(Ordering::SeqCst), 1);
        let first = service
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .unwrap();
        assert_eq!(first.token, "access-1");
        let mut rejected_new = rejected.clone();
        if let eliteforge_agent_core::ModelError::Provider(error) = &mut rejected_new {
            error.authorization_attempt = Some(service.sdk_credential_attempt("access-1"));
        }
        let (stale, new) = tokio::join!(
            service.sdk_recover_model_authentication(&rejected),
            service.sdk_recover_model_authentication(&rejected_new)
        );
        assert!(stale.unwrap() && new.unwrap());
        assert_eq!(adapter.refresh_count.load(Ordering::SeqCst), 2);
        let second = service
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .unwrap();
        assert_eq!(second.token, "access-2");
        assert_eq!(
            *adapter.refresh_tokens.lock().expect("read refresh tokens"),
            ["initial-refresh", "refresh-1"]
        );
        let stored = {
            service
                .connection
                .call(
                    move |connection| read_connection(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage,
                )
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
        use eliteforge_agent_official_plugin_model_http::ModelTransportFactory;
        let factory = crate::adapters::outbound::model::PersonAiModelProviderClient::new(
            crate::adapters::outbound::model::HttpModelProviderClient::new(
                eliteforge_agent_official_plugin_model_http::ModelHttpClient::new(),
            ),
            service.clone(),
        );
        let mut config = person_ai_agent_config(
            "PERSON_AI:ROUTE:ChatAuto",
            "",
            None,
            None,
            AgentModelProtocol::ChatCompletions,
        );
        config.model_code = "external".into();
        let external: eliteforge_agent_official_plugin_model_http::AgentModelConfig =
            eliteforge_agent_official_plugin_model_http::AgentModelConfig::from(&config);
        factory.on_failure(&external, &rejected);
        assert_eq!(
            service
                .connection
                .call(
                    |connection| read_connection(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage
                )
                .unwrap()
                .unwrap()
                .status,
            ProviderConnectionStatus::Connected
        );
        config.model_code = "PERSON_AI:ROUTE:ChatAuto".into();
        let personai: eliteforge_agent_official_plugin_model_http::AgentModelConfig =
            eliteforge_agent_official_plugin_model_http::AgentModelConfig::from(&config);
        factory.on_failure(&personai, &rejected);
        assert_eq!(
            service
                .connection
                .call(
                    |connection| read_connection(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage
                )
                .unwrap()
                .unwrap()
                .status,
            ProviderConnectionStatus::Connected
        );
        let mut rejected_current = rejected.clone();
        if let eliteforge_agent_core::ModelError::Provider(error) = &mut rejected_current {
            error.authorization_attempt = Some(service.sdk_credential_attempt("access-2"));
        }
        factory.on_failure(&personai, &rejected_current);
        assert_eq!(
            service
                .connection
                .call(
                    |connection| read_connection(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage
                )
                .unwrap()
                .unwrap()
                .status,
            ProviderConnectionStatus::Degraded
        );
        // A call without recovery budget leaves credentials available for automatic refresh.
        let refreshed = service
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .unwrap();
        assert_eq!(refreshed.token, "access-3");
        assert_eq!(adapter.refresh_count.load(Ordering::SeqCst), 3);
        assert_eq!(
            service
                .connection
                .call(
                    |connection| read_connection(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage,
                )
                .unwrap()
                .unwrap()
                .status,
            ProviderConnectionStatus::Connected
        );
        service.connection.call(
            |connection| connection.execute("UPDATE provider_connections SET status='AuthRequired',last_error='PersonAI rejected access credentials; reauthorization required' WHERE provider_code='PersonAi'", []),
            || rusqlite::Error::InvalidQuery,
        ).unwrap();
        let recovered_legacy = service
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .unwrap();
        assert_eq!(recovered_legacy.token, "access-4");
        assert_eq!(adapter.refresh_count.load(Ordering::SeqCst), 4);
        service.connection.call(
            |connection| connection.execute("UPDATE provider_connections SET status='AuthRequired',last_error='token refresh failed' WHERE provider_code='PersonAi'", []),
            || rusqlite::Error::InvalidQuery,
        ).unwrap();
        assert!(matches!(
            service
                .active_credentials(ModelProviderCode::PersonAi)
                .await,
            Err(ModelProviderError::AuthenticationRequired)
        ));
        assert_eq!(adapter.refresh_count.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn activating_a_profile_restores_its_own_person_ai_credentials() {
        let cipher: Arc<dyn VariableCipher> = Arc::new(
            Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
                key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
            })
            .expect("create test cipher"),
        );
        let service =
            ModelProviderService::connect(":memory:", cipher).expect("create provider service");
        let alice = ProviderCredentials {
            token: "alice-access".into(),
            refresh_token: "alice-refresh".into(),
            client_id: "desktop-client".into(),
            public_key: "alice-public-key".into(),
            server_base_url: "https://person-ai.invalid".into(),
            authorization_expires_at: Some(now_millis() + 60_000),
        };
        let bob = ProviderCredentials {
            token: "bob-access".into(),
            refresh_token: "bob-refresh".into(),
            client_id: "desktop-client".into(),
            public_key: "bob-public-key".into(),
            server_base_url: "https://person-ai.invalid".into(),
            authorization_expires_at: Some(now_millis() + 60_000),
        };

        for (user_id, username, credentials) in [
            ("person-alice", "alice", &alice),
            ("person-bob", "bob", &bob),
        ] {
            let encrypted = service
                .encrypt_credentials(credentials)
                .expect("encrypt profile credentials");
            let credentials = credentials.clone();
            service.connection.call(move |connection| {
                let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(connection,rusqlite::TransactionBehavior::Immediate).unwrap();
            replace_authorized_connection(
                &transaction,
                ModelProviderCode::PersonAi,
                &credentials,
                user_id,
                username,
                &encrypted,
                "test-session",
                &[],
                &[],
                &format!("test-sync-{username}"),
                now_millis(),
            )
            .expect("store profile credentials");
            transaction
                .commit()
                .expect("commit authorization transaction");
                Ok::<_,ModelProviderError>(())
            },||ModelProviderError::Storage).unwrap();
        }

        for (user_id, username, expected_token) in [
            ("person-alice", "alice", "alice-access"),
            ("person-bob", "bob", "bob-access"),
            ("person-alice", "alice", "alice-access"),
        ] {
            service
                .activate_person_ai_profile(user_id, username)
                .expect("activate profile credentials");
            let active = {
                service
                    .connection
                    .call(
                        move |connection| read_connection(connection, ModelProviderCode::PersonAi),
                        || ModelProviderError::Storage,
                    )
                    .expect("read provider connection")
                    .expect("active provider connection")
            };
            assert_eq!(active.person_ai_user_id.as_deref(), Some(user_id));
            assert_eq!(active.person_ai_username.as_deref(), Some(username));
            assert_eq!(
                service
                    .decrypt_credentials(
                        active
                            .credentials_encrypted
                            .as_deref()
                            .expect("active encrypted credentials"),
                    )
                    .expect("decrypt active credentials")
                    .token,
                expected_token
            );
        }

        service
            .activate_person_ai_profile("person-charlie", "charlie")
            .expect("activate profile without stored credentials");
        let active = {
            service
                .connection
                .call(
                    move |connection| read_connection(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage,
                )
                .expect("read provider connection")
                .expect("active provider connection")
        };
        assert_eq!(active.person_ai_user_id.as_deref(), Some("person-charlie"));
        assert_eq!(active.status, ProviderConnectionStatus::AuthRequired);
        assert!(active.credentials_encrypted.is_none());
    }

    #[tokio::test]
    async fn switched_profile_rejects_inflight_refresh_success_and_failure() {
        for failure in [false, true] {
            let cipher: Arc<dyn VariableCipher> = Arc::new(
                Sm4GcmVariableCipher::from_config(VariableCryptoConfig {
                    key_base64: "MDEyMzQ1Njc4OUFCQ0RFRg==".into(),
                })
                .unwrap(),
            );
            let mut service = ModelProviderService::connect(":memory:", cipher).unwrap();
            let adapter = Arc::new(UnauthorizedOnceAdapter::default());
            adapter.block.store(true, Ordering::SeqCst);
            adapter.fail.store(failure, Ordering::SeqCst);
            service.adapters = Arc::new(HashMap::from([(
                ModelProviderCode::PersonAi,
                adapter.clone() as Arc<dyn ModelProviderAdapter>,
            )]));
            for user in ["alice", "bob"] {
                let credentials = ProviderCredentials {
                    token: format!("{user}-token"),
                    refresh_token: format!("{user}-refresh"),
                    client_id: "client".into(),
                    public_key: "key".into(),
                    server_base_url: format!("https://{user}.invalid"),
                    authorization_expires_at: Some(now_millis() + 3_600_000),
                };
                let encrypted = service.encrypt_credentials(&credentials).unwrap();
                service.connection.call(move |connection| {
                    let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(connection,rusqlite::TransactionBehavior::Immediate).unwrap();
                replace_authorized_connection(
                    &transaction,
                    ModelProviderCode::PersonAi,
                    &credentials,
                    user,
                    user,
                    &encrypted,
                    "session",
                    &[],
                    &[],
                    &format!("sync-{user}"),
                    now_millis(),
                )
                .unwrap();
                transaction.commit().unwrap();
                    Ok::<_,ModelProviderError>(())
                },||ModelProviderError::Storage).unwrap();
            }
            service
                .activate_person_ai_profile("alice", "alice")
                .unwrap();

            let attempt = service.sdk_credential_attempt("alice-token");
            service.activate_person_ai_profile("bob", "bob").unwrap();
            let rejected = eliteforge_agent_core::ModelError::Provider(Box::new(
                eliteforge_agent_core::ProviderModelError {
                    authorization_attempt: Some(attempt),
                    provider: "personai".into(),
                    category: eliteforge_agent_core::ProviderErrorCategory::Authentication,
                    http_status: Some(401),
                    business_code: None,
                    error_type: None,
                    model: None,
                    message: "rejected".into(),
                },
            ));
            assert!(
                service
                    .sdk_recover_model_authentication(&rejected)
                    .await
                    .is_err()
            );
            assert_eq!(
                adapter.refresh_count.load(Ordering::SeqCst),
                0,
                "switched profile must not refresh or authorize a retry"
            );
            service.sdk_final_model_failure(&rejected);
            {
                let active = service
                    .connection
                    .call(
                        move |connection| read_connection(connection, ModelProviderCode::PersonAi),
                        || ModelProviderError::Storage,
                    )
                    .unwrap()
                    .unwrap();
                assert_eq!(active.status, ProviderConnectionStatus::Connected);
                assert_eq!(
                    service
                        .decrypt_credentials(active.credentials_encrypted.as_deref().unwrap())
                        .unwrap()
                        .token,
                    "bob-token"
                );
            }
            service
                .activate_person_ai_profile("alice", "alice")
                .unwrap();
            let running = {
                let service = service.clone();
                tokio::spawn(async move {
                    service
                        .refresh_credentials_after_unauthorized(
                            ModelProviderCode::PersonAi,
                            "alice-token",
                        )
                        .await
                })
            };
            adapter.entered.notified().await;
            service.activate_person_ai_profile("bob", "bob").unwrap();
            adapter.release.notify_one();
            assert!(matches!(
                running.await.unwrap(),
                Err(ModelProviderError::ConnectionNotReady)
            ));
            let active = service
                .connection
                .call(
                    move |connection| read_connection(connection, ModelProviderCode::PersonAi),
                    || ModelProviderError::Storage,
                )
                .unwrap()
                .unwrap();
            assert_eq!(active.person_ai_user_id.as_deref(), Some("bob"));
            assert_eq!(active.status, ProviderConnectionStatus::Connected);
            assert_eq!(
                service
                    .decrypt_credentials(active.credentials_encrypted.as_deref().unwrap())
                    .unwrap()
                    .token,
                "bob-token"
            );
            for user in ["alice", "bob"] {
                let profile = service
                    .connection
                    .call(
                        move |connection| read_person_ai_profile_credentials(connection, user),
                        || ModelProviderError::Storage,
                    )
                    .unwrap()
                    .unwrap();
                assert_eq!(profile.status, ProviderConnectionStatus::Connected);
                assert_eq!(
                    service
                        .decrypt_credentials(&profile.credentials_encrypted)
                        .unwrap()
                        .token,
                    format!("{user}-token")
                );
            }
            assert!(!service.device_unbound.load(Ordering::SeqCst));
        }
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
            tags: vec![],
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
    fn multimedia_catalog_persists_vendor_and_coefficients() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(include_str!("../../../migrations/0001_init.sql")).unwrap();
        let tx = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate).unwrap();
        let model = ExternalMultimediaCatalogModel { model_id: "id".into(), model_code: "model".into(), model_name: "Model".into(), catalog_model_type: ProviderCatalogModelType::Embedding, dimension: Some(1024), tags: vec!["text_embedding".into()], upstream_vendor_code: Some("vendor".into()), upstream_vendor_name: Some("Vendor".into()), input_coefficient: Some(0.1), cache_hit_coefficient: Some(0.2), output_coefficient: Some(0.3) };
        apply_multimedia_catalog_snapshot(&tx, ModelProviderCode::PersonAi, &[model], 100).unwrap();
        tx.commit().unwrap();
        let row = read_multimedia_catalog_models(&connection, ModelProviderCode::PersonAi).unwrap().remove(0);
        assert_eq!(row.upstream_vendor_name.as_deref(), Some("Vendor"));
        assert_eq!((row.input_coefficient, row.cache_hit_coefficient, row.output_coefficient), (Some(0.1), Some(0.2), Some(0.3)));
        assert_eq!(row.tags, vec!["text_embedding"]);
    }

    #[test]
    fn full_catalog_snapshot_updates_and_marks_missing_models_unavailable() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(include_str!("../../../migrations/0001_init.sql"))
            .expect("create provider tables");

        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection,rusqlite::TransactionBehavior::Immediate).expect("start first snapshot");
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

        let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection,rusqlite::TransactionBehavior::Immediate).expect("start second snapshot");
        let mut updated_deepseek =
            catalog_model("deepseek-v4", "DeepSeek V4 Pro", "DeepSeek", Some(128_000));
        updated_deepseek.tags = vec!["visual_understanding".into()];
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
        assert_eq!(deepseek.tags, vec!["visual_understanding"]);
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
            service
                .connection
                .call(
                    move |connection| {
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
                        Ok::<_, ModelProviderError>(())
                    },
                    || ModelProviderError::Storage,
                )
                .unwrap();
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
