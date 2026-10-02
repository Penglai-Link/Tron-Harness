use super::*;

impl ModelService {
    async fn verify_model_definition(&self, model: &Model) -> Result<VerifyOutcome, ModelError> {
        let endpoint_path = model
            .api_path
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_verify_path(model.protocol, model.model_type).into());
        let url = join_verify_url(&model.base_url, &endpoint_path)?;
        let payload = verify_request_payload(model.protocol, model.model_type, &model.model_name);
        use eliteforge_agent_core as fw;
        use eliteforge_agent_official_plugin_model_http as sdk;
        let api_key = self.resolve_api_key_value(model)?;
        let method_label = format!("POST {}", url.path());
        let config = sdk::AgentModelConfig {
            model_code: model.model_code.clone(),
            protocol: sdk::AgentModelProtocol::ChatCompletions,
            base_url: model.base_url.clone(),
            api_path: Some(endpoint_path),
            api_key,
            model_name: model.model_name.clone(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: Some(MIN_VERIFY_OUTPUT_TOKENS),
            context_window_tokens: model.context_window_tokens,
            top_p: None,
            max_retries: model.max_retries,
        };
        let factory = self.verification_factory.clone();
        let job = eliteforge_agent_framework::ChatModelJob {
            workflow: None,
            connection: fw::ModelConnection::new(
                "probe",
                Arc::new(sdk::HttpProbeModel { config, factory }),
                vec![fw::ModelDescriptor::new("probe")],
            ),
            model: fw::ModelRef::new("probe", "probe"),
            request: fw::ChatRequest {
                messages: vec![fw::Message::user(fw::Content(vec![
                    fw::ContentBlock::Json { value: payload },
                ]))],
                tools: vec![],
                extensions: Default::default(),
                options: fw::GenerationOptions {
                    max_output_tokens: Some(MIN_VERIFY_OUTPUT_TOKENS),
                    ..Default::default()
                },
            },
            limits: fw::RunLimits {
                max_model_output_tokens: MIN_VERIFY_OUTPUT_TOKENS,
                max_model_retries: model.max_retries.unwrap_or(0) as usize,
                model_timeout: std::time::Duration::from_secs(30),
                run_timeout: std::time::Duration::from_secs(30),
                ..Default::default()
            },
            stage: "model.verify".into(),
            max_calls: model.max_retries.unwrap_or(0) as usize + 1,
            repair: None,
        };
        self.model_jobs
            .get()
            .await
            .map_err(|error| ModelError::VerificationFailed(error.to_string()))?
            .execute(job)
            .await
            .map_err(|error| {
                ModelError::VerificationFailed(match error.primary() {
                    fw::AgentError::Model(fw::ModelError::Provider(detail))
                        if detail.http_status.is_some() =>
                    {
                        format!(
                            "{method_label} returned {}: {}",
                            reqwest::StatusCode::from_u16(detail.http_status.unwrap())
                                .expect("provider HTTP status"),
                            truncate_message(&detail.message)
                        )
                    }
                    _ => format!(
                        "{method_label} failed: {}",
                        truncate_message(&error.to_string())
                    ),
                })
            })?;
        Ok(VerifyOutcome {
            message: format!("Verified via {method_label}"),
            status: ModelVerifyStatus::Succeeded,
        })
    }

    fn resolve_api_key_value(&self, model: &Model) -> Result<String, ModelError> {
        let reference = model.api_key_ref.trim();
        if reference.is_empty() {
            return Err(ModelError::ApiKeyRefRequired);
        }
        self.reveal_api_key_ref(reference)
    }

    pub(super) async fn verify_model_and_persist(&self, model: Model) -> Result<Model, ModelError> {
        match self.verify_model_definition(&model).await {
            Ok(outcome) => self.apply_verify_outcome(model, outcome),
            Err(ModelError::VerificationFailed(message)) => {
                self.apply_verify_outcome(
                    model,
                    VerifyOutcome {
                        message: message.clone(),
                        status: ModelVerifyStatus::Failed,
                    },
                )?;
                Err(ModelError::VerificationFailed(message))
            }
            Err(error) => Err(error),
        }
    }

    pub(super) async fn verify_model_preview(&self, model: Model) -> Result<Model, ModelError> {
        match self.verify_model_definition(&model).await {
            Ok(outcome) => Ok(apply_verify_outcome_to_model(model, outcome)),
            Err(error) => Err(error),
        }
    }

    fn apply_verify_outcome(
        &self,
        model: Model,
        outcome: VerifyOutcome,
    ) -> Result<Model, ModelError> {
        let model = apply_verify_outcome_to_model(model, outcome);
        self.repository
            .lock()
            .expect("model repository mutex poisoned")
            .update_model(&model)?;
        Ok(model)
    }
}
