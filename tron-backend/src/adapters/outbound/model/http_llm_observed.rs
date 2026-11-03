//! Observation is a returned SDK report, not an extra statistics pipeline.
use super::*;
use eliteforge_agent_framework::{ModelExecutionReport, ObservedModelResult};

fn failure<T>(message: impl Into<String>) -> ObservedModelResult<T> {
    ObservedModelResult {
        result: Err(fw::AgentError::InvalidConfiguration(message.into())),
        report: ModelExecutionReport::default(),
    }
}
impl HttpLlmRuntimeClient {
    pub(super) async fn observed_generate(
        &self,
        config: &RuntimeModelConfig,
        prompt: &str,
    ) -> ObservedModelResult<String> {
        if config.model_type != ModelType::Text {
            return failure("text generation requires TEXT model");
        }
        let request = ModelRequest {
            messages: vec![AgentMessage::user(prompt)],
            tools: vec![],
            prepared_tool_schema: None,
            system_agent_session_id: None,
            response_input_items: None,
            previous_response_id: None,
        };
        let observed = self
            .models
            .generate_observed(&runtime_to_agent_model_config(config), &request)
            .await;
        ObservedModelResult {
            result: observed.result.and_then(|response| {
                if response.finish == ModelFinish::Final {
                    Ok(response.output_text())
                } else {
                    Err(fw::AgentError::Model(fw::ModelError::Fatal(
                        "text generation unexpectedly requested tool calls".into(),
                    )))
                }
            }),
            report: observed.report,
        }
    }
    pub(super) async fn observed_embed(
        &self,
        config: &RuntimeModelConfig,
        input: &str,
    ) -> ObservedModelResult<(Vec<f32>, Option<String>)> {
        self.observed_embed_batch(config, &[input.to_owned()], None)
            .await
            .map(|response| {
                (
                    response
                        .vectors
                        .into_iter()
                        .next()
                        .expect("validated nonempty batch")
                        .values,
                    response.provider_model,
                )
            })
    }
    pub(super) async fn observed_embed_batch(
        &self,
        config: &RuntimeModelConfig,
        inputs: &[String],
        dimensions: Option<usize>,
    ) -> ObservedModelResult<fw::EmbeddingResponse> {
        let contents = inputs
            .iter()
            .cloned()
            .map(fw::Content::from)
            .collect::<Vec<_>>();
        self.observed_embed_content_batch(config, &contents, dimensions)
            .await
    }
    pub(super) async fn observed_embed_content_batch(
        &self,
        config: &RuntimeModelConfig,
        inputs: &[fw::Content],
        dimensions: Option<usize>,
    ) -> ObservedModelResult<fw::EmbeddingResponse> {
        if !matches!(
            config.model_type,
            ModelType::TextEmbedding | ModelType::MultimodalEmbedding
        ) {
            return failure("embedding requires an embedding model");
        }
        let model = runtime_to_agent_model_config(config);
        let routers = self.pool_routers(config);
        let (runtime, choice, _) = match self.auxiliary_pool(config, dimensions, false) {
            Ok(pool) => pool,
            Err(error) => {
                return ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let mut request = fw::EmbeddingRequest::texts(std::iter::empty::<String>());
        request.inputs = inputs.to_vec();
        request.dimensions = dimensions;
        request.choice = choice;
        let config = self.models.invocation_config().embedding;
        let effective = limits(
            &model,
            &config.timeouts(),
            self.models.invocation_config().default_output_tokens,
        );
        let mut options = match fw::ModelInvocationOptions::for_call(&effective) {
            Ok(options) => options,
            Err(error) => return failure(error.to_string()),
        };
        options.max_batch_size = config.max_batch_inputs;
        let observed = self
            .models
            .executor
            .with_runtime(runtime)
            .with_auto_routers(routers)
            .with_options(options)
            .embed_observed(request, &effective)
            .await;
        ObservedModelResult {
            result: observed.result.and_then(|response| {
                response.validate(inputs.len(), dimensions)?;
                Ok(response)
            }),
            report: observed.report,
        }
    }
    pub(super) async fn observed_rerank(
        &self,
        config: &RuntimeModelConfig,
        query: &str,
        documents: &[String],
        top_n: Option<usize>,
    ) -> ObservedModelResult<Vec<LlmRuntimeRerankHit>> {
        let documents = documents
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<fw::Content>>();
        self.observed_rerank_content(config, &query.into(), &documents, top_n)
            .await
    }
    pub(super) async fn observed_rerank_content(
        &self,
        config: &RuntimeModelConfig,
        query: &fw::Content,
        documents: &[fw::Content],
        top_n: Option<usize>,
    ) -> ObservedModelResult<Vec<LlmRuntimeRerankHit>> {
        if !matches!(
            config.model_type,
            ModelType::TextRerank | ModelType::MultimodalRerank
        ) {
            return failure("rerank requires RERANK model");
        }
        if top_n == Some(0) {
            return failure("rerank top_n must be positive");
        }
        if documents.is_empty() {
            return ObservedModelResult {
                result: Ok(vec![]),
                report: Default::default(),
            };
        }
        let model = runtime_to_agent_model_config(config);
        let routers = self.pool_routers(config);
        let (runtime, choice, adapters) = match self.auxiliary_pool(config, None, true) {
            Ok(pool) => pool,
            Err(error) => {
                return ObservedModelResult {
                    result: Err(error),
                    report: Default::default(),
                };
            }
        };
        let request = fw::RerankRequest {
            choice,
            query: query.clone(),
            documents: documents.to_vec(),
            top_n: top_n.map(|n| n.min(documents.len())),
        };
        let invocation = self.models.invocation_config();
        let effective = limits(&model, &invocation.rerank, invocation.default_output_tokens);
        let options = match fw::ModelInvocationOptions::for_call(&effective) {
            Ok(options) => options,
            Err(error) => return failure(error.to_string()),
        };
        // Native rerank scores each query/document pair independently. Keep
        // the same scoped adapter for all batches and merge their global top-k.
        let capacity = adapters
            .iter()
            .filter_map(|adapter| fw::RerankModel::input_capacity(adapter.as_ref()))
            .min()
            .unwrap_or(effective.max_context_tokens)
            .min(effective.max_context_tokens);
        let batch_request = |range: std::ops::Range<usize>| {
            let mut batch = request.clone();
            batch.documents = request.documents[range].to_vec();
            batch.top_n = top_n.map(|n| n.min(batch.documents.len()));
            batch
        };
        let mut batches = Vec::new();
        let mut start = 0;
        while start < documents.len() {
            let mut end = start;
            while end < documents.len()
                && adapters
                    .iter()
                    .map(|adapter| {
                        fw::RerankModel::input_token_estimate(
                            adapter.as_ref(),
                            &batch_request(start..end + 1),
                        )
                    })
                    .max()
                    .unwrap_or(0)
                    <= capacity as u64
            {
                end += 1;
            }
            if end == start {
                return failure("单个重排候选超过模型输入预算");
            }
            batches.push((start, batch_request(start..end)));
            start = end;
        }
        let client = self
            .models
            .executor
            .with_runtime(runtime)
            .with_auto_routers(routers)
            .with_options(options);
        let mut report = None;
        let mut hits = Vec::new();
        let deadline = tokio::time::Instant::now() + effective.run_timeout;
        for (offset, batch) in batches {
            let observed =
                match tokio::time::timeout_at(deadline, client.rerank_observed(batch, &effective))
                    .await
                {
                    Ok(observed) => observed,
                    Err(_) => {
                        let mut partial: ModelExecutionReport = report.unwrap_or_default();
                        partial.provider_usage = None;
                        partial.provider_metrics = None;
                        partial.diagnostics_truncated = true;
                        return ObservedModelResult {
                            result: Err(fw::AgentError::Run("重排操作超时".into())),
                            report: partial,
                        };
                    }
                };
            merge_rerank_report(&mut report, observed.report);
            match observed.result {
                Ok(response) => {
                    hits.extend(response.results.into_iter().map(|hit| LlmRuntimeRerankHit {
                        document_index: offset + hit.index,
                        score: hit.score,
                    }))
                }
                Err(error) => {
                    return ObservedModelResult {
                        result: Err(error),
                        report: report.unwrap_or_default(),
                    };
                }
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.document_index.cmp(&b.document_index))
        });
        if let Some(n) = top_n {
            hits.truncate(n);
        }
        ObservedModelResult {
            result: Ok(hits),
            report: report.unwrap_or_default(),
        }
    }
}

fn merge_rerank_report(total: &mut Option<ModelExecutionReport>, next: ModelExecutionReport) {
    fn add(a: fw::Usage, b: fw::Usage) -> fw::Usage {
        fw::Usage {
            input_tokens: a.input_tokens.saturating_add(b.input_tokens),
            output_tokens: a.output_tokens.saturating_add(b.output_tokens),
        }
    }
    let Some(total) = total.as_mut() else {
        *total = Some(next);
        return;
    };
    total.calls += next.calls;
    total.attempts += next.attempts;
    total.retries += next.retries;
    // Batches share one budget ledger; each report contains its cumulative snapshot.
    total.ledger_usage = next.ledger_usage;
    total.provider_usage = total
        .provider_usage
        .take()
        .zip(next.provider_usage)
        .map(|(a, b)| add(a, b));
    total.provider_usage_subtotal = match (
        total.provider_usage_subtotal.take(),
        next.provider_usage_subtotal,
    ) {
        (Some(a), Some(b)) => Some(add(a, b)),
        (a, b) => a.or(b),
    };
    total.provider_metrics = total
        .provider_metrics
        .take()
        .zip(next.provider_metrics)
        .map(|(a, b)| fw::ProviderMetrics {
            cached_input_tokens: a
                .cached_input_tokens
                .zip(b.cached_input_tokens)
                .map(|(a, b)| a.saturating_add(b)),
            reasoning_tokens: a
                .reasoning_tokens
                .zip(b.reasoning_tokens)
                .map(|(a, b)| a.saturating_add(b)),
        })
        .filter(|_| total.provider_usage.is_some());
    total.diagnostics_truncated |= next.diagnostics_truncated
        || total.selected.len() + next.selected.len() > 256
        || total.dispatches.len() + next.dispatches.len() > 256;
    total.selected.extend(
        next.selected
            .into_iter()
            .take(256usize.saturating_sub(total.selected.len())),
    );
    total.dispatches.extend(
        next.dispatches
            .into_iter()
            .take(256usize.saturating_sub(total.dispatches.len())),
    );
}
