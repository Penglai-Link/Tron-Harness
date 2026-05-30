use serde::Deserialize;
use serde_json::Value;

use crate::application::rag::{RagIngestAccessContext, RagIngestRequest, RagIngestScope};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagIngestHttpRequest {
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub ingest_profile_id: Option<String>,
    #[serde(default)]
    pub workspace_code: Option<String>,
    #[serde(default)]
    pub access_context: Option<RagIngestAccessContextRequest>,
    #[serde(default)]
    pub target_scope: Option<RagIngestScopeRequest>,
    #[serde(default)]
    pub source_code: Option<String>,
    #[serde(default)]
    pub source_version: Option<String>,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagIngestAccessContextRequest {
    #[serde(default)]
    pub user_code: Option<String>,
    #[serde(default)]
    pub agent_code: Option<String>,
    #[serde(default)]
    pub workspace_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagIngestScopeRequest {
    #[serde(default)]
    pub scope_type: Option<String>,
    #[serde(default)]
    pub scope_code: Option<String>,
}

impl RagIngestHttpRequest {
    pub fn into_application(self) -> RagIngestRequest {
        let access_context = self.access_context.unwrap_or_default();
        let target_scope = self.target_scope.unwrap_or_default();
        RagIngestRequest {
            corpus_id: self.corpus_id.unwrap_or_default(),
            ingest_profile_id: self.ingest_profile_id.unwrap_or_default(),
            workspace_code: self.workspace_code.unwrap_or_default(),
            access_context: RagIngestAccessContext {
                user_code: access_context.user_code.unwrap_or_default(),
                agent_code: access_context.agent_code.unwrap_or_default(),
                workspace_code: access_context.workspace_code.unwrap_or_default(),
            },
            target_scope: RagIngestScope {
                scope_type: target_scope.scope_type.unwrap_or_default(),
                scope_code: target_scope.scope_code.unwrap_or_default(),
            },
            source_code: self.source_code.unwrap_or_default(),
            source_version: self.source_version.unwrap_or_default(),
            source_type: self.source_type.unwrap_or_default(),
            payload: self.payload,
        }
    }
}

impl Default for RagIngestAccessContextRequest {
    fn default() -> Self {
        Self {
            user_code: None,
            agent_code: None,
            workspace_code: None,
        }
    }
}

impl Default for RagIngestScopeRequest {
    fn default() -> Self {
        Self {
            scope_type: None,
            scope_code: None,
        }
    }
}
