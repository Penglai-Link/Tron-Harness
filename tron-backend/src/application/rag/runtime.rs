use std::{future::Future, pin::Pin, sync::Arc};

use crate::application::llm::{LlmEmbeddingCallCommand, LlmRerankCallCommand, LlmRuntimeService};
use crate::application::model_router::TextModelRouter;

use super::{
    RagEmbeddingBatchCall, RagEmbeddingBatchResult, RagEmbeddingCall, RagEmbeddingResult,
    RagEmbeddingRuntime, RagGraphCall, RagGraphResult, RagGraphRuntime, RagRerankCall,
    RagSearchRuntime,
};

#[derive(Clone)]
pub struct DefaultRagRuntime {
    llm: LlmRuntimeService,
    llm_router: Arc<dyn TextModelRouter>,
}

impl DefaultRagRuntime {
    pub(crate) async fn generate_text(
        &self,
        workspace: &str,
        model: Option<&str>,
        prompt: String,
    ) -> Result<String, String> {
        let config = self.llm_router.route_text(workspace, model)?;
        self.llm
            .text_generation_call_with_config(config, prompt)
            .await
            .map(|result| result.output)
            .map_err(|error| error.to_string())
    }
    pub fn new(llm: LlmRuntimeService, llm_router: impl TextModelRouter + 'static) -> Self {
        Self {
            llm,
            llm_router: Arc::new(llm_router),
        }
    }
}

impl RagEmbeddingRuntime for DefaultRagRuntime {
    fn embedding_batch_input_limit(
        &self,
        workspace: &str,
        model: Option<&str>,
    ) -> Result<Option<usize>, String> {
        RagEmbeddingRuntime::embedding_batch_input_limit(&self.llm, workspace, model)
    }
    fn embed_content_batch<'a>(
        &'a self,
        call: super::RagEmbeddingContentBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        RagEmbeddingRuntime::embed_content_batch(&self.llm, call)
    }
    fn resolve_index_model(
        &self,
        workspace: &str,
        model: Option<&str>,
    ) -> Result<Option<String>, String> {
        RagEmbeddingRuntime::resolve_index_model(&self.llm, workspace, model)
    }
    fn embedding_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimension: usize,
    ) -> Result<Option<String>, String> {
        self.llm
            .embedding_space_fingerprint(workspace, model, dimension)
            .map(Some)
            .map_err(|error| error.to_string())
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>> {
        RagEmbeddingRuntime::embed_batch(&self.llm, call)
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            self.llm
                .embedding_call(LlmEmbeddingCallCommand {
                    workspace_code: call.workspace_code,
                    model_code: call.model_code,
                    input: call.input,
                })
                .await
                .map(|result| RagEmbeddingResult {
                    provider_model: result.provider_model,
                    model_code: result.model_code,
                    embedding: result.embedding,
                })
                .map_err(|error| error.to_string())
        })
    }
}

impl RagSearchRuntime for DefaultRagRuntime {
    fn embedding_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimension: usize,
    ) -> Result<Option<String>, String> {
        RagEmbeddingRuntime::embedding_fingerprint(self, workspace, model, dimension)
    }
    fn embed_for_retrieval<'a>(
        &'a self,
        call: RagEmbeddingCall,
        dimensions: Option<usize>,
        run: Option<&'a eliteforge_agent_framework::RunContext>,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let Some(run) = run else {
                return self
                    .llm
                    .embedding_query_call(
                        LlmEmbeddingCallCommand {
                            workspace_code: call.workspace_code,
                            model_code: call.model_code,
                            input: call.input,
                        },
                        dimensions.unwrap_or(1024),
                    )
                    .await
                    .map(|result| RagEmbeddingResult {
                        provider_model: result.provider_model,
                        model_code: result.model_code,
                        embedding: result.embedding,
                    })
                    .map_err(|error| error.to_string());
            };
            self.llm
                .embedding_query_in_run(
                    LlmEmbeddingCallCommand {
                        workspace_code: call.workspace_code,
                        model_code: call.model_code,
                        input: call.input,
                    },
                    dimensions
                        .unwrap_or(crate::shared::sqlite_extensions::TRON_VECTOR_INDEX_DIMENSION),
                    run,
                )
                .await
                .map(|value| RagEmbeddingResult {
                    model_code: value.model_code,
                    embedding: value.embedding,
                    provider_model: value.provider_model,
                })
                .map_err(|error| error.to_string())
        })
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>> {
        Box::pin(async move { <Self as RagEmbeddingRuntime>::embed(self, call).await })
    }

    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>> {
        Box::pin(async move {
            self.llm
                .rerank_call(LlmRerankCallCommand {
                    workspace_code: call.workspace_code,
                    model_code: call.model_code,
                    query: call.query,
                    documents: call.documents,
                    top_n: call.top_n,
                })
                .await
                .map(|result| {
                    result
                        .items
                        .into_iter()
                        .map(|item| (item.document_index, item.score))
                        .collect()
                })
                .map_err(|error| error.to_string())
        })
    }
}

impl RagGraphRuntime for DefaultRagRuntime {
    fn extract_graph<'a>(
        &'a self,
        call: RagGraphCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagGraphResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let config = graph_extraction_config(
                self.llm_router
                    .route_text(call.workspace_code.as_str(), call.model_code.as_deref())?,
            );
            self.llm
                .text_generation_call_with_config(config, call.prompt)
                .await
                .map(|result| RagGraphResult {
                    model_code: result.model_code,
                    graph_json: result.output,
                })
                .map_err(|error| error.to_string())
        })
    }
}

/// Graph extraction needs reasoning and a shared reasoning/answer budget. This
/// policy belongs to the host use case, not a provider-wide parameter default.
fn graph_extraction_config(
    mut config: crate::application::model::RuntimeModelConfig,
) -> crate::application::model::RuntimeModelConfig {
    config.thinking_enabled = true;
    config.reasoning_effort = config
        .reasoning_effort
        .or(Some(crate::domain::reasoning::ReasoningEffort::High));
    config.max_tokens = Some(config.max_tokens.unwrap_or(0).max(32_768));
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        model::{ModelProtocol, ModelType},
        reasoning::ReasoningEffort,
    };

    #[test]
    fn graph_extraction_preserves_deep_reasoning_and_gives_it_an_explicit_output_budget() {
        for protocol in [ModelProtocol::OpenAiCompatible, ModelProtocol::Responses] {
            let original = crate::application::model::RuntimeModelConfig {
                model_code: "extractor".into(),
                model_name: "extractor".into(),
                protocol,
                model_type: ModelType::Text,
                base_url: "https://provider.invalid".into(),
                api_path: None,
                api_key: String::new(),
                thinking_enabled: false,
                reasoning_effort: None,
                responses_stateful_enabled: false,
                max_tokens: None,
                context_window_tokens: Some(128_000),
                temperature: None,
                top_p: None,
                max_retries: Some(0),
            };
            let configured = graph_extraction_config(original.clone());
            assert!(configured.thinking_enabled);
            assert_eq!(configured.reasoning_effort, Some(ReasoningEffort::High));
            assert_eq!(configured.max_tokens, Some(32_768));
            assert_eq!(configured.protocol, original.protocol);
            assert_eq!(
                configured.context_window_tokens,
                original.context_window_tokens
            );
            let mut explicit = original;
            explicit.thinking_enabled = true;
            explicit.reasoning_effort = Some(ReasoningEffort::Max);
            explicit.max_tokens = Some(65_536);
            assert_eq!(graph_extraction_config(explicit.clone()), explicit);
        }
    }
}
