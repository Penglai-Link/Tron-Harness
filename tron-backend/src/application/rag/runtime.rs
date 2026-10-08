use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

use crate::application::llm::{LlmEmbeddingCallCommand, LlmRerankCallCommand, LlmRuntimeService};
use crate::application::model_router::TextModelRouter;
use crate::config::MineruConfig;

use super::{
    RagDocumentParseCall, RagDocumentSourceCall, RagDocumentSourceRuntime, RagEmbeddingCall,
    RagEmbeddingResult, RagEmbeddingRuntime, RagRerankCall, RagSearchRuntime, RagTreeSummaryCall,
    RagTreeSummaryResult, RagTreeSummaryRuntime,
    document_source::{
        RagResolvedDocument, resolve_project_doc_markdown_file, resolve_project_doc_parse_file,
        resolve_project_doc_reference_file,
    },
    mineru::MineruDocumentParser,
};

use crate::domain::rag::RagError;

#[derive(Clone)]
pub struct DefaultRagRuntime {
    llm: LlmRuntimeService,
    llm_router: Arc<dyn TextModelRouter>,
    project_doc_root: PathBuf,
    mineru: MineruConfig,
    document_parser_client: reqwest::Client,
    runtime_config: Option<crate::application::system_config::BackendRuntimeConfig>,
}

impl DefaultRagRuntime {
    pub fn new(
        llm: LlmRuntimeService,
        llm_router: impl TextModelRouter + 'static,
        project_doc_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            llm,
            llm_router: Arc::new(llm_router),
            project_doc_root: project_doc_root.into(),
            mineru: MineruConfig::default(),
            document_parser_client: reqwest::Client::new(),
            runtime_config: None,
        }
    }

    pub fn with_document_parser(mut self, mineru: MineruConfig, client: reqwest::Client) -> Self {
        self.mineru = mineru;
        self.document_parser_client = client;
        self
    }

    pub fn with_runtime_config(
        mut self,
        runtime: crate::application::system_config::BackendRuntimeConfig,
    ) -> Self {
        self.runtime_config = Some(runtime);
        self
    }

    fn project_doc_root(&self) -> PathBuf {
        self.runtime_config
            .as_ref()
            .map(|runtime| PathBuf::from(runtime.snapshot().project_doc_root))
            .unwrap_or_else(|| self.project_doc_root.clone())
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

impl RagTreeSummaryRuntime for DefaultRagRuntime {
    fn summarize_tree<'a>(
        &'a self,
        call: RagTreeSummaryCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagTreeSummaryResult, String>> + Send + 'a>> {
        Box::pin(async move {
            let config = self
                .llm_router
                .route_text(call.workspace_code.as_str(), call.model_code.as_deref())?;
            self.llm
                .text_generation_call_with_config(config, call.prompt)
                .await
                .map(|result| RagTreeSummaryResult {
                    model_code: result.model_code,
                    summary_text: result.output,
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
        Box::pin(async move { resolve_project_doc_markdown_file(self.project_doc_root(), &call) })
    }

    fn resolve_project_doc_file<'a>(
        &'a self,
        call: RagDocumentSourceCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async move { resolve_project_doc_reference_file(self.project_doc_root(), &call) })
    }

    fn parse_project_doc_with_mineru<'a>(
        &'a self,
        call: RagDocumentParseCall,
    ) -> Pin<Box<dyn Future<Output = Result<RagResolvedDocument, RagError>> + Send + 'a>> {
        Box::pin(async move {
            let active = self
                .runtime_config
                .as_ref()
                .map(|runtime| runtime.snapshot());
            let root = active
                .as_ref()
                .map(|config| PathBuf::from(&config.project_doc_root))
                .unwrap_or_else(|| self.project_doc_root.clone());
            let file = resolve_project_doc_parse_file(root, &call)?;
            let config = active
                .map(|config| config.mineru)
                .unwrap_or_else(|| self.mineru.clone());
            MineruDocumentParser::new(self.document_parser_client.clone(), config)
                .parse(file)
                .await
        })
    }
}
