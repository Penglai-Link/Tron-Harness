use serde::Serialize;
use serde_json::Value;

use crate::application::rag::RagIngestResult;

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
