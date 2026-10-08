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
        if config.model_type != ModelType::Embedding || dimensions == 0 {
            return Err("query requires a configured embedding index space".into());
        }
        if let Some(run) = run {
            run.check_cancelled().map_err(|error| error.to_string())?;
        }
        let model = runtime_to_agent_model_config(config);
        let wire = sdk_config(&model)?;
        let factory = Arc::new(
            sdk::ScopedModelTransportFactory::new(self.models.factory.clone(), &wire)
                .map_err(|error| error.to_string())?,
        );
        let reference = fw::ModelRef::new("http-embedding", &model.model_code);
        let space = fw::EmbeddingSpace {
            model: reference.clone(),
            revision: "v1".into(),
            dimensions,
            preprocessing: "knowledge-base-local-v1".into(),
        };
        let mut update =
            eliteforge_agent_framework::ModelRuntimeUpdate::new("http-embedding", 0, vec![]);
        update.embedding.push(fw::EmbeddingModelService::new(
            reference,
            Arc::new(sdk::HttpEmbeddingModel {
                config: wire,
                factory,
            }),
        ));
        update.spaces.push(space.clone());
        let runtime = eliteforge_agent_framework::ModelRuntime::new(update)
            .map_err(|error| error.to_string())?;
        let mut executor = self.models.executor.with_runtime(runtime);
        let effective = limits(
            &model,
            &self.models.invocation_config().embedding.timeouts(),
        );
        let options = match run {
            Some(run) => fw::ModelInvocationOptions::for_run(run, &effective),
            None => fw::ModelInvocationOptions::for_call(&effective)
                .map_err(|error| error.to_string())?,
        };
        executor = executor.with_options(options);
        // One attempt. The SDK keeps the existing step/continuation/token budgets binding.
        let response = executor
            .embed_query(&space, fw::EmbeddingRequest::texts([input]))
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
