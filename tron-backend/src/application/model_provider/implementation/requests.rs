use super::*;

impl ModelProviderService {
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
                .map_err(|error| {
                    let error = server_api_error(error);
                    self.mark_auth_required_after_rejected_credentials(&error);
                    error.to_string()
                });
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
                .map_err(|error| {
                    let error = server_api_error(error);
                    self.mark_auth_required_after_rejected_credentials(&error);
                    error.to_string()
                });
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
            .map_err(|error| {
                self.mark_auth_required_after_rejected_credentials(&error);
                error.to_string()
            })
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
}
