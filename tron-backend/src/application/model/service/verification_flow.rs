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
        let native_chat = matches!(
            model.protocol,
            ModelProtocol::Anthropic | ModelProtocol::Gemini
        );
        let config = sdk::AgentModelConfig {
            request_headers: model.request_headers.clone(),
            model_code: "probe".into(),
            protocol: match model.protocol {
                ModelProtocol::Responses => sdk::AgentModelProtocol::Responses,
                ModelProtocol::Anthropic => sdk::AgentModelProtocol::Anthropic,
                ModelProtocol::Gemini => sdk::AgentModelProtocol::Gemini,
                ModelProtocol::OpenAiCompatibleEmbedding => sdk::AgentModelProtocol::Embeddings,
                _ => sdk::AgentModelProtocol::ChatCompletions,
            },
            base_url: model.base_url.clone(),
            api_path: if native_chat {
                None
            } else {
                Some(endpoint_path)
            },
            api_key,
            model_name: model.model_name.clone(),
            thinking_enabled: false,
            supports_developer_role: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: Some(MIN_VERIFY_OUTPUT_TOKENS),
            context_window_tokens: model.context_window_tokens,
            top_p: None,
            max_retries: model.max_retries,
        };
        let factory = self.verification_factory.clone();
        let descriptor = sdk::model_descriptor(&config);
        let adapter: Arc<dyn fw::ChatModel> = if native_chat {
            Arc::new(sdk::ProtocolChatModel {
                config,
                factory,
                streaming: false,
            })
        } else {
            Arc::new(sdk::HttpProbeModel { config, factory })
        };
        let connection = fw::ModelConnection::new("probe", adapter, vec![descriptor]);
        let runtime = eliteforge_agent_framework::ModelRuntime::new(
            eliteforge_agent_framework::ModelRuntimeUpdate::new("probe", 0, vec![connection]),
        )
        .map_err(|error| ModelError::VerificationFailed(error.to_string()))?;
        let request = fw::ChatRequest {
            messages: if native_chat {
                vec![fw::Message::user("Respond with OK.")]
            } else {
                vec![fw::Message::user(fw::Content(vec![
                    fw::ContentBlock::Json { value: payload },
                ]))]
            },
            tools: vec![],
            extensions: Default::default(),
            options: fw::GenerationOptions {
                max_output_tokens: Some(MIN_VERIFY_OUTPUT_TOKENS),
                ..Default::default()
            },
        };
        let timeouts = self
            .runtime_config
            .as_ref()
            .map(|runtime| runtime.snapshot().model_invocation.verification)
            .unwrap_or_else(|| crate::config::ModelInvocationConfig::default().verification);
        let limits = fw::RunLimits {
            max_model_output_tokens: MIN_VERIFY_OUTPUT_TOKENS,
            max_model_retries: model.max_retries.unwrap_or(0) as usize,
            max_model_calls_per_step: Some(model.max_retries.unwrap_or(0) as usize + 1),
            model_timeout: std::time::Duration::from_secs(timeouts.call_timeout_seconds),
            run_timeout: std::time::Duration::from_secs(timeouts.operation_timeout_seconds),
            ..Default::default()
        };
        let options = fw::ModelInvocationOptions::for_call(&limits)
            .map_err(|error| ModelError::VerificationFailed(error.to_string()))?;
        self.model_executor
            .with_runtime(runtime)
            .with_options(options)
            .chat(
                &fw::ModelRef::new("probe", "probe"),
                request,
                &limits,
                "model.verify",
            )
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
