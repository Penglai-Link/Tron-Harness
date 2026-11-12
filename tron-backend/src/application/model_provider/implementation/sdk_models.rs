use super::*;
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use super::encryption_keys::{is_encryption_key_failure, is_encryption_key_failure_detail};

use eliteforge_agent_official_plugin_model_http as sdk;

#[path = "sdk_model_errors.rs"]
mod model_errors;

impl ModelProviderService {
    pub fn embedding_accepts_images(&self, model: &str) -> Result<bool, ModelError> {
        let Some(code) = model.strip_prefix(PERSON_AI_EMBEDDING_PREFIX) else {
            return Ok(false);
        };
        let code = code.to_owned();
        self.connection.call(move |connection| {
            let mut statement = connection.prepare("SELECT dimension,tags_json FROM provider_multimedia_catalog_models WHERE provider_code='PersonAi' AND media_type=6 AND status='Available' AND model_code=?1")
                .map_err(|_|ModelError::StorageFailure)?;
            let rows = statement.query_map([&code], |row|Ok((row.get::<_,Option<usize>>(0)?,row.get::<_,String>(1)?)))
                .map_err(|_|ModelError::StorageFailure)?;
            let mut models = vec![];
            for row in rows {
                let (dimension,tags)=row.map_err(|_|ModelError::StorageFailure)?;
                models.push(eliteforge_agent_capability_personai::PersonAiEmbeddingModel {
                    model: eliteforge_agent_core::ModelRef::new("personai", &code), dimension,
                    tags: serde_json::from_str(&tags).unwrap_or_default(),
                });
            }
            Ok(!eliteforge_agent_capability_personai::personai_embedding_candidates(models,
                eliteforge_agent_capability_personai::PersonAiEmbeddingKind::Multimodal,None).is_empty())
        }, ||ModelError::StorageFailure)
    }

    pub fn sdk_model_authorization_scope(&self) -> Result<String, fw::ModelError> {
        let this = self.clone();
        this.connection.clone().call(
            move |connection| {
                let row = read_connection(&connection, ModelProviderCode::PersonAi)
                    .map_err(|e| fw::ModelError::Fatal(e.to_string()))?
                    .ok_or_else(|| fw::ModelError::Fatal("provider connection missing".into()))?;
                Ok(this.sdk_identity_scope(&row))
            },
            || fw::ModelError::Fatal("provider database worker stopped".into()),
        )
    }
    pub fn model_executor(&self) -> Arc<eliteforge_agent_framework::ModelClient> {
        self.model_executor.clone()
    }
    pub async fn sdk_model_transport(
        &self,
        config: &AgentModelConfig,
    ) -> Result<Arc<dyn sdk::ModelTransport>, fw::ModelError> {
        let expected_scope = {
            let this = self.clone();
            this.connection.clone().call(
                move |connection| {
                    let row = read_connection(&connection, ModelProviderCode::PersonAi)
                        .map_err(|e| fw::ModelError::Fatal(e.to_string()))?
                        .ok_or_else(|| {
                            fw::ModelError::Fatal("provider connection missing".into())
                        })?;
                    Ok(this.sdk_identity_scope(&row))
                },
                || fw::ModelError::Fatal("provider database worker stopped".into()),
            )?
        };
        let credentials = self
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
        let embedding_code = config
            .model_code
            .strip_prefix(PERSON_AI_EMBEDDING_PREFIX)
            .map(str::to_owned);
        let rerank_id = config
            .model_code
            .strip_prefix(PERSON_AI_RERANK_PREFIX)
            .map(str::to_owned);
        let (attempt, embedding_instance_id) = {
            let this = self.clone();
            let worker_credentials = credentials.clone();
            let embedding_code = embedding_code.clone();
            this.connection.clone().call(
                move |connection| {
                    let credentials = worker_credentials;
                    let row = read_connection(&connection, ModelProviderCode::PersonAi)
                        .map_err(|e| fw::ModelError::Fatal(e.to_string()))?
                        .ok_or_else(|| {
                            fw::ModelError::Fatal("provider connection missing".into())
                        })?;
                    let current = row
                        .credentials_encrypted
                        .as_deref()
                        .and_then(|value| this.decrypt_credentials(value).ok());
                    if this.sdk_identity_scope(&row) != expected_scope
                        || !current.is_some_and(|value| {
                            value.token == credentials.token
                                && value.client_id == credentials.client_id
                                && value.public_key == credentials.public_key
                                && value.server_base_url == credentials.server_base_url
                        })
                    {
                        return Err(fw::ModelError::Fatal(
                            "PersonAI profile changed before model dispatch".into(),
                        ));
                    }
                    if let Some(id) = rerank_id.as_deref() {
                        let available = connection.query_row(
                            "SELECT EXISTS(SELECT 1 FROM provider_multimedia_catalog_models WHERE provider_code='PersonAi' AND media_type=7 AND status='Available' AND model_id=?1)",
                            [id], |row| row.get::<_, bool>(0),
                        ).map_err(|_| fw::ModelError::Fatal("rerank catalog lookup failed".into()))?;
                        if !available { return Err(fw::ModelError::Fatal("PersonAI rerank model is unavailable".into())); }
                    }
                    let instance_id = if let Some(code) = embedding_code.as_deref() {
                        let ordinal = this.embedding_route_counter.fetch_add(1, Ordering::Relaxed);
                        Some(select_embedding_instance(connection, code, ordinal)
                            .map_err(|_| fw::ModelError::Fatal("embedding catalog lookup failed".into()))?
                            .ok_or_else(|| fw::ModelError::Fatal(format!(
                                "PersonAI embedding model {code} has no available instances"
                            )))?)
                    } else {
                        None
                    };
                    Ok((this.sdk_attempt_for_row(&row, &credentials.token), instance_id))
                },
                || fw::ModelError::Fatal("provider database worker stopped".into()),
            )?
        };
        let (conversation,run) = crate::adapters::outbound::model::person_ai_model_provider_client::person_ai_runtime_metadata(config);
        let quality = person_ai_runtime_quality(config)
            .map(provider_quality_to_sdk)
            .unwrap_or(QualityType::High);
        let transport = sdk::PersonAiModelTransport::new(
            credentials.server_base_url.clone(),
            credentials.context(),
            quality,
            conversation.map(str::to_owned),
        )?;
        Ok(Arc::new(SnapshotTransport {
            inner: Arc::new(transport),
            providers: self.clone(),
            config: config.clone(),
            run: run.map(str::to_owned),
            attempt,
            embedding_instance_id,
            embedding_logical_code: embedding_code,
        }))
    }
    pub async fn sdk_recover_model_authentication(
        &self,
        error: &fw::ModelError,
    ) -> Result<bool, fw::ModelError> {
        let id = match error {
            fw::ModelError::Provider(error) => error.authorization_attempt.as_deref(),
            _ => None,
        }
        .ok_or_else(|| {
            fw::ModelError::Fatal("PersonAI rejected attempt identity missing".into())
        })?;

        if is_encryption_key_failure(error) {
            return match self.recover_encryption_key_for_match(id).await {
                Ok(Some(_)) => Ok(true),
                Ok(None) => {
                    let mut failure = error.clone();
                    if let fw::ModelError::Provider(detail) = &mut failure {
                        detail.message = "PersonAI encryption key changed; authorize PersonAI again".into();
                    }
                    Err(failure)
                }
                Err(failure) => Err(fw::ModelError::Fatal(failure.to_string())),
            };
        }

        match self
            .refresh_credentials_for_match(ModelProviderCode::PersonAi, id, true, true)
            .await
        {
            Ok(_) => Ok(true),
            Err(error) => Err(fw::ModelError::Fatal(error.to_string())),
        }
    }
    pub fn sdk_final_model_failure(&self, error: &fw::ModelError) {
        self.sdk_apply_scoped_failure(error, true);
    }
    fn sdk_model_failure(&self, error: &fw::ModelError) {
        self.sdk_apply_scoped_failure(error, false);
    }
    fn sdk_apply_scoped_failure(&self, error: &fw::ModelError, terminal: bool) {
        let fw::ModelError::Provider(error) = error else {
            return;
        };
        let Some(identity) = error.authorization_attempt.as_deref() else {
            return;
        };
        let unbound = error.category == fw::ProviderErrorCategory::AccountChanged
            && error.error_type.as_deref() == Some("device_unbound");
        let rejected = unbound
            || error.category == fw::ProviderErrorCategory::AccountChanged
            || (terminal && error.category == fw::ProviderErrorCategory::Authentication);
        if !rejected {
            return;
        }
        let rejected_key = terminal && is_encryption_key_failure_detail(error);
        let rejected_access =
            terminal && error.category == fw::ProviderErrorCategory::Authentication && !rejected_key;
        let identity = identity.to_owned();
        let result = (|| -> Result<(), ModelProviderError> {
            let this = self.clone();
            this.connection.clone().call(move |connection| {
            let transaction = crate::adapters::outbound::persistence::sqlite::framework_storage::business_transaction(&connection, rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| ModelProviderError::Storage)?;
            let Some(active) = read_connection(&transaction, ModelProviderCode::PersonAi)? else {
                return Ok(());
            };
            let Some(encrypted) = active.credentials_encrypted.as_deref() else {
                return Ok(());
            };
            let credentials = this.decrypt_credentials(encrypted)?;
            if this.sdk_attempt_for_row(&active, &credentials.token) != identity {
                return Ok(());
            }
            let now = now_millis();
            if rejected_access {
                // A model call can exhaust its retry budget before refresh runs.
                // Rejecting an access token alone does not revoke the refresh token.
                if !matches!(active.status, ProviderConnectionStatus::Connected | ProviderConnectionStatus::Degraded) {
                    return Ok(());
                }
                let mut retryable = credentials;
                retryable.authorization_expires_at = Some(0);
                let encrypted = this.encrypt_credentials(&retryable)?;
                let reason = "PersonAI access token rejected; refresh pending";
                transaction.execute("UPDATE provider_connections SET status=?2,last_error=?3,authorization_expires_at=0,credentials_encrypted=?4,updated_at=?5 WHERE provider_code=?1", params![ModelProviderCode::PersonAi.storage_value(),ProviderConnectionStatus::Degraded.storage_value(),reason,encrypted,now]).map_err(|_| ModelProviderError::Storage)?;
                if let Some(user) = active.person_ai_user_id.as_deref() {
                    transaction.execute("UPDATE person_ai_profile_credentials SET status=?2,last_error=?3,authorization_expires_at=0,credentials_encrypted=?4,updated_at=?5 WHERE person_ai_user_id=?1",params![user,ProviderConnectionStatus::Degraded.storage_value(),reason,encrypted,now]).map_err(|_| ModelProviderError::Storage)?;
                }
                transaction.commit().map_err(|_| ModelProviderError::Storage)?;
                return Ok(());
            }
            let reason = if unbound {
                "PersonAI device was unbound; login again"
            } else if rejected_key {
                "PersonAI encryption key changed; authorize PersonAI again"
            } else {
                "PersonAI rejected access credentials; reauthorization required"
            };
            transaction.execute("UPDATE provider_connections SET status=?2,last_error=?3,updated_at=?4,credentials_encrypted=CASE WHEN ?5 THEN NULL ELSE credentials_encrypted END,authorization_expires_at=CASE WHEN ?5 THEN NULL ELSE authorization_expires_at END WHERE provider_code=?1",params![ModelProviderCode::PersonAi.storage_value(),ProviderConnectionStatus::AuthRequired.storage_value(),reason,now,unbound]).map_err(|_|ModelProviderError::Storage)?;
            if let Some(user) = active.person_ai_user_id.as_deref() {
                transaction.execute("UPDATE person_ai_profile_credentials SET status=?2,last_error=?3,updated_at=?4,credentials_encrypted=CASE WHEN ?5 THEN '' ELSE credentials_encrypted END,authorization_expires_at=CASE WHEN ?5 THEN NULL ELSE authorization_expires_at END WHERE person_ai_user_id=?1",params![user,ProviderConnectionStatus::AuthRequired.storage_value(),reason,now,unbound]).map_err(|_|ModelProviderError::Storage)?;
            }
            if unbound {
                this.invalidate_unbound_catalog(&transaction, now)?;
            }
            transaction
                .commit()
                .map_err(|_| ModelProviderError::Storage)?;
            if unbound {
                this.device_unbound.store(true, Ordering::SeqCst);
                let _ = this.server_auth_events.send(());
            }
            Ok(())
         },||ModelProviderError::Storage)
        })();
        if let Err(error) = result {
            tracing::error!("[person-ai] scoped credential failure persistence failed: {error}");
        }
    }
}
struct SnapshotTransport {
    inner: Arc<dyn sdk::ModelTransport>,
    providers: ModelProviderService,
    config: AgentModelConfig,
    run: Option<String>,
    attempt: String,
    embedding_instance_id: Option<String>,
    embedding_logical_code: Option<String>,
}
#[async_trait]
impl sdk::ModelTransport for SnapshotTransport {
    async fn json(&self, path: &str, body: &Value) -> Result<Value, fw::ModelError> {
        let routed_body = if path.trim_matches('/') == "chat/completions" {
            Some(
                eliteforge_agent_capability_personai::personai_generation_request_body(
                    body,
                    self.config.thinking_enabled,
                ),
            )
        } else if let Some(instance_id) = self.embedding_instance_id.as_deref() {
            let mut routed_body = body.clone();
            let object = routed_body.as_object_mut().ok_or_else(|| {
                fw::ModelError::Fatal("embedding request body must be an object".into())
            })?;
            object.insert("model".into(), Value::String(instance_id.to_owned()));
            Some(routed_body)
        } else if let Some(id) = self.config.model_code.strip_prefix(PERSON_AI_RERANK_PREFIX) {
            let mut routed_body = body.clone();
            let object = routed_body.as_object_mut().ok_or_else(|| {
                fw::ModelError::Fatal("rerank request body must be an object".into())
            })?;
            object.remove("model");
            object.insert("model_id".into(), Value::String(id.to_owned()));
            Some(routed_body)
        } else {
            None
        };
        let response = self
            .inner
            .json(path, routed_body.as_ref().unwrap_or(body))
            .await;
        match response {
            Ok(response) => {
                self.providers
                    .record_runtime_snapshot(self.run.as_deref(), "", &self.config, &response)
                    .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
                if let Some(error) = model_errors::output_limit_failure(
                    &response,
                    routed_body.as_ref().unwrap_or(body),
                ) {
                    return Err(error);
                }
                if let Some(logical_code) = self.embedding_logical_code.as_deref() {
                    let mut normalized = response;
                    if let Some(object) = normalized.as_object_mut() {
                        object.insert("model".into(), Value::String(logical_code.to_owned()));
                    }
                    Ok(normalized)
                } else {
                    Ok(response)
                }
            }
            Err(error) => Err(self.failure(error)),
        }
    }
    async fn events(
        &self,
        path: &str,
        body: &Value,
        sink: sdk::EventSink<'_>,
    ) -> Result<(), fw::ModelError> {
        let routed_body = if path.trim_matches('/') == "chat/completions" {
            Some(
                eliteforge_agent_capability_personai::personai_generation_request_body(
                    body,
                    self.config.thinking_enabled,
                ),
            )
        } else {
            None
        };
        let metadata = std::sync::Mutex::new(super::snapshots::RuntimeSnapshotMetadata::default());
        let result = self
            .inner
            .events(
                path,
                routed_body.as_ref().unwrap_or(body),
                Arc::new(|event| {
                    {
                        let mut saved = metadata
                            .lock()
                            .map_err(|_| fw::ModelError::Fatal("snapshot lock poisoned".into()))?;
                        let mut next = saved.clone();
                        next.merge(&event);
                        if next != *saved {
                            self.providers
                                .record_runtime_snapshot(
                                    self.run.as_deref(),
                                    "",
                                    &self.config,
                                    &next.value(),
                                )
                                .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
                            *saved = next;
                        }
                    }
                    if let Some(error) = model_errors::output_limit_failure(
                        &event,
                        routed_body.as_ref().unwrap_or(body),
                    ) {
                        return Err(error);
                    }
                    sink(event)
                }),
            )
            .await;
        result.map_err(|error| self.failure(error))
    }
}

impl SnapshotTransport {
    fn failure(&self, error: fw::ModelError) -> fw::ModelError {
        let mut error = model_errors::classify_upstream_failure(error);
        if let fw::ModelError::Provider(detail) = &mut error {
            detail.authorization_attempt = Some(self.attempt.clone());
        }
        self.providers.sdk_model_failure(&error);
        error
    }
}

impl ModelProviderService {
    fn sdk_fingerprint(&self, value: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(self.model_attempt_salt.as_bytes());
        digest.update([0]);
        digest.update(value.as_bytes());
        format!("{:x}", digest.finalize())
    }
    pub(super) fn sdk_identity_scope(&self, row: &ProviderConnectionRow) -> String {
        use sha2::{Digest, Sha256};
        // Account identity survives process restarts and credential refreshes.
        // Credential attempts below retain their process-local salted digest.
        let mut digest = Sha256::new();
        digest.update(b"tron.provider.account.v1\0");
        digest.update(serde_json::json!([
                row.provider_code.storage_value(),
                row.person_ai_user_id,
                row.server_base_url
            ]).to_string().as_bytes());
        format!("{:x}", digest.finalize())
    }
    pub(super) fn sdk_attempt_for_row(&self, row: &ProviderConnectionRow, token: &str) -> String {
        // A device reauthorization can rotate the encryption key while the
        // access token stays valid. Late failures must identify that key too.
        let credentials = row.credentials_encrypted.as_deref()
            .and_then(|encrypted| self.decrypt_credentials(encrypted).ok());
        let material = serde_json::json!([
            token,
            credentials.as_ref().map(|value| &value.client_id),
            credentials.as_ref().map(|value| &value.public_key),
        ]).to_string();
        format!(
            "{}:{}",
            self.sdk_identity_scope(row),
            self.sdk_fingerprint(&material)
        )
    }

}
