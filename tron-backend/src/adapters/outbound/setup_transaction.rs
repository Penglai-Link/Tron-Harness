use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use bcrypt::hash;
use chrono::Utc;
use rand::RngCore;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;

use crate::{
    application::model_provider::migrate_provider_catalog_model_schema,
    application::setup_state::SetupState,
    config::VariableCryptoConfig,
    domain::{
        model::{ModelCapability, ModelProtocol, ModelType, validate_model_configuration},
        reasoning::ReasoningEffort,
        user::validate_initial_password,
    },
    shared::time::current_timestamp,
};

const SYSTEM_INITIALIZATION_MIGRATION: &str =
    include_str!("../../../migrations/20260825_0001_create_system_initialization.sql");
const MODEL_PROVIDER_MIGRATION: &str =
    include_str!("../../../migrations/20260915_0001_create_model_provider_management.sql");
const BCRYPT_COST: u32 = 4;

#[derive(Clone)]
pub struct SetupTransactionService {
    application_config_path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupCompletionCommand {
    pub new_password: String,
    pub display_name: String,
    pub encryption_key_base64: String,
    pub workspace_name: String,
    pub workspace_description: Option<String>,
    #[serde(default)]
    pub model_access: SetupModelAccess,
    pub models: Option<SetupModelCommands>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SetupModelAccess {
    #[default]
    Manual,
    PersonAi,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupModelCommands {
    pub text: SetupModelCommand,
    pub embedding: SetupModelCommand,
    pub rerank: Option<SetupModelCommand>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupModelCommand {
    pub display_name: String,
    pub vendor: String,
    pub protocol: ModelProtocol,
    pub model_type: ModelType,
    pub model_capabilities: Vec<ModelCapability>,
    pub base_url: String,
    pub api_path: Option<String>,
    pub api_key_ref: String,
    pub model_name: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub thinking_enabled: bool,
    pub supported_reasoning_efforts: Vec<ReasoningEffort>,
    pub default_reasoning_effort: Option<ReasoningEffort>,
    pub structured_output_enabled: bool,
    pub responses_stateful_enabled: bool,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub context_window_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub max_retries: Option<u32>,
    pub allow_inheritance: Option<bool>,
}

#[derive(Debug)]
pub enum SetupTransactionError {
    AlreadyCompleted,
    WorkspaceAlreadyExists,
    InvalidPassword,
    InvalidEncryptionKey,
    InvalidDisplayName,
    InvalidWorkspaceName,
    InvalidModel(String),
    PersonAiNotReady,
    AdminNotFound,
    Storage,
}

impl SetupTransactionService {
    pub fn connect(
        database_url: impl Into<String>,
        application_config_path: impl Into<PathBuf>,
    ) -> Result<Self, SetupTransactionError> {
        let database_url = database_url.into();
        let connection = open_connection(&database_url)?;
        connection
            .execute_batch(SYSTEM_INITIALIZATION_MIGRATION)
            .map_err(|_| SetupTransactionError::Storage)?;
        connection
            .execute_batch(MODEL_PROVIDER_MIGRATION)
            .map_err(|_| SetupTransactionError::Storage)?;
        migrate_provider_catalog_model_schema(&connection)
            .map_err(|_| SetupTransactionError::Storage)?;
        Ok(Self {
            application_config_path: application_config_path.into(),
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn get(&self) -> Result<SetupState, SetupTransactionError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SetupTransactionError::Storage)?;
        read_state(&connection)
    }

    pub fn complete(
        &self,
        actor_user_code: &str,
        command: SetupCompletionCommand,
    ) -> Result<SetupState, SetupTransactionError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SetupTransactionError::Storage)?;
        let tx = connection
            .transaction()
            .map_err(|_| SetupTransactionError::Storage)?;

        let state = read_state_from(&tx)?;
        if state.initialized {
            return Err(SetupTransactionError::AlreadyCompleted);
        }
        let active_workspace_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM workspaces WHERE status = 'active'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| SetupTransactionError::Storage)?;
        if active_workspace_count != 0 {
            return Err(SetupTransactionError::WorkspaceAlreadyExists);
        }

        validate_initial_password(&command.new_password)
            .map_err(|_| SetupTransactionError::InvalidPassword)?;
        if command.display_name.trim().is_empty() {
            return Err(SetupTransactionError::InvalidDisplayName);
        }
        if command.workspace_name.trim().is_empty() {
            return Err(SetupTransactionError::InvalidWorkspaceName);
        }
        let _key = decode_key(&command.encryption_key_base64)?;
        let password_hash =
            hash(&command.new_password, BCRYPT_COST).map_err(|_| SetupTransactionError::Storage)?;
        let now = current_timestamp();
        let now_millis = crate::shared::time::timestamp_to_millis(&now);
        tx.query_row(
            "SELECT id FROM users WHERE user_code = 'USR-ADMIN' AND status = 'active'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| SetupTransactionError::Storage)?
        .ok_or(SetupTransactionError::AdminNotFound)?;

        let manual_models = match command.model_access {
            SetupModelAccess::Manual => Some(command.models.as_ref().ok_or_else(|| {
                SetupTransactionError::InvalidModel(
                    "manual setup requires text and embedding models".into(),
                )
            })?),
            SetupModelAccess::PersonAi => {
                validate_person_ai_setup_ready(&tx)?;
                None
            }
        };
        if let Some(models) = manual_models {
            validate_models(models)?;
        }
        let cipher = crate::adapters::outbound::crypto::Sm4GcmVariableCipher::from_config(
            VariableCryptoConfig {
                key_base64: command.encryption_key_base64.trim().to_owned(),
            },
        )
        .map_err(|_| SetupTransactionError::InvalidEncryptionKey)?;

        tx.execute(
            "UPDATE users SET display_name = ?1, password_hash = ?2, password_updated_at = ?3,
             password_reset_required = 0, updated_at = ?3, version = version + 1
             WHERE user_code = 'USR-ADMIN' AND password_reset_required = 1",
            params![command.display_name.trim(), password_hash, now_millis],
        )
        .map_err(|_| SetupTransactionError::Storage)?;

        let workspace_id = next_id(
            &tx,
            &[
                "workspaces",
                "workspace_user_access",
                "workspace_role_access",
            ],
        )?;
        let workspace_code = generate_workspace_code();
        tx.execute(
            "INSERT INTO workspaces (id, workspace_code, name, description, parent_workspace_code,
             status, created_at, updated_at, archived_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 'active', ?5, ?5, NULL)",
            params![
                workspace_id,
                workspace_code,
                command.workspace_name.trim(),
                command
                    .workspace_description
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty()),
                now_millis
            ],
        )
        .map_err(|_| SetupTransactionError::Storage)?;
        let access_id = workspace_id + 1;
        tx.execute(
            "INSERT INTO workspace_user_access (id, workspace_code, user_code, is_owner, created_at, updated_at)
             VALUES (?1, ?2, 'USR-ADMIN', 1, ?3, ?3)",
            params![access_id, workspace_code, now_millis],
        )
        .map_err(|_| SetupTransactionError::Storage)?;

        if let Some(models) = manual_models {
            let mut model_id = next_id(&tx, &["models"])?;
            let mut setup_models = vec![
                (ModelType::Text, &models.text),
                (ModelType::Embedding, &models.embedding),
            ];
            if let Some(rerank) = models.rerank.as_ref() {
                setup_models.push((ModelType::Rerank, rerank));
            }
            for (model_type, model) in setup_models {
                let model_code = format!("MODEL-{model_id}");
                let encrypted_key = protect_api_key(&cipher, &model.api_key_ref)?;
                insert_platform_model(
                    &tx,
                    model_id,
                    &model_code,
                    model,
                    encrypted_key,
                    now_millis,
                )?;
                tx.execute(
                    "INSERT INTO platform_default_models (model_type, model_code, updated_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(model_type) DO UPDATE SET model_code = excluded.model_code, updated_at = excluded.updated_at",
                    params![model_type_storage(model_type), model_code, now_millis],
                )
                .map_err(|_| SetupTransactionError::Storage)?;
                tx.execute(
                    "INSERT INTO default_model_selections (
                        scope_key, workspace_code, model_type, selection_kind, provider_code,
                        model_code, route_code, created_at, updated_at
                     ) VALUES ('platform', NULL, ?1, 'Fixed', NULL, ?2, NULL, ?3, ?3)
                     ON CONFLICT(scope_key, model_type) DO UPDATE SET
                        selection_kind = excluded.selection_kind,
                        provider_code = NULL,
                        model_code = excluded.model_code,
                        route_code = NULL,
                        updated_at = excluded.updated_at",
                    params![model_type_storage(model_type), model_code, now_millis],
                )
                .map_err(|_| SetupTransactionError::Storage)?;
                model_id += 1;
            }
        }

        let initialized_at = Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE system_initialization SET initialized = 1, initialized_at = ?1, initialized_by = ?2 WHERE id = 1",
            params![initialized_at, actor_user_code],
        )
        .map_err(|_| SetupTransactionError::Storage)?;

        let original_config =
            fs::read(&self.application_config_path).map_err(|_| SetupTransactionError::Storage)?;
        let next_config = update_config(&original_config, &command.encryption_key_base64)?;
        atomic_write(&self.application_config_path, &next_config)?;

        if let Err(_) = tx.commit() {
            let _ = atomic_write(&self.application_config_path, &original_config);
            return Err(SetupTransactionError::Storage);
        }
        Ok(SetupState {
            initialized: true,
            initialized_at: Some(initialized_at),
            initialized_by: Some(actor_user_code.to_owned()),
        })
    }
}

fn open_connection(database_url: &str) -> Result<Connection, SetupTransactionError> {
    if database_url != ":memory:"
        && !database_url.starts_with("file:")
        && let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|_| SetupTransactionError::Storage)?;
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
    .map_err(|_| SetupTransactionError::Storage)
}

fn read_state(connection: &Connection) -> Result<SetupState, SetupTransactionError> {
    read_state_from(connection)
}

fn read_state_from(connection: &Connection) -> Result<SetupState, SetupTransactionError> {
    connection
        .query_row(
            "SELECT initialized, initialized_at, initialized_by FROM system_initialization WHERE id = 1",
            [],
            |row| {
                Ok(SetupState {
                    initialized: row.get::<_, i64>(0)? != 0,
                    initialized_at: row.get(1)?,
                    initialized_by: row.get(2)?,
                })
            },
        )
        .map_err(|_| SetupTransactionError::Storage)
}

fn next_id(connection: &Connection, tables: &[&str]) -> Result<i64, SetupTransactionError> {
    let mut max_id = 0_i64;
    for table in tables {
        let sql = format!("SELECT COALESCE(MAX(id), 0) FROM {table}");
        max_id = max_id.max(
            connection
                .query_row(&sql, [], |row| row.get(0))
                .map_err(|_| SetupTransactionError::Storage)?,
        );
    }
    Ok(max_id + 1)
}

fn generate_workspace_code() -> String {
    let mut bytes = [0_u8; 8];
    rand::rng().fill_bytes(&mut bytes);
    format!("WS-{}", hex::encode_upper(bytes))
}

fn decode_key(value: &str) -> Result<[u8; 16], SetupTransactionError> {
    BASE64
        .decode(value.trim())
        .map_err(|_| SetupTransactionError::InvalidEncryptionKey)?
        .try_into()
        .map_err(|_| SetupTransactionError::InvalidEncryptionKey)
}

fn validate_models(models: &SetupModelCommands) -> Result<(), SetupTransactionError> {
    let mut setup_models = vec![
        (ModelType::Text, &models.text),
        (ModelType::Embedding, &models.embedding),
    ];
    if let Some(rerank) = models.rerank.as_ref() {
        setup_models.push((ModelType::Rerank, rerank));
    }
    for (expected_type, model) in setup_models {
        if model.model_type != expected_type {
            return Err(SetupTransactionError::InvalidModel(
                "model type does not match setup slot".into(),
            ));
        }
        validate_model_configuration(
            &model.display_name,
            &model.vendor,
            &model.base_url,
            &model.api_key_ref,
            &model.model_name,
            model.protocol,
            model.model_type,
            &model.model_capabilities,
            model.responses_stateful_enabled,
        )
        .map_err(|error| SetupTransactionError::InvalidModel(error.to_string()))?;
        if model.model_type == ModelType::Text && model.context_window_tokens.is_none() {
            return Err(SetupTransactionError::InvalidModel(
                "contextWindowTokens is required".into(),
            ));
        }
    }
    Ok(())
}

fn validate_person_ai_setup_ready(connection: &Connection) -> Result<(), SetupTransactionError> {
    let connected: i64 = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM provider_connections
                WHERE provider_code = 'PersonAi' AND status = 'Connected'
                  AND credentials_encrypted IS NOT NULL
            )",
            [],
            |row| row.get(0),
        )
        .map_err(|_| SetupTransactionError::Storage)?;
    if connected == 0 {
        return Err(SetupTransactionError::PersonAiNotReady);
    }
    let required_defaults: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM default_model_selections
             WHERE scope_key = 'platform' AND provider_code = 'PersonAi'
               AND model_type IN ('TEXT', 'EMBEDDING')
               AND selection_kind = 'ProviderRoute'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| SetupTransactionError::Storage)?;
    if required_defaults != 2 {
        return Err(SetupTransactionError::PersonAiNotReady);
    }
    Ok(())
}

fn protect_api_key(
    cipher: &impl crate::application::variable::VariableCipher,
    value: &str,
) -> Result<String, SetupTransactionError> {
    let encrypted = cipher
        .encrypt(value.trim(), b"model.apiKeyRef")
        .map_err(|_| SetupTransactionError::Storage)?;
    serde_json::to_string(&encrypted)
        .map(|payload| format!("enc:v1:{payload}"))
        .map_err(|_| SetupTransactionError::Storage)
}

fn insert_platform_model(
    tx: &Connection,
    id: i64,
    model_code: &str,
    model: &SetupModelCommand,
    encrypted_key: String,
    now_millis: i64,
) -> Result<(), SetupTransactionError> {
    let capabilities = serde_json::to_string(&model.model_capabilities)
        .map_err(|_| SetupTransactionError::Storage)?;
    let reasoning = serde_json::to_string(&model.supported_reasoning_efforts)
        .map_err(|_| SetupTransactionError::Storage)?;
    tx.execute(
        "INSERT INTO models (id, scope_kind, workspace_code, model_code, display_name, vendor, protocol,
         model_type, model_capabilities, base_url, api_path, api_key_ref, model_name, enabled, description,
         thinking_enabled, supported_reasoning_efforts, default_reasoning_effort, structured_output_enabled,
         responses_stateful_enabled, temperature, max_tokens, context_window_tokens, top_p, max_retries,
         last_verified_at, last_verify_status, last_verify_message, source_kind, source_model_code,
         allow_inheritance, created_at, updated_at)
         VALUES (?1, 'platform', NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
         ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, NULL, NULL, NULL, 'platform', NULL, NULL, ?24, ?24)",
        params![
            id, model_code, model.display_name.trim(), model.vendor.trim(),
            protocol_storage(model.protocol), model_type_storage(model.model_type), capabilities,
            model.base_url.trim().trim_end_matches('/'), model.api_path.as_deref().map(str::trim),
            encrypted_key, model.model_name.trim(), i64::from(model.enabled),
            model.description.as_deref().map(str::trim), i64::from(model.thinking_enabled), reasoning,
            model.default_reasoning_effort.map(|value| value.storage_value()),
            i64::from(model.structured_output_enabled), i64::from(model.responses_stateful_enabled),
            model.temperature, model.max_tokens, model.context_window_tokens, model.top_p,
            model.max_retries, now_millis,
        ],
    )
    .map(|_| ())
    .map_err(|_| SetupTransactionError::Storage)
}

fn protocol_storage(value: ModelProtocol) -> &'static str {
    match value {
        ModelProtocol::OpenAiCompatible => "OPENAI_COMPATIBLE",
        ModelProtocol::OpenAiCompatibleEmbedding => "OPENAI_COMPATIBLE_EMBEDDING",
        ModelProtocol::Responses => "RESPONSES",
        ModelProtocol::Rerank => "RERANK",
    }
}

fn model_type_storage(value: ModelType) -> &'static str {
    match value {
        ModelType::Text => "TEXT",
        ModelType::Embedding => "EMBEDDING",
        ModelType::Rerank => "RERANK",
        ModelType::Image => "IMAGE",
    }
}

fn update_config(original: &[u8], key: &str) -> Result<Vec<u8>, SetupTransactionError> {
    let mut value: serde_yaml::Value =
        serde_yaml::from_slice(original).map_err(|_| SetupTransactionError::Storage)?;
    let mapping = value
        .as_mapping_mut()
        .ok_or(SetupTransactionError::Storage)?;
    let crypto = mapping
        .entry(serde_yaml::Value::String("variableCrypto".into()))
        .or_insert_with(|| serde_yaml::Value::Mapping(Default::default()));
    let crypto = crypto
        .as_mapping_mut()
        .ok_or(SetupTransactionError::Storage)?;
    crypto.insert(
        serde_yaml::Value::String("keyBase64".into()),
        serde_yaml::Value::String(key.trim().into()),
    );
    serde_yaml::to_string(&value)
        .map(|content| content.into_bytes())
        .map_err(|_| SetupTransactionError::Storage)
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), SetupTransactionError> {
    let parent = path.parent().ok_or(SetupTransactionError::Storage)?;
    let temp = parent.join(format!(".setup-transaction-{}.tmp", std::process::id()));
    fs::write(&temp, content).map_err(|_| SetupTransactionError::Storage)?;
    fs::rename(&temp, path).map_err(|_| SetupTransactionError::Storage)
}
