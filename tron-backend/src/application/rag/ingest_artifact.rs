use serde_json::{Value, json};

use crate::domain::rag::RagError;

use super::{
    RagIngestRequest, RagIngestScope, RagIngestWarning, RagResolvedDocument, RagTextUnitWrite,
    RagTreeNodeWrite, ingest_payload::PreparedIngestPayload,
};

pub(crate) const ARTIFACT_RAW_SOURCE: &str = "RAW_SOURCE";
pub(crate) const ARTIFACT_VALIDATED_SOURCE: &str = "VALIDATED_SOURCE";
pub(crate) const ARTIFACT_RESOLVED_DOCUMENT: &str = "RESOLVED_DOCUMENT";
pub(crate) const ARTIFACT_PARSED_DOCUMENT: &str = "PARSED_DOCUMENT";
pub(crate) const ARTIFACT_TEXT_UNIT_BATCH: &str = "TEXT_UNIT_BATCH";
pub(crate) const ARTIFACT_FULLTEXT_INDEX_RESULT: &str = "FULLTEXT_INDEX_RESULT";
pub(crate) const ARTIFACT_VECTOR_INDEX_RESULT: &str = "VECTOR_INDEX_RESULT";
pub(crate) const ARTIFACT_TREE_GRAPH: &str = "TREE_GRAPH";
pub(crate) const ARTIFACT_TREE_SUMMARY_RESULT: &str = "TREE_SUMMARY_RESULT";
pub(crate) const ARTIFACT_TREE_FULLTEXT_INDEX_RESULT: &str = "TREE_FULLTEXT_INDEX_RESULT";
pub(crate) const ARTIFACT_TREE_VECTOR_INDEX_RESULT: &str = "TREE_VECTOR_INDEX_RESULT";
pub(crate) const ARTIFACT_TREE_PERSIST_RESULT: &str = "TREE_PERSIST_RESULT";
pub(crate) const ARTIFACT_PERSIST_RESULT: &str = "PERSIST_RESULT";

#[derive(Debug, Clone)]
pub(crate) struct ArtifactBundle {
    artifacts: Vec<RagArtifact>,
}

impl ArtifactBundle {
    pub(crate) fn from_request(request: &RagIngestRequest) -> Self {
        let mut bundle = Self {
            artifacts: Vec::new(),
        };
        bundle.append(
            request,
            ARTIFACT_RAW_SOURCE,
            RagArtifactPayload::RawSource(request.payload.clone()),
            json!({ "sourceType": request.source.source_type }),
        );
        bundle
    }

    pub(crate) fn append(
        &mut self,
        request: &RagIngestRequest,
        artifact_type: &str,
        payload: RagArtifactPayload,
        metadata: Value,
    ) {
        let ordinal = self.artifacts.len();
        self.artifacts.push(RagArtifact {
            artifact_id: format!(
                "{}:{}:{}:{}",
                request.source.code, request.source.version, artifact_type, ordinal
            ),
            artifact_type: artifact_type.to_string(),
            corpus_id: request.corpus_id.clone(),
            source_code: request.source.code.clone(),
            source_version: request.source.version.clone(),
            scope: request.target_scope.clone(),
            payload,
            schema_version: "v1".into(),
            metadata,
        });
    }

    pub(crate) fn raw_source_payload(&self) -> Result<&Value, RagError> {
        match &self.require(ARTIFACT_RAW_SOURCE)?.payload {
            RagArtifactPayload::RawSource(payload) => Ok(payload),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn validated_source_payload(&self) -> Result<&Value, RagError> {
        match &self.require(ARTIFACT_VALIDATED_SOURCE)?.payload {
            RagArtifactPayload::ValidatedSource(payload) => Ok(payload),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn resolved_document(&self) -> Result<&RagResolvedDocument, RagError> {
        match &self.require(ARTIFACT_RESOLVED_DOCUMENT)?.payload {
            RagArtifactPayload::ResolvedDocument(document) => Ok(document),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn parsed_document(&self) -> Result<&RagResolvedDocument, RagError> {
        match &self.require(ARTIFACT_PARSED_DOCUMENT)?.payload {
            RagArtifactPayload::ParsedDocument(document) => Ok(document),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn prepared_payload(&self) -> Result<&PreparedIngestPayload, RagError> {
        match &self.require(ARTIFACT_TEXT_UNIT_BATCH)?.payload {
            RagArtifactPayload::PreparedPayload(prepared) => Ok(prepared),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn vector_result(&self) -> Result<&RagVectorIndexArtifact, RagError> {
        match &self.require(ARTIFACT_VECTOR_INDEX_RESULT)?.payload {
            RagArtifactPayload::VectorIndexResult(result) => Ok(result),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn tree_vector_result(&self) -> Result<&RagTreeIndexArtifact, RagError> {
        match &self.require(ARTIFACT_TREE_VECTOR_INDEX_RESULT)?.payload {
            RagArtifactPayload::TreeVectorIndexResult(result) => Ok(result),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn tree_summary_result(&self) -> Result<&RagTreeIndexArtifact, RagError> {
        match &self.require(ARTIFACT_TREE_SUMMARY_RESULT)?.payload {
            RagArtifactPayload::TreeSummaryResult(result) => Ok(result),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn tree_fulltext_result(&self) -> Result<&RagTreeIndexArtifact, RagError> {
        match &self.require(ARTIFACT_TREE_FULLTEXT_INDEX_RESULT)?.payload {
            RagArtifactPayload::TreeFulltextIndexResult(result) => Ok(result),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn tree_persist_result(&self) -> Result<&RagTreeIndexArtifact, RagError> {
        match &self.require(ARTIFACT_TREE_PERSIST_RESULT)?.payload {
            RagArtifactPayload::TreePersistResult(result) => Ok(result),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    pub(crate) fn persist_result(&self) -> Result<&RagPersistResultArtifact, RagError> {
        match &self.require(ARTIFACT_PERSIST_RESULT)?.payload {
            RagArtifactPayload::PersistResult(result) => Ok(result),
            _ => Err(RagError::IngestStageFailed),
        }
    }

    fn require(&self, artifact_type: &str) -> Result<&RagArtifact, RagError> {
        self.artifacts
            .iter()
            .rev()
            .find(|artifact| artifact.artifact_type == artifact_type)
            .ok_or(RagError::IngestStageFailed)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct RagArtifact {
    pub(crate) artifact_id: String,
    pub(crate) artifact_type: String,
    pub(crate) corpus_id: String,
    pub(crate) source_code: String,
    pub(crate) source_version: String,
    pub(crate) scope: RagIngestScope,
    pub(crate) payload: RagArtifactPayload,
    pub(crate) schema_version: String,
    pub(crate) metadata: Value,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum RagArtifactPayload {
    RawSource(Value),
    ValidatedSource(Value),
    ResolvedDocument(RagResolvedDocument),
    ParsedDocument(RagResolvedDocument),
    PreparedPayload(PreparedIngestPayload),
    FulltextIndexResult { text_unit_count: usize },
    VectorIndexResult(RagVectorIndexArtifact),
    TreeGraph(Value),
    TreeSummaryResult(RagTreeIndexArtifact),
    TreeFulltextIndexResult(RagTreeIndexArtifact),
    TreeVectorIndexResult(RagTreeIndexArtifact),
    TreePersistResult(RagTreeIndexArtifact),
    PersistResult(RagPersistResultArtifact),
}

#[derive(Debug, Clone)]
pub(crate) struct RagVectorIndexArtifact {
    pub(crate) text_units: Vec<RagTextUnitWrite>,
    pub(crate) embedding_status: String,
    pub(crate) warnings: Vec<RagIngestWarning>,
    pub(crate) fatal_error: Option<RagError>,
}

#[derive(Debug, Clone)]
pub(crate) struct RagTreeIndexArtifact {
    pub(crate) tree_nodes: Vec<RagTreeNodeWrite>,
    pub(crate) embedding_status: String,
    pub(crate) warnings: Vec<RagIngestWarning>,
    pub(crate) fatal_error: Option<RagError>,
}

#[derive(Debug, Clone)]
pub(crate) struct RagPersistResultArtifact {
    pub(crate) source_status: String,
    pub(crate) text_unit_count: usize,
    pub(crate) fulltext_status: String,
    pub(crate) embedding_status: String,
    pub(crate) warnings: Vec<RagIngestWarning>,
    pub(crate) fatal_error: Option<RagError>,
}
