use super::*;
use async_trait::async_trait;
use eliteforge_agent_core as fw;

use eliteforge_agent_official_plugin_model_http as sdk;

impl ModelProviderService {
    pub fn sdk_model_authorization_scope(&self) -> Result<String, fw::ModelError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| fw::ModelError::Fatal("credential storage unavailable".into()))?;
        let row = read_connection(&connection, ModelProviderCode::PersonAi)
            .map_err(|e| fw::ModelError::Fatal(e.to_string()))?
            .ok_or_else(|| fw::ModelError::Fatal("provider connection missing".into()))?;
        Ok(self.sdk_identity_scope(&row))
    }
    pub fn model_executor(&self) -> Arc<eliteforge_agent_framework::ModelClient> {
        self.model_executor.clone()
    }
    pub async fn sdk_model_transport(
        &self,
        config: &AgentModelConfig,
    ) -> Result<Arc<dyn sdk::ModelTransport>, fw::ModelError> {
        let expected_scope = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| fw::ModelError::Fatal("credential storage unavailable".into()))?;
            let row = read_connection(&connection, ModelProviderCode::PersonAi)
                .map_err(|e| fw::ModelError::Fatal(e.to_string()))?
                .ok_or_else(|| fw::ModelError::Fatal("provider connection missing".into()))?;
            self.sdk_identity_scope(&row)
        };
        let credentials = self
            .active_credentials(ModelProviderCode::PersonAi)
            .await
            .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
        let attempt = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| fw::ModelError::Fatal("credential storage unavailable".into()))?;
            let row = read_connection(&connection, ModelProviderCode::PersonAi)
                .map_err(|e| fw::ModelError::Fatal(e.to_string()))?
                .ok_or_else(|| fw::ModelError::Fatal("provider connection missing".into()))?;
            let current = row
                .credentials_encrypted
                .as_deref()
                .and_then(|value| self.decrypt_credentials(value).ok());
            if self.sdk_identity_scope(&row) != expected_scope
                || !current.is_some_and(|value| {
                    value.token == credentials.token
                        && value.server_base_url == credentials.server_base_url
                })
            {
                return Err(fw::ModelError::Fatal(
                    "PersonAI profile changed before model dispatch".into(),
                ));
            }
            self.sdk_attempt_for_row(&row, &credentials.token)
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
        let result = (|| -> Result<(), ModelProviderError> {
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| ModelProviderError::Storage)?;
            let transaction = connection
                .transaction()
                .map_err(|_| ModelProviderError::Storage)?;
            let Some(active) = read_connection(&transaction, ModelProviderCode::PersonAi)? else {
                return Ok(());
            };
            let Some(encrypted) = active.credentials_encrypted.as_deref() else {
                return Ok(());
            };
            let credentials = self.decrypt_credentials(encrypted)?;
            if self.sdk_attempt_for_row(&active, &credentials.token) != identity {
                return Ok(());
            }
            let now = now_millis();
            let reason = if unbound {
                "PersonAI device was unbound; login again"
            } else {
                "PersonAI rejected access credentials; reauthorization required"
            };
            transaction.execute("UPDATE provider_connections SET status=?2,last_error=?3,updated_at=?4,credentials_encrypted=CASE WHEN ?5 THEN NULL ELSE credentials_encrypted END,authorization_expires_at=CASE WHEN ?5 THEN NULL ELSE authorization_expires_at END WHERE provider_code=?1",params![ModelProviderCode::PersonAi.storage_value(),ProviderConnectionStatus::AuthRequired.storage_value(),reason,now,unbound]).map_err(|_|ModelProviderError::Storage)?;
            if let Some(user) = active.person_ai_user_id.as_deref() {
                transaction.execute("UPDATE person_ai_profile_credentials SET status=?2,last_error=?3,updated_at=?4,credentials_encrypted=CASE WHEN ?5 THEN '' ELSE credentials_encrypted END,authorization_expires_at=CASE WHEN ?5 THEN NULL ELSE authorization_expires_at END WHERE person_ai_user_id=?1",params![user,ProviderConnectionStatus::AuthRequired.storage_value(),reason,now,unbound]).map_err(|_|ModelProviderError::Storage)?;
            }
            if unbound {
                self.invalidate_unbound_catalog(&transaction, now)?;
            }
            transaction
                .commit()
                .map_err(|_| ModelProviderError::Storage)?;
            if unbound {
                self.device_unbound.store(true, Ordering::SeqCst);
                let _ = self.server_auth_events.send(());
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("[person-ai] scoped credential failure persistence failed: {error}");
        }
    }
}
struct SnapshotTransport {
    inner: Arc<dyn sdk::ModelTransport>,
    providers: ModelProviderService,
    config: AgentModelConfig,
    run: Option<String>,
    attempt: String,
}
#[async_trait]
impl sdk::ModelTransport for SnapshotTransport {
    async fn json(&self, path: &str, body: &Value) -> Result<Value, fw::ModelError> {
        let response = self.inner.json(path, body).await;
        match response {
            Ok(response) => {
                self.providers
                    .record_runtime_snapshot(self.run.as_deref(), "", &self.config, &response)
                    .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
                Ok(response)
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
        let result = self
            .inner
            .events(
                path,
                body,
                Arc::new(|event| {
                    self.providers
                        .record_runtime_snapshot(self.run.as_deref(), "", &self.config, &event)
                        .map_err(|e| fw::ModelError::Fatal(e.to_string()))?;
                    sink(event)
                }),
            )
            .await;
        result.map_err(|error| self.failure(error))
    }
}

impl ModelProviderService {
    pub(super) async fn verify_sdk_model_credentials(
        &self,
        credentials: &ProviderCredentials,
    ) -> Result<(), ModelProviderError> {
        let transport = sdk::PersonAiModelTransport::new(
            credentials.server_base_url.clone(),
            credentials.context(),
            QualityType::High,
            None,
        )
        .map_err(|e| ModelProviderError::Remote(e.to_string()))?;
        let factory: Arc<dyn sdk::ModelTransportFactory> =
            Arc::new(sdk::FixedModelTransportFactory(Arc::new(transport)));
        let config = person_ai_agent_config(
            "PERSON_AI:ROUTE:ChatAuto",
            "",
            None,
            None,
            AgentModelProtocol::ChatCompletions,
        );
        let mut config: sdk::AgentModelConfig =
            crate::adapters::outbound::model::http_model_client::convert(&config)
                .map_err(ModelProviderError::Remote)?;
        config.max_tokens = Some(8);
        let request = sdk::ModelRequest {
            messages: vec![sdk::AgentMessage::user("Reply with OK.")],
            tools: vec![],
            prepared_tool_schema: None,
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        };
        let connection = fw::ModelConnection::new(
            "verify",
            Arc::new(sdk::ProtocolChatModel {
                config: config.clone(),
                factory: factory.clone(),
                streaming: false,
            }),
            vec![fw::ModelDescriptor::new("chat")],
        );
        let runtime = eliteforge_agent_framework::ModelRuntime::new(
            eliteforge_agent_framework::ModelRuntimeUpdate::new("verify", 0, vec![connection]),
        )
        .map_err(sdk_provider_error)?;
        let limits = fw::RunLimits {
            max_model_output_tokens: 8,
            max_model_retries: 0,
            max_model_calls_per_step: Some(1),
            ..Default::default()
        };
        self.model_executor
            .with_runtime(runtime)
            .chat(
                &fw::ModelRef::new("verify", "chat"),
                sdk::request_to_framework(request)
                    .map_err(|e| ModelProviderError::Remote(e.to_string()))?,
                &limits,
                "provider.verify_chat",
            )
            .await
            .map_err(sdk_provider_error)?;
        let embedding_config = person_ai_agent_config(
            "PERSON_AI:ROUTE:EmbeddingAuto",
            "",
            None,
            None,
            AgentModelProtocol::Embeddings,
        );
        let config =
            crate::adapters::outbound::model::http_model_client::convert(&embedding_config)
                .map_err(ModelProviderError::Remote)?;
        let service = fw::EmbeddingModelService::new(
            fw::ModelRef::new("verify", "embedding"),
            Arc::new(sdk::HttpEmbeddingModel { config, factory }),
        );
        let mut request =
            fw::EmbeddingRequest::texts(["Tron Person-AI embedding connectivity check"]);
        request.choice = fw::ModelChoice::model(service.model.clone());
        let mut update = eliteforge_agent_framework::ModelRuntimeUpdate::new("verify", 0, vec![]);
        update.embedding.push(service);
        let runtime =
            eliteforge_agent_framework::ModelRuntime::new(update).map_err(sdk_provider_error)?;
        self.model_executor
            .with_runtime(runtime)
            .embed_observed(
                request,
                &fw::RunLimits {
                    max_model_retries: 0,
                    max_model_calls_per_step: Some(1),
                    ..Default::default()
                },
            )
            .await
            .into_result()
            .map_err(sdk_provider_error)?;
        Ok(())
    }
}
fn sdk_provider_error(error: fw::AgentError) -> ModelProviderError {
    match error.primary() {
        fw::AgentError::Model(fw::ModelError::Provider(error))
            if error.category == fw::ProviderErrorCategory::AccountChanged
                && error.error_type.as_deref() == Some("device_unbound") =>
        {
            ModelProviderError::DeviceUnbound
        }
        fw::AgentError::Model(fw::ModelError::Provider(error))
            if error.category == fw::ProviderErrorCategory::Authentication =>
        {
            ModelProviderError::Unauthorized
        }
        _ => ModelProviderError::Remote(error.to_string()),
    }
}

impl SnapshotTransport {
    fn failure(&self, mut error: fw::ModelError) -> fw::ModelError {
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
        self.sdk_fingerprint(
            &serde_json::json!([
                row.provider_code.storage_value(),
                row.person_ai_user_id,
                row.server_base_url
            ])
            .to_string(),
        )
    }
    pub(super) fn sdk_attempt_for_row(&self, row: &ProviderConnectionRow, token: &str) -> String {
        format!(
            "{}:{}",
            self.sdk_identity_scope(row),
            self.sdk_fingerprint(token)
        )
    }
    #[cfg(test)]
    pub(super) fn sdk_credential_attempt(&self, token: &str) -> String {
        let connection = self.connection.lock().unwrap();
        let row = read_connection(&connection, ModelProviderCode::PersonAi)
            .unwrap()
            .unwrap();
        self.sdk_attempt_for_row(&row, token)
    }
}

#[cfg(test)]
#[path = "sdk_models_tests.rs"]
mod tests;
