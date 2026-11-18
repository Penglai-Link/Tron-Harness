use crate::application::{document::ChunkConfig, knowledge_quality::*};
use eliteforge_agent_core::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StrategyVo {
    pub id: String,
    pub version: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StrategyListVo {
    pub strategies: Vec<StrategyVo>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StartDto {
    pub request_id: String,
    pub options: OptionsDto,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OptionsDto {
    pub candidates: Vec<ChunkConfigDto>,
    pub target_recall: f64,
    pub minimum_improvement: f64,
    pub top_k: usize,
    pub max_trials: usize,
    pub max_cases: usize,
    pub budget: BudgetDto,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ChunkConfigDto {
    pub mode: String,
    pub max_chars: usize,
    pub overlap_chars: usize,
    pub heading_depth: u8,
}
impl From<ChunkConfigDto> for ChunkConfig {
    fn from(c: ChunkConfigDto) -> Self {
        Self {
            mode: serde_json::from_value(serde_json::Value::String(c.mode))
                .expect("open strategy ID"),
            max_chars: c.max_chars,
            overlap_chars: c.overlap_chars,
            heading_depth: c.heading_depth,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BudgetDto {
    pub max_model_operations: usize,
    pub max_input_tokens: usize,
    pub max_duration_seconds: u64,
}
impl From<OptionsDto> for QualityOptimizationOptions {
    fn from(o: OptionsDto) -> Self {
        Self {
            candidates: o.candidates.into_iter().map(Into::into).collect(),
            target_recall: o.target_recall,
            minimum_improvement: o.minimum_improvement,
            top_k: o.top_k,
            max_trials: o.max_trials,
            max_cases: o.max_cases,
            budget: QualityBudget {
                max_model_operations: o.budget.max_model_operations,
                max_input_tokens: o.budget.max_input_tokens,
                max_duration_seconds: o.budget.max_duration_seconds,
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DatasetPutDto {
    pub source_fingerprint: String,
    pub cases: Vec<CaseDto>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CaseDto {
    pub id: String,
    pub query: String,
    pub expected: Vec<TargetDto>,
    pub partition: PartitionDto,
    pub provenance: ProvenanceDto,
}
#[derive(Deserialize)]
pub(super) enum PartitionDto {
    Training,
    Holdout,
}
#[derive(Deserialize)]
pub(super) enum ProvenanceDto {
    HumanLabeled,
    Generated,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TargetDto {
    pub document_id: String,
    pub span: SpanDto,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SpanDto {
    pub block_id: String,
    pub start: usize,
    pub end: usize,
}
impl From<CaseDto> for RetrievalEvaluationCase {
    fn from(c: CaseDto) -> Self {
        Self {
            id: c.id,
            query: c.query,
            partition: match c.partition {
                PartitionDto::Training => EvaluationPartition::Training,
                PartitionDto::Holdout => EvaluationPartition::Holdout,
            },
            provenance: match c.provenance {
                ProvenanceDto::HumanLabeled => EvaluationProvenance::HumanLabeled,
                ProvenanceDto::Generated => EvaluationProvenance::Generated,
            },
            expected: c
                .expected
                .into_iter()
                .map(|t| RetrievalEvidenceTarget {
                    document_id: t.document_id,
                    span: ChunkSourceSpan {
                        block_id: t.span.block_id,
                        start: t.span.start,
                        end: t.span.end,
                    },
                })
                .collect(),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CancelDto {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SessionDto {}

// Shared nested protocol records, never persisted entities. Each endpoint owns its outer VO.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CaseVo {
    id: String,
    query: String,
    expected: Vec<TargetVo>,
    partition: &'static str,
    provenance: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetVo {
    document_id: String,
    span: SpanVo,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpanVo {
    block_id: String,
    start: usize,
    end: usize,
}
impl From<RetrievalEvidenceTarget> for TargetVo {
    fn from(t: RetrievalEvidenceTarget) -> Self {
        Self {
            document_id: t.document_id,
            span: SpanVo {
                block_id: t.span.block_id,
                start: t.span.start,
                end: t.span.end,
            },
        }
    }
}
impl From<RetrievalEvaluationCase> for CaseVo {
    fn from(c: RetrievalEvaluationCase) -> Self {
        Self {
            id: c.id,
            query: c.query,
            expected: c.expected.into_iter().map(Into::into).collect(),
            partition: match c.partition {
                EvaluationPartition::Training => "Training",
                EvaluationPartition::Holdout => "Holdout",
            },
            provenance: match c.provenance {
                EvaluationProvenance::HumanLabeled => "HumanLabeled",
                EvaluationProvenance::Generated => "Generated",
            },
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DatasetVo {
    library_id: String,
    source_fingerprint: String,
    revision: u64,
    created_at: i64,
    cases: Vec<CaseVo>,
}
impl From<KnowledgeEvaluationDataset> for DatasetVo {
    fn from(d: KnowledgeEvaluationDataset) -> Self {
        Self {
            library_id: d.library_id,
            source_fingerprint: d.source_fingerprint,
            revision: d.revision,
            created_at: d.created_at,
            cases: d.cases.into_iter().map(Into::into).collect(),
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DatasetGetVo {
    pub dataset: Option<DatasetVo>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DatasetPutVo {
    pub dataset: DatasetVo,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricsVo {
    cases: usize,
    recall: f64,
    precision: f64,
    mrr: f64,
    discounted_recall: f64,
    hit_rate: f64,
    human_labeled_cases: usize,
    generated_cases: usize,
}
impl From<RetrievalEvaluationMetrics> for MetricsVo {
    fn from(m: RetrievalEvaluationMetrics) -> Self {
        Self {
            cases: m.cases,
            recall: m.recall,
            precision: m.precision,
            mrr: m.mrr,
            discounted_recall: m.discounted_recall,
            hit_rate: m.hit_rate,
            human_labeled_cases: m.human_labeled_cases,
            generated_cases: m.generated_cases,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrialVo {
    strategy: StrategyVo,
    chunk_config: ChunkConfigVo,
    training: Option<MetricsVo>,
    holdout: Option<MetricsVo>,
    chunk_count: usize,
    duration_ms: u64,
    error: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChunkConfigVo {
    mode: String,
    max_chars: usize,
    overlap_chars: usize,
    heading_depth: u8,
}
fn trial_vo(t: QualityTrial, locale: &str) -> TrialVo {
    TrialVo {
        strategy: StrategyVo {
            id: t.strategy.id,
            version: t.strategy.version,
        },
        chunk_config: ChunkConfigVo {
            mode: t.chunk_config.mode.strategy_id().into(),
            max_chars: t.chunk_config.max_chars,
            overlap_chars: t.chunk_config.overlap_chars,
            heading_depth: t.chunk_config.heading_depth,
        },
        training: t.training.map(Into::into),
        holdout: t.holdout.map(Into::into),
        chunk_count: t.chunk_count,
        duration_ms: t.duration_ms,
        error: t.error.map(|error| diagnostic(error, locale)),
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MissingVo {
    case_id: String,
    recall: f64,
    missing: Vec<TargetVo>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunVo {
    id: String,
    library_id: String,
    request_id: String,
    source_fingerprint: String,
    dataset_revision: u64,
    cases: Vec<CaseVo>,
    status: String,
    stage: String,
    target_recall: f64,
    top_k: usize,
    baseline_training: Option<MetricsVo>,
    baseline_holdout: Option<MetricsVo>,
    trials: Vec<TrialVo>,
    selected_trial: Option<usize>,
    published: bool,
    target_reached: bool,
    missing: Vec<MissingVo>,
    model_operations: usize,
    estimated_input_tokens: usize,
    warnings: Vec<String>,
    error: Option<String>,
    created_at: i64,
    updated_at: i64,
}
fn pascal(s: &str) -> String {
    s.split('_')
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|f| format!("{}{}", f.to_uppercase(), c.as_str().to_lowercase()))
                .unwrap_or_default()
        })
        .collect()
}
pub(super) fn run_vo(r: QualityOptimizationRun, locale: &str) -> RunVo {
    RunVo {
        id: r.id,
        library_id: r.library_id,
        request_id: r.request_id,
        source_fingerprint: r.source_fingerprint,
        dataset_revision: r.dataset_revision,
        cases: r.cases.into_iter().map(Into::into).collect(),
        status: pascal(&r.status),
        stage: pascal(&r.stage),
        target_recall: r.options.target_recall,
        top_k: r.options.top_k,
        baseline_training: r.baseline_training.map(Into::into),
        baseline_holdout: r.baseline_holdout.map(Into::into),
        trials: r
            .trials
            .into_iter()
            .map(|trial| trial_vo(trial, locale))
            .collect(),
        selected_trial: r.selected_trial,
        published: r.published,
        target_reached: r.target_reached,
        missing: r
            .missing
            .into_iter()
            .map(|m| MissingVo {
                case_id: m.case_id,
                recall: m.recall,
                missing: m.missing.into_iter().map(Into::into).collect(),
            })
            .collect(),
        model_operations: r.model_operations,
        estimated_input_tokens: r.estimated_input_tokens,
        warnings: r
            .warnings
            .into_iter()
            .map(|warning| diagnostic(warning, locale))
            .collect(),
        error: r.error.map(|error| diagnostic(error, locale)),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StartVo {
    pub run: RunVo,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunsVo {
    pub runs: Vec<RunVo>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunGetVo {
    pub run: RunVo,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CancelVo {
    pub run: RunVo,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SessionVo {
    pub session: SessionRecordVo,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SessionRecordVo {
    pub id: String,
    pub session_code: String,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub agent_code: Option<String>,
    pub title: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

fn diagnostic(message: String, locale: &str) -> String {
    // Structured diagnostics persist a stable key + arguments and are rendered
    // with the request locale. Legacy rows stored plain text before the
    // migration and are passed through unchanged (no data migration).
    crate::shared::i18n_message::StoredMessage::decode(message).render(locale, None)
}
