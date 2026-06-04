use std::{future::Future, path::PathBuf, pin::Pin};

use crate::application::llm::{LlmEmbeddingCallCommand, LlmRerankCallCommand, LlmRuntimeService};

use super::{
    RagDocumentSourceCall, RagDocumentSourceRuntime, RagEmbeddingCall, RagEmbeddingResult,
    RagEmbeddingRuntime, RagRerankCall, RagSearchRuntime,
    document_source::{RagResolvedDocument, resolve_project_doc_markdown_file},
};

use crate::domain::rag::RagError;

#[derive(Clone)]
pub struct DefaultRagRuntime {
    llm: LlmRuntimeService,
    project_doc_root: PathBuf,
}

impl DefaultRagRuntime {
    pub fn new(llm: LlmRuntimeService, project_doc_root: impl Into<PathBuf>) -> Self {
        Self {
            llm,
            project_doc_root: project_doc_root.into(),
        }
    }
}

impl RagEmbeddingRuntime for DefaultRagRuntime {
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
                    model_code: result.model_code,
                    embedding: result.embedding,
                })
                .map_err(|error| error.to_string())
        })
    }
}

impl RagSearchRuntime for DefaultRagRuntime {
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

impl RagDocumentSourceRuntime for DefaultRagRuntime {
    fn resolve_project_doc_markdown<'a>(
        &'a self,
        call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(
            async move { resolve_project_doc_markdown_file(self.project_doc_root.clone(), &call) },
        )
    }
}
