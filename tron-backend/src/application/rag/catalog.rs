use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::rag::RagError;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RagSettings {
    #[serde(default = "enabled")]
    pub fulltext: bool,
    #[serde(default = "enabled")]
    pub vector: bool,
    #[serde(default)]
    pub graph: bool,
    #[serde(default)]
    pub embedding_model_code: Option<String>,
    #[serde(default = "default_dimension")]
    pub embedding_dimension: usize,
    #[serde(default)]
    pub rerank_model_code: Option<String>,
    #[serde(default)]
    pub graph_model_code: Option<String>,
}
fn enabled() -> bool {
    true
}
fn default_dimension() -> usize {
    1024
}
impl Default for RagSettings {
    fn default() -> Self {
        Self {
            fulltext: true,
            vector: true,
            graph: false,
            embedding_model_code: None,
            embedding_dimension: 1024,
            rerank_model_code: None,
            graph_model_code: None,
        }
    }
}
impl RagSettings {
    pub fn from_capabilities(value: &Value) -> Result<Self, RagError> {
        value
            .get("settings")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map(|settings| settings.unwrap_or_default())
            .map_err(|_| RagError::StorageBindingInvalid)
    }
    pub fn validate(&self) -> Result<(), RagError> {
        if !(1..=8192).contains(&self.embedding_dimension) {
            return Err(RagError::SearchRequestInvalid);
        }
        if !self.fulltext && !self.vector && !self.graph {
            return Err(RagError::SearchRequestInvalid);
        }
        for value in [
            &self.embedding_model_code,
            &self.rerank_model_code,
            &self.graph_model_code,
        ]
        .into_iter()
        .flatten()
        {
            if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                return Err(RagError::SearchRequestInvalid);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagIndexCounts {
    pub source_count: usize,
    pub rebuild_source_count: usize,
    pub chunk_count: usize,
    pub vector_count: usize,
    pub graph_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRecordTask {
    pub id: String,
    pub corpus_id: String,
    pub user_code: String,
    pub workspace_code: String,
    pub source_code: String,
    pub source_version: String,
    pub scope_type: String,
    pub scope_code: String,
    pub payload: Value,
    pub checkpoint: Option<RagRecordCheckpoint>,
    pub state: String,
    pub stage: String,
    pub completed_units: usize,
    pub total_units: usize,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagTaskView {
    pub document_id: Option<String>,
    pub id: String,
    pub corpus_id: String,
    pub source_code: String,
    pub title: String,
    pub state: String,
    pub stage: String,
    pub completed_units: usize,
    pub total_units: usize,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
impl From<RagRecordTask> for RagTaskView {
    fn from(task: RagRecordTask) -> Self {
        Self {
            document_id: None,
            title: task.payload["title"]
                .as_str()
                .unwrap_or(&task.source_code)
                .into(),
            id: task.id,
            corpus_id: task.corpus_id,
            source_code: task.source_code,
            state: task.state,
            stage: task.stage,
            completed_units: task.completed_units,
            total_units: task.total_units,
            error: task.error,
            created_at: task.created_at,
            updated_at: task.updated_at,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RagCorpusSummary {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub knowledge_base_id: Option<String>,
    pub settings: RagSettings,
    pub counts: RagIndexCounts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRecordCheckpoint {
    pub settings: RagSettings,
    pub content_hash: String,
    pub vectors: super::RagPreparedVectors,
}
