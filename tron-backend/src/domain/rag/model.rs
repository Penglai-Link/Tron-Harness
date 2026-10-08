use std::{collections::BTreeMap, error::Error, fmt};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagScopeFields {
    pub scope_type: String,
    pub scope_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagStatus {
    Enabled,
    Reserved,
    Disabled,
}

impl RagStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Reserved => "reserved",
            Self::Disabled => "disabled",
        }
    }
}

impl TryFrom<&str> for RagStatus {
    type Error = RagError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "enabled" => Ok(Self::Enabled),
            "reserved" => Ok(Self::Reserved),
            "disabled" => Ok(Self::Disabled),
            _ => Err(RagError::StorageFailure),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagStorageBackend {
    Sqlite,
    Graph,
}

impl RagStorageBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sqlite => "SQLITE",
            Self::Graph => "GRAPH",
        }
    }

    pub fn is_graph(&self) -> bool {
        *self == Self::Graph
    }
}

impl TryFrom<&str> for RagStorageBackend {
    type Error = RagError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "SQLITE" => Ok(Self::Sqlite),
            "GRAPH" => Ok(Self::Graph),
            _ => Err(RagError::StorageBindingInvalid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagStorageSegmentTarget {
    pub id: String,
    pub backend: RagStorageBackend,
    pub layout: String,
    pub table: Option<String>,
    pub namespace: Option<String>,
    pub node_label: Option<String>,
    pub code_strategy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagStorageBinding {
    pub segment_bindings: BTreeMap<String, String>,
    pub segment_targets: BTreeMap<String, RagStorageSegmentTarget>,
}

impl RagStorageBinding {
    pub fn target_for_segment(&self, segment: &str) -> Option<&RagStorageSegmentTarget> {
        let target_id = self.segment_bindings.get(segment).or_else(|| {
            if segment == "fulltext" {
                self.segment_bindings.get("textUnit")
            } else {
                None
            }
        })?;
        self.segment_targets.get(target_id)
    }

    pub fn sqlite_table_for_segment(&self, segment: &str) -> Option<&str> {
        let target = self.target_for_segment(segment)?;
        (target.backend == RagStorageBackend::Sqlite)
            .then_some(target.table.as_deref())
            .flatten()
    }

    pub fn diagnostics_table(&self) -> Option<&str> {
        self.sqlite_table_for_segment("diagnostics")
    }

    pub fn is_graph_backend(&self) -> bool {
        self.segment_bindings
            .iter()
            .filter(|(segment, _)| segment.as_str() != "diagnostics")
            .any(|(segment, _)| {
                self.target_for_segment(segment)
                    .is_some_and(|target| target.backend == RagStorageBackend::Graph)
            })
    }

    pub fn is_sqlite_backend(&self) -> bool {
        self.segment_bindings
            .iter()
            .filter(|(segment, _)| segment.as_str() != "diagnostics")
            .all(|(segment, _)| {
                self.target_for_segment(segment)
                    .is_some_and(|target| target.backend == RagStorageBackend::Sqlite)
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagCorpusRecord {
    pub corpus_id: String,
    pub display_name: String,
    pub status: RagStatus,
    pub description: Option<String>,
    pub capabilities: Value,
    pub filter_schema: Value,
    pub field_mapping: BTreeMap<String, String>,
    pub source_field_mapping: BTreeMap<String, String>,
    pub text_unit_field_mapping: BTreeMap<String, String>,
    pub storage_binding: RagStorageBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagError {
    SchemaIncompatible,
    CorpusNotFound,
    SourceNotFound,
    IngestRequestInvalid,
    IngestScopeInvalid,
    EmbeddingFailed,
    SearchRequestInvalid,
    SearchFilterInvalid,
    SearchScopeInvalid,
    SearchRouteUnavailable,
    DebugRequestInvalid,
    TracePayloadInvalid,
    StorageBindingInvalid,
    StorageFailure,
}

impl fmt::Display for RagError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SchemaIncompatible => formatter
                .write_str("旧 RAG 数据结构与统一入库版本不兼容；请使用新数据库。现有数据未删除。"),
            Self::CorpusNotFound => formatter.write_str("RAG corpus not found"),
            Self::SourceNotFound => formatter.write_str("RAG source not found"),
            Self::IngestRequestInvalid => formatter.write_str("RAG ingest request invalid"),
            Self::IngestScopeInvalid => formatter.write_str("RAG ingest scope invalid"),
            Self::EmbeddingFailed => formatter.write_str("RAG embedding failed"),
            Self::SearchRequestInvalid => formatter.write_str("RAG search request invalid"),
            Self::SearchFilterInvalid => formatter.write_str("RAG search filter invalid"),
            Self::SearchScopeInvalid => formatter.write_str("RAG search scope invalid"),
            Self::SearchRouteUnavailable => formatter.write_str("RAG search route unavailable"),
            Self::DebugRequestInvalid => formatter.write_str("RAG debug request invalid"),
            Self::TracePayloadInvalid => formatter.write_str("RAG trace payload invalid"),
            Self::StorageBindingInvalid => formatter.write_str("RAG storage binding invalid"),
            Self::StorageFailure => formatter.write_str("RAG storage failure"),
        }
    }
}

impl Error for RagError {}
