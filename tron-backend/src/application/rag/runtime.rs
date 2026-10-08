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
    pub fn new(llm: LlmRuntimeService, llm_router: impl TextModelRouter + 'static) -> Self {
        Self {
            llm,
            llm_router: Arc::new(llm_router),
        }
    }
}

impl RagEmbeddingRuntime for DefaultRagRuntime {
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
            let config = self
                .llm_router
                .route_text(call.workspace_code.as_str(), call.model_code.as_deref())?;
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
