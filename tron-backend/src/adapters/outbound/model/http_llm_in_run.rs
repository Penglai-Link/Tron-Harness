//! Retrieval uses the existing SDK model transport and the active run ledger.
use super::*;
use fw::ModelOperations;

impl HttpLlmRuntimeClient {
    pub(super) async fn query_embedding_in_run(
        &self,
        config: &RuntimeModelConfig,
        input: &str,
        dimensions: usize,
        run: &fw::RunContext,
    ) -> Result<(Vec<f32>, Option<String>), String> {
        self.query_embedding(config, input, dimensions, Some(run))
            .await
    }
    pub(super) async fn query_embedding(
        &self,
        config: &RuntimeModelConfig,
        input: &str,
        dimensions: usize,
        run: Option<&fw::RunContext>,
    ) -> Result<(Vec<f32>, Option<String>), String> {
        if !matches!(
            config.model_type,
            ModelType::TextEmbedding | ModelType::MultimodalEmbedding
        ) || dimensions == 0
        {
            return Err("query requires a configured embedding index space".into());
        }
        if let Some(run) = run {
            run.check_cancelled().map_err(|error| error.to_string())?;
        }
        let model = runtime_to_agent_model_config(config);
        let (runtime, choice, _) = self
            .auxiliary_pool(config, Some(dimensions), false)
            .map_err(|error| error.to_string())?;
        let first = if let Some(route) = &config.route {
            fw::ModelRef::new(
                &route.candidates[0].provider_id,
                &route.candidates[0].config.model_code,
            )
        } else {
            fw::ModelRef::new("http-embedding", &config.model_code)
        };
        let shared = config.route.as_ref().filter(|route| route.shared_identity);
        let space = fw::EmbeddingSpace {
            model: first,
            logical_model_code: shared.map(|route| route.model_code.clone()),
            revision: shared.map_or_else(|| "v1".into(), |route| route.model_code.clone()),
            dimensions,
            preprocessing: "knowledge-base-local-v1".into(),
        };
        let mut executor = self
            .models
            .executor
            .with_runtime(runtime)
            .with_auto_routers(self.pool_routers(config));
        let effective = limits(
            &model,
            &self.models.invocation_config().embedding.timeouts(),
            self.models.invocation_config().default_output_tokens,
        );
        let options = match run {
            Some(run) => fw::ModelInvocationOptions::for_run(run, &effective),
            None => fw::ModelInvocationOptions::for_call(&effective)
                .map_err(|error| error.to_string())?,
        };
        executor = executor.with_options(options);
        let mut request = fw::EmbeddingRequest::texts([input]);
        request.choice = choice;
        let response = executor
            .embed_query(&space, request)
            .await
            .map_err(|error| error.to_string())?;
        let provider_model = response.provider_model;
        response
            .vectors
            .into_iter()
            .next()
            .map(|vector| (vector.values, provider_model))
            .ok_or_else(|| "query embedding response was empty".into())
    }
}
