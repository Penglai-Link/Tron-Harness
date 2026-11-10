//! Relevance gates use model scores; reciprocal-rank fusion is ordering only.
use serde::{Deserialize, Serialize};

use super::{RagEvidence, RagService};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RagRetrievalConfig {
    /// Cosine similarity, not a confidence probability. None disables this gate.
    pub min_vector_similarity: Option<f64>,
    /// Native reranker score. None disables this gate; scales depend on the model.
    pub min_rerank_score: Option<f64>,
}

impl Default for RagRetrievalConfig {
    fn default() -> Self {
        Self {
            min_vector_similarity: Some(0.55),
            min_rerank_score: Some(0.5),
        }
    }
}

impl RagRetrievalConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self
            .min_vector_similarity
            .is_some_and(|score| !score.is_finite() || !(0.0..=1.0).contains(&score))
        {
            return Err("minVectorSimilarity must be finite and between 0 and 1 / 向量相似度阈值必须在 0 到 1 之间".into());
        }
        if self
            .min_rerank_score
            .is_some_and(|score| !score.is_finite() || score < 0.0)
        {
            return Err(
                "minRerankScore must be finite and nonnegative / 重排阈值必须是有限非负数".into(),
            );
        }
        Ok(())
    }

    pub(super) fn accepts_vector(&self, score: Option<f64>) -> bool {
        self.min_vector_similarity
            .is_none_or(|minimum| score.is_some_and(|score| score.is_finite() && score >= minimum))
    }

    pub(super) fn accepts_rerank(&self, score: f64) -> bool {
        score.is_finite() && self.min_rerank_score.is_none_or(|minimum| score >= minimum)
    }

    pub(super) fn filter_fallback(&self, hits: &mut Vec<RagEvidence>) {
        // Unscored fulltext/graph hits have no calibrated semantic score. They
        // may pass a successful reranker, but cannot bypass the fallback gate.
        hits.retain(|hit| self.accepts_vector(hit.vector_similarity));
    }
}

impl<R: Clone, E: Clone> RagService<R, E> {
    pub(super) fn retrieval_config(&self) -> RagRetrievalConfig {
        self.invocation_runtime_config
            .as_ref()
            .map(|runtime| runtime.snapshot().rag_retrieval)
            .unwrap_or_default()
    }
}
