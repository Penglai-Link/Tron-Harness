use super::*;
use crate::application::rag::*;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

fn budget_message(key: &'static str) -> String {
    crate::shared::i18n_message::MessageText::new(key).encode()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct QualityBudget {
    pub max_model_operations: usize,
    pub max_input_tokens: usize,
    pub max_duration_seconds: u64,
}
impl Default for QualityBudget {
    fn default() -> Self {
        Self {
            max_model_operations: 160,
            max_input_tokens: 250_000,
            max_duration_seconds: 900,
        }
    }
}
impl QualityBudget {
    pub fn validate(&self) -> Result<(), KnowledgeBaseError> {
        if !(1..=1000).contains(&self.max_model_operations)
            || !(1000..=2_000_000).contains(&self.max_input_tokens)
            || !(30..=3600).contains(&self.max_duration_seconds)
        {
            return Err(KnowledgeBaseError::invalid(
                "knowledge.quality.budget_out_of_range",
            ));
        }
        Ok(())
    }
}
#[derive(Default)]
pub(crate) struct QualityJobs {
    pub jobs: Mutex<BTreeMap<String, CancellationToken>>,
}

pub(crate) struct BudgetState {
    pub budget: QualityBudget,
    pub token: CancellationToken,
    pub started: Instant,
    pub operations: AtomicUsize,
    pub input_tokens: AtomicUsize,
    admission: Mutex<()>,
    query_vectors:
        Mutex<BTreeMap<(String, Option<String>, String, Option<usize>), RagEmbeddingResult>>,
}
impl BudgetState {
    pub fn new(budget: QualityBudget, token: CancellationToken) -> Arc<Self> {
        Arc::new(Self {
            budget,
            token,
            started: Instant::now(),
            operations: AtomicUsize::new(0),
            input_tokens: AtomicUsize::new(0),
            admission: Mutex::new(()),
            query_vectors: Mutex::new(BTreeMap::new()),
        })
    }
    pub fn check(&self) -> Result<(), String> {
        if self.token.is_cancelled() {
            return Err(budget_message("knowledge.quality.diagnostic.cancelled"));
        }
        if self.started.elapsed() > Duration::from_secs(self.budget.max_duration_seconds) {
            return Err(budget_message("knowledge.quality.diagnostic.time_budget"));
        }
        Ok(())
    }
    pub fn admit(&self, texts: impl IntoIterator<Item = String>) -> Result<(), String> {
        self.check()?;
        let tokens: usize = texts
            .into_iter()
            .map(|s| eliteforge_agent_core::Content::from(s).estimated_tokens())
            .sum();
        self.admit_tokens(tokens)
    }
    fn admit_tokens(&self, tokens: usize) -> Result<(), String> {
        self.check()?;
        let _guard = self
            .admission
            .lock()
            .map_err(|_| budget_message("knowledge.quality.internal_failed"))?;
        let operations = self.operations.load(Ordering::SeqCst);
        let input = self.input_tokens.load(Ordering::SeqCst);
        if operations >= self.budget.max_model_operations {
            return Err(budget_message(
                "knowledge.quality.diagnostic.model_operation_budget",
            ));
        }
        let next = input
            .checked_add(tokens)
            .filter(|n| *n <= self.budget.max_input_tokens)
            .ok_or_else(|| budget_message("knowledge.quality.diagnostic.input_budget"))?;
        self.operations.store(operations + 1, Ordering::SeqCst);
        self.input_tokens.store(next, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait QualityQuestionGenerator: Send + Sync {
    async fn generate(&self, prompt: String) -> Result<String, String>;
}
pub(crate) struct DefaultQuestionGenerator(pub DefaultRagRuntime);
#[async_trait::async_trait]
impl QualityQuestionGenerator for DefaultQuestionGenerator {
    async fn generate(&self, prompt: String) -> Result<String, String> {
        self.0.generate_text("PLATFORM", None, prompt).await
    }
}

#[derive(Clone)]
pub(crate) struct QualityRuntime {
    pub inner: DefaultRagRuntime,
    pub budget: Arc<BudgetState>,
}
impl RagEmbeddingRuntime for QualityRuntime {
    fn embedding_batch_input_limit(
        &self,
        workspace: &str,
        model: Option<&str>,
    ) -> Result<Option<usize>, String> {
        RagEmbeddingRuntime::embedding_batch_input_limit(&self.inner, workspace, model)
    }

    fn resolve_index_model(
        &self,
        workspace: &str,
        model: Option<&str>,
    ) -> Result<Option<String>, String> {
        RagEmbeddingRuntime::resolve_index_model(&self.inner, workspace, model)
    }
    fn embedding_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimension: usize,
    ) -> Result<Option<String>, String> {
        RagEmbeddingRuntime::embedding_fingerprint(&self.inner, workspace, model, dimension)
    }
    fn embed_content_batch<'a>(
        &'a self,
        call: RagEmbeddingContentBatchCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.budget.admit_tokens(
                call.inputs
                    .iter()
                    .map(|input| input.estimated_tokens())
                    .sum(),
            )?;
            tokio::select! {_ = self.budget.token.cancelled()=>Err("knowledge.quality.diagnostic.cancelled".into()),result=self.inner.embed_content_batch(call)=>result}
        })
    }
    fn embed_batch<'a>(
        &'a self,
        call: RagEmbeddingBatchCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagEmbeddingBatchResult, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.budget.admit(call.inputs.clone())?;
            tokio::select! {_ = self.budget.token.cancelled()=>Err("knowledge.quality.diagnostic.cancelled".into()),result=self.inner.embed_batch(call)=>result}
        })
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.budget.admit([call.input.clone()])?;
            RagEmbeddingRuntime::embed(&self.inner, call).await
        })
    }
}
impl RagSearchRuntime for QualityRuntime {
    fn embedding_fingerprint(
        &self,
        workspace: &str,
        model: Option<&str>,
        dimension: usize,
    ) -> Result<Option<String>, String> {
        RagSearchRuntime::embedding_fingerprint(&self.inner, workspace, model, dimension)
    }
    fn embed<'a>(
        &'a self,
        call: RagEmbeddingCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>,
    > {
        RagEmbeddingRuntime::embed(self, call)
    }
    fn embed_for_retrieval<'a>(
        &'a self,
        call: RagEmbeddingCall,
        dimensions: Option<usize>,
        run: Option<&'a eliteforge_agent_framework::RunContext>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagEmbeddingResult, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.budget.check()?;
            let key = (
                call.workspace_code.clone(),
                call.model_code.clone(),
                call.input.clone(),
                dimensions,
            );
            if let Some(vector) = self
                .budget
                .query_vectors
                .lock()
                .map_err(|_| "Query cache unavailable")?
                .get(&key)
                .cloned()
            {
                return Ok(vector);
            }
            self.budget.admit([call.input.clone()])?;
            let vector = tokio::select! {_ = self.budget.token.cancelled()=>return Err("knowledge.quality.diagnostic.cancelled".into()),result=self.inner.embed_for_retrieval(call,dimensions,run)=>result?};
            self.budget
                .query_vectors
                .lock()
                .map_err(|_| "Query cache unavailable")?
                .insert(key, vector.clone());
            Ok(vector)
        })
    }
    fn rerank<'a>(
        &'a self,
        call: RagRerankCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<(usize, f64)>, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut texts = call.documents.clone();
            texts.push(call.query.clone());
            self.budget.admit(texts)?;
            tokio::select! {_ = self.budget.token.cancelled()=>Err("knowledge.quality.diagnostic.cancelled".into()),result=self.inner.rerank(call)=>result}
        })
    }
}
impl RagGraphRuntime for QualityRuntime {
    fn extract_graph<'a>(
        &'a self,
        call: RagGraphCall,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<RagGraphResult, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            self.budget.admit([call.prompt.clone()])?;
            tokio::select! {_ = self.budget.token.cancelled()=>Err("knowledge.quality.diagnostic.cancelled".into()),result=self.inner.extract_graph(call)=>result}
        })
    }
}

pub(crate) struct QualityCheckpoint(pub Arc<BudgetState>);
impl RagIndexCheckpoint for QualityCheckpoint {
    fn validate(&self) -> Result<(), String> {
        self.0.check()
    }
    fn save_batch(&self, _result: &RagEmbeddingBatchResult) -> Result<(), String> {
        self.0.check()
    }
}
