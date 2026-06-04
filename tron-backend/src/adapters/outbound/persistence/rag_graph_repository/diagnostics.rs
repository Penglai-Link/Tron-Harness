use serde_json::json;

use crate::{
    application::rag::{
        RagDebugSourceRecord, RagDebugSourceRequest, RagDebugSourceResult, RagDebugTextUnitRecord,
        RagRetryIndexRequest,
    },
    domain::rag::{RagCorpusRecord, RagError},
};

use super::{
    GraphBackedRagRepository,
    model::{prop_i64, prop_text},
};

impl GraphBackedRagRepository {
    pub(super) fn debug_graph_source(
        &self,
        corpus: &RagCorpusRecord,
        request: &RagDebugSourceRequest,
    ) -> Result<Option<RagDebugSourceResult>, RagError> {
        let retry_request = RagRetryIndexRequest {
            workspace_code: request.workspace_code.clone(),
            corpus_id: request.corpus_id.clone(),
            source_code: request.source_code.clone(),
            source_version: request.source_version.clone(),
            scope_type: request.scope_type.clone(),
            scope_code: request.scope_code.clone(),
            stages: vec!["fulltextIndex".into()],
            embedding_model_code: None,
            user_code: String::new(),
            agent_code: String::new(),
        };
        let Some(node) = self.find_graph_skill_node(corpus, &retry_request)? else {
            return Ok(None);
        };
        let props = &node.properties;
        let text_units = request
            .include_text_units
            .then(|| {
                vec![RagDebugTextUnitRecord {
                    text_unit_code: node.node_code.clone(),
                    ordinal: 0,
                    status: prop_text(props, "textUnitStatus")
                        .unwrap_or("ACTIVE")
                        .to_string(),
                    fulltext_status: prop_text(props, "fulltextStatus")
                        .unwrap_or("INDEXED")
                        .to_string(),
                    embedding_status: prop_text(props, "embeddingStatus")
                        .unwrap_or("FAILED")
                        .to_string(),
                    embedding_model_code: prop_text(props, "embeddingModelCode")
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    heading_path: prop_text(props, "headingPath").unwrap_or("").to_string(),
                    token_count: prop_i64(props, "tokenCount"),
                }]
            })
            .unwrap_or_default();
        Ok(Some(RagDebugSourceResult {
            source: RagDebugSourceRecord {
                corpus_id: request.corpus_id.clone(),
                source_code: request.source_code.clone(),
                source_version: request.source_version.clone(),
                scope_type: request.scope_type.clone(),
                scope_code: request.scope_code.clone(),
                status: prop_text(props, "sourceStatus")
                    .unwrap_or("ACTIVE")
                    .to_string(),
                title: prop_text(props, "title").map(ToOwned::to_owned),
                metadata: prop_text(props, "sourceMetadataJson")
                    .and_then(|value| serde_json::from_str(value).ok())
                    .unwrap_or_else(|| json!({})),
            },
            text_units,
            warnings: Vec::new(),
        }))
    }
}
