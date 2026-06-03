use serde::Deserialize;
use serde_json::Value;

use crate::application::rag::{
    RagDebugSourceRequest, RagIngestAccessContext, RagIngestRequest, RagIngestScope,
    RagIngestSource, RagRetryIndexRequest, RagSearchContextRequest, RagSearchRequest,
    RagSearchTraceRequest,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagIngestHttpRequest {
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub ingest_profile_id: Option<String>,
    #[serde(default)]
    pub access_context: Option<RagIngestAccessContextRequest>,
    #[serde(default)]
    pub target_scope: Option<RagIngestScopeRequest>,
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
        let source = ingest_source_from_payload(&self.payload);
        let workspace_code = access_context.workspace_code.unwrap_or_default();
        RagIngestRequest {
            corpus_id: self.corpus_id.unwrap_or_default(),
            ingest_profile_id: self.ingest_profile_id.unwrap_or_default(),
            workspace_code: workspace_code.clone(),
            access_context: RagIngestAccessContext {
                user_code: access_context.user_code.unwrap_or_default(),
                agent_code: access_context.agent_code.unwrap_or_default(),
                workspace_code,
            },
            target_scope: RagIngestScope {
                scope_type: target_scope.scope_type.unwrap_or_default(),
                scope_code: target_scope.scope_code.unwrap_or_default(),
            },
            source,
            payload: self.payload,
        }
    }
}

fn ingest_source_from_payload(payload: &Value) -> RagIngestSource {
    let source = payload.get("source");
    RagIngestSource {
        code: payload_source_string(source, "code"),
        version: payload_source_string(source, "version"),
        source_type: payload_source_string(source, "type"),
    }
}

fn payload_source_string(source: Option<&Value>, field: &str) -> String {
    source
        .and_then(|source| source.get(field))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchHttpRequest {
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub access_context: Option<RagIngestAccessContextRequest>,
    #[serde(default)]
    pub filters: Value,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub token_budget: Option<usize>,
    #[serde(default)]
    pub context: Option<RagSearchContextHttpRequest>,
    #[serde(default)]
    pub trace: Option<RagSearchTraceHttpRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchContextHttpRequest {
    #[serde(default)]
    pub strategy: Option<String>,
    #[serde(default)]
    pub return_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchTraceHttpRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
}

impl RagSearchHttpRequest {
    pub fn into_application(self) -> RagSearchRequest {
        let access_context = self.access_context.unwrap_or_default();
        let context = self.context.unwrap_or_default();
        let trace = self.trace.unwrap_or_default();
        let workspace_code = access_context.workspace_code.unwrap_or_default();
        RagSearchRequest {
            profile_id: self.profile_id.unwrap_or_default(),
            workspace_code: workspace_code.clone(),
            query: self.query.unwrap_or_default(),
            access_context: RagIngestAccessContext {
                user_code: access_context.user_code.unwrap_or_default(),
                agent_code: access_context.agent_code.unwrap_or_default(),
                workspace_code,
            },
            filters: self.filters,
            limit: self.limit.unwrap_or(10),
            token_budget: self.token_budget.unwrap_or(1200),
            context: RagSearchContextRequest {
                strategy: context.strategy.unwrap_or_else(|| "RESULT_LIST".into()),
                return_fields: context
                    .return_fields
                    .into_iter()
                    .map(|field| field.trim().to_string())
                    .filter(|field| !field.is_empty())
                    .collect(),
            },
            trace: RagSearchTraceRequest {
                enabled: trace.enabled.unwrap_or(false),
            },
        }
    }
}

impl Default for RagSearchContextHttpRequest {
    fn default() -> Self {
        Self {
            strategy: None,
            return_fields: Vec::new(),
        }
    }
}

impl Default for RagSearchTraceHttpRequest {
    fn default() -> Self {
        Self { enabled: None }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagDebugProfileHttpQuery {
    #[serde(default)]
    pub workspace_code: Option<String>,
    #[serde(default)]
    pub profile_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagDebugSourceHttpQuery {
    #[serde(default)]
    pub workspace_code: Option<String>,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub source_code: Option<String>,
    #[serde(default)]
    pub source_version: Option<String>,
    #[serde(default)]
    pub scope_type: Option<String>,
    #[serde(default)]
    pub scope_code: Option<String>,
    #[serde(default)]
    pub include_text_units: Option<bool>,
    #[serde(default)]
    pub include_warnings: Option<bool>,
}

impl RagDebugSourceHttpQuery {
    pub fn into_application(self) -> RagDebugSourceRequest {
        RagDebugSourceRequest {
            workspace_code: self.workspace_code.unwrap_or_default(),
            corpus_id: self.corpus_id.unwrap_or_default(),
            source_code: self.source_code.unwrap_or_default(),
            source_version: self.source_version.unwrap_or_default(),
            scope_type: self.scope_type.unwrap_or_default(),
            scope_code: self.scope_code.unwrap_or_default(),
            include_text_units: self.include_text_units.unwrap_or(true),
            include_warnings: self.include_warnings.unwrap_or(true),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagRetryIndexHttpRequest {
    #[serde(default)]
    pub workspace_code: Option<String>,
    #[serde(default)]
    pub corpus_id: Option<String>,
    #[serde(default)]
    pub source_code: Option<String>,
    #[serde(default)]
    pub source_version: Option<String>,
    #[serde(default)]
    pub scope_type: Option<String>,
    #[serde(default)]
    pub scope_code: Option<String>,
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub embedding_model_code: Option<String>,
}

impl RagRetryIndexHttpRequest {
    pub fn into_application(self, user_code: String, agent_code: String) -> RagRetryIndexRequest {
        RagRetryIndexRequest {
            workspace_code: self.workspace_code.unwrap_or_default(),
            corpus_id: self.corpus_id.unwrap_or_default(),
            source_code: self.source_code.unwrap_or_default(),
            source_version: self.source_version.unwrap_or_default(),
            scope_type: self.scope_type.unwrap_or_default(),
            scope_code: self.scope_code.unwrap_or_default(),
            stages: self.stages,
            embedding_model_code: self.embedding_model_code,
            user_code,
            agent_code,
        }
    }
}
