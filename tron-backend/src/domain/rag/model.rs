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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagProfileType {
    Ingest,
    Retrieval,
}

impl RagProfileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ingest => "INGEST",
            Self::Retrieval => "RETRIEVAL",
        }
    }
}

impl TryFrom<&str> for RagProfileType {
    type Error = RagError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "INGEST" => Ok(Self::Ingest),
            "RETRIEVAL" => Ok(Self::Retrieval),
            _ => Err(RagError::StorageFailure),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagStorageBinding {
    pub source_table: Option<String>,
    pub text_unit_table: Option<String>,
    pub diagnostics_table: Option<String>,
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
    pub storage_binding: RagStorageBinding,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RagProfileRecord {
    pub profile_id: String,
    pub profile_type: RagProfileType,
    pub status: RagStatus,
    pub primary_corpus_id: Option<String>,
    pub corpus_ids: Vec<String>,
    pub required_source_version: Option<String>,
    pub config: Value,
    pub filters: Value,
    pub degrade_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagError {
    CorpusNotFound,
    ProfileNotFound,
    RetrievalProfileNotFound,
    IngestRequestInvalid,
    IngestProfileInvalid,
    IngestScopeInvalid,
    IngestStageFailed,
    EmbeddingFailed,
    SearchRequestInvalid,
    RetrievalProfileInvalid,
    SearchFilterInvalid,
    SearchScopeInvalid,
    SearchRouteUnavailable,
    HydrateSourceInactive,
    RerankFailed,
    CorpusReserved,
    StorageBindingInvalid,
    StorageFailure,
}

impl fmt::Display for RagError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorpusNotFound => formatter.write_str("RAG corpus not found"),
            Self::ProfileNotFound => formatter.write_str("RAG profile not found"),
            Self::RetrievalProfileNotFound => {
                formatter.write_str("RAG retrieval profile not found")
            }
            Self::IngestRequestInvalid => formatter.write_str("RAG ingest request invalid"),
            Self::IngestProfileInvalid => formatter.write_str("RAG ingest profile invalid"),
            Self::IngestScopeInvalid => formatter.write_str("RAG ingest scope invalid"),
            Self::IngestStageFailed => formatter.write_str("RAG ingest stage failed"),
            Self::EmbeddingFailed => formatter.write_str("RAG embedding failed"),
            Self::SearchRequestInvalid => formatter.write_str("RAG search request invalid"),
            Self::RetrievalProfileInvalid => formatter.write_str("RAG retrieval profile invalid"),
            Self::SearchFilterInvalid => formatter.write_str("RAG search filter invalid"),
            Self::SearchScopeInvalid => formatter.write_str("RAG search scope invalid"),
            Self::SearchRouteUnavailable => formatter.write_str("RAG search route unavailable"),
            Self::HydrateSourceInactive => formatter.write_str("RAG hydrate source inactive"),
            Self::RerankFailed => formatter.write_str("RAG rerank failed"),
            Self::CorpusReserved => formatter.write_str("RAG corpus reserved"),
            Self::StorageBindingInvalid => formatter.write_str("RAG storage binding invalid"),
            Self::StorageFailure => formatter.write_str("RAG storage failure"),
        }
    }
}

impl Error for RagError {}
