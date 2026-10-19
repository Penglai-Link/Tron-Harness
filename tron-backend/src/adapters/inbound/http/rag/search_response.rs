//! Each HTTP use case owns its root DTO; nested evidence is a public concept.
use crate::application::rag::{
    RagEvidenceView, RagQueryResult, RagRetrievalStage, graph::GraphEvidenceView,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RecallInput {
    pub query: String,
    #[serde(default)]
    pub corpus_ids: Vec<String>,
    #[serde(default = "filters")]
    pub filters: Value,
    #[serde(default = "limit")]
    pub limit: usize,
}
fn filters() -> Value {
    json!({})
}
fn limit() -> usize {
    8
}

macro_rules! response {
    ($name:ident) => {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        pub(super) struct $name {
            query: String,
            authorized_corpus_ids: Vec<String>,
            stages: Vec<RagRetrievalStage>,
            warnings: Vec<String>,
            evidence: Vec<RagEvidenceView>,
            graph_context: Vec<GraphEvidenceView>,
            duration_ms: u64,
        }
        impl From<RagQueryResult> for $name {
            fn from(result: RagQueryResult) -> Self {
                Self {
                    query: result.query,
                    authorized_corpus_ids: result.authorized_corpus_ids,
                    stages: result.stages,
                    warnings: result.warnings,
                    evidence: result.evidence,
                    graph_context: result.graph_context,
                    duration_ms: result.duration_ms,
                }
            }
        }
    };
}
response!(SearchResponse);
response!(RecallResponse);
