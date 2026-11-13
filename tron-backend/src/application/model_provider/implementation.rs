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

use tokio::sync::broadcast;
use url::Url;

mod capability_center;
mod chat_auto_routes;
mod person_ai_services;
mod person_ai_tools;
mod route_sources;
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
