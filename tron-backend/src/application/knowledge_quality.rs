//! Product-owned datasets, isolated retrieval experiments and automatic publication.
mod candidate;
mod dataset;
mod optimizer;
mod publication;
mod runtime;
mod snapshot;
use crate::application::knowledge_base::{KnowledgeBaseError, KnowledgeBaseService};
use dataset::{invalid, invalid_error, invalid_with};
use eliteforge_agent_core::*;

pub(crate) use runtime::QualityJobs;
pub use runtime::{QualityBudget, QualityQuestionGenerator};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct KnowledgeQualityService {
    pub(crate) host: KnowledgeBaseService,
}
impl KnowledgeQualityService {
    pub fn new(host: KnowledgeBaseService) -> Self {
        Self { host }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEvaluationDataset {
    pub library_id: String,
    pub source_fingerprint: String,
    pub revision: u64,
    pub created_at: i64,
    pub cases: Vec<RetrievalEvaluationCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct QualityOptimizationOptions {
    pub candidates: Vec<crate::application::document::ChunkConfig>,
    pub target_recall: f64,
    pub minimum_improvement: f64,
    pub top_k: usize,
    pub max_trials: usize,
    pub max_cases: usize,
    pub budget: QualityBudget,
}
impl Default for QualityOptimizationOptions {
    fn default() -> Self {
        Self {
            candidates: vec![],
            target_recall: 0.85,
            minimum_improvement: 0.02,
            top_k: 8,
            max_trials: 3,
            max_cases: 16,
            budget: Default::default(),
        }
    }
}
impl QualityOptimizationOptions {
    pub fn validate(&self) -> Result<(), KnowledgeBaseError> {
        if !self.target_recall.is_finite()
            || !(0.0..=1.0).contains(&self.target_recall)
            || !self.minimum_improvement.is_finite()
            || !(0.0..=0.5).contains(&self.minimum_improvement)
            || !(1..=30).contains(&self.top_k)
            || !(1..=6).contains(&self.max_trials)
            || !(8..=64).contains(&self.max_cases)
        {
            return Err(KnowledgeBaseError::invalid(
                "knowledge.quality.options_out_of_range",
            ));
        }
        if self.candidates.len() > self.max_trials {
            return Err(KnowledgeBaseError::invalid(
                "knowledge.quality.too_many_candidates",
            ));
        }
        for config in &self.candidates {
            config.validate()?;
        }
        self.budget.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityTrial {
    pub strategy: ChunkStrategyDescriptor,
    pub chunk_config: crate::application::document::ChunkConfig,
    pub training: Option<RetrievalEvaluationMetrics>,
    pub holdout: Option<RetrievalEvaluationMetrics>,
    pub chunk_count: usize,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityOptimizationRun {
    pub id: String,
    pub library_id: String,
    #[serde(skip)]
    pub owner_user_code: String,
    pub request_id: String,
    pub source_fingerprint: String,
    pub dataset_revision: u64,
    #[serde(default)]
    pub cases: Vec<RetrievalEvaluationCase>,
    pub status: String,
    pub stage: String,
    pub options: QualityOptimizationOptions,
    pub baseline_training: Option<RetrievalEvaluationMetrics>,
    pub baseline_holdout: Option<RetrievalEvaluationMetrics>,
    pub trials: Vec<QualityTrial>,
    pub selected_trial: Option<usize>,
    pub published: bool,
    pub target_reached: bool,
    pub missing: Vec<RetrievalCaseScore>,
    pub model_operations: usize,
    pub estimated_input_tokens: usize,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualitySourceExcerpt {
    pub document_id: String,
    pub document_name: String,
    pub block_id: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub location: serde_json::Value,
}

impl KnowledgeBaseService {
    pub(crate) fn recover_quality_optimizations(&self) -> Result<(), KnowledgeBaseError> {
        self.repository.recover_quality_runs()
    }
}
