use serde::Serialize;
use serde_json::Value;

use crate::application::rag::{
    CandidateHit, ContextPack, RagDebugProfileResult, RagDebugSourceResult, RagIngestResult,
    RagRetryIndexResult, RagRetryStageResult, RagSearchResult,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagIngestResponse {
    pub corpus_id: String,
    pub profile_id: String,
    pub source_code: String,
    pub source_version: String,
    pub status: String,
    pub text_unit_count: usize,
    pub fulltext_status: String,
    pub embedding_status: String,
    pub warnings: Vec<Value>,
}

impl From<RagIngestResult> for RagIngestResponse {
    fn from(result: RagIngestResult) -> Self {
        Self {
            corpus_id: result.corpus_id,
            profile_id: result.profile_id,
            source_code: result.source_code,
            source_version: result.source_version,
            status: result.status,
            text_unit_count: result.text_unit_count,
            fulltext_status: result.fulltext_status,
            embedding_status: result.embedding_status,
            warnings: result.warnings,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchResponse {
    pub profile_id: String,
    pub query: String,
    pub context_strategy: String,
    pub hits: Vec<RagCandidateHitResponse>,
    pub context_pack: Option<RagContextPackResponse>,
    pub warnings: Vec<Value>,
    pub trace_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCandidateHitResponse {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub target_type: String,
    pub target_code: String,
    pub route_ids: Vec<String>,
    pub score: f64,
    pub scope_type: String,
    pub scope_code: String,
    pub preview: String,
    pub text: String,
    pub metadata: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagContextPackResponse {
    pub items: Vec<RagContextPackItemResponse>,
    pub token_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagContextPackItemResponse {
    pub source_code: String,
    pub text: String,
    pub tokens: usize,
    pub metadata: Value,
}

impl From<RagSearchResult> for RagSearchResponse {
    fn from(result: RagSearchResult) -> Self {
        Self {
            profile_id: result.profile_id,
            query: result.query,
            context_strategy: result.context_strategy,
            hits: result
                .hits
                .into_iter()
                .map(RagCandidateHitResponse::from)
                .collect(),
            context_pack: result.context_pack.map(RagContextPackResponse::from),
            warnings: result.warnings,
            trace_code: result.trace_code,
        }
    }
}

impl From<CandidateHit> for RagCandidateHitResponse {
    fn from(hit: CandidateHit) -> Self {
        Self {
            corpus_id: hit.corpus_id,
            source_code: hit.source_code,
            source_version: hit.source_version,
            target_type: hit.target_type,
            target_code: hit.target_code,
            route_ids: hit.route_ids,
            score: hit.score,
            scope_type: hit.scope_type,
            scope_code: hit.scope_code,
            preview: hit.preview,
            text: hit.text,
            metadata: hit.metadata,
        }
    }
}

pub type RagDebugProfileResponse = RagDebugProfileResult;
pub type RagDebugSourceResponse = RagDebugSourceResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetryIndexResponse {
    pub corpus_id: String,
    pub source_code: String,
    pub source_version: String,
    pub status: String,
    pub stage_results: Vec<RagRetryStageResponse>,
    pub warnings: Vec<Value>,
    pub trace_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetryStageResponse {
    pub stage_id: String,
    pub status: String,
    pub updated_count: usize,
    pub warnings: Vec<Value>,
}

impl From<RagRetryIndexResult> for RagRetryIndexResponse {
    fn from(result: RagRetryIndexResult) -> Self {
        Self {
            corpus_id: result.corpus_id,
            source_code: result.source_code,
            source_version: result.source_version,
            status: result.status,
            stage_results: result
                .stage_results
                .into_iter()
                .map(RagRetryStageResponse::from)
                .collect(),
            warnings: result.warnings,
            trace_code: result.trace_code,
        }
    }
}

impl From<RagRetryStageResult> for RagRetryStageResponse {
    fn from(result: RagRetryStageResult) -> Self {
        Self {
            stage_id: result.stage_id,
            status: result.status,
            updated_count: result.updated_count,
            warnings: result.warnings,
        }
    }
}

impl From<ContextPack> for RagContextPackResponse {
    fn from(pack: ContextPack) -> Self {
        Self {
            items: pack
                .items
                .into_iter()
                .map(|item| RagContextPackItemResponse {
                    source_code: item.source_code,
                    text: item.text,
                    tokens: item.tokens,
                    metadata: item.metadata,
                })
                .collect(),
            token_count: pack.token_count,
        }
    }
}
