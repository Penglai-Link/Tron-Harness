use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptCategory {
    System,
    Persona,
    Tool,
    Template,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptScopeKind {
    Platform,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptSourceKind {
    Platform,
    WorkspaceLocal,
    WorkspaceOverride,
    WorkspaceStatusPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptViewSourceKind {
    Platform,
    Local,
    Inherited,
    Override,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: PromptVariableType,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptVariableType {
    String,
    Number,
    Boolean,
    Json,
}

impl PromptVariableType {
    pub fn code(self) -> &'static str {
        match self {
            Self::String => "STRING",
            Self::Number => "NUMBER",
            Self::Boolean => "BOOLEAN",
            Self::Json => "JSON",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub id: SnowflakeId,
    pub scope_kind: PromptScopeKind,
    pub workspace_code: Option<String>,
    pub prompt_code: String,
    pub display_name: String,
    pub category: PromptCategory,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub source_kind: PromptSourceKind,
    pub source_prompt_code: Option<String>,
    pub allow_inheritance: Option<bool>,
    pub version: u64,
    pub variables: Vec<PromptVariable>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptWorkspacePathItem {
    pub workspace_code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptView {
    pub scope_kind: PromptScopeKind,
    pub workspace_code: Option<String>,
    pub prompt_code: String,
    pub display_name: String,
    pub category: PromptCategory,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub source_kind: PromptViewSourceKind,
    pub source_label: String,
    pub source_prompt_code: Option<String>,
    pub source_workspace_code: Option<String>,
    pub source_workspace_name: Option<String>,
    pub source_workspace_path: Vec<PromptWorkspacePathItem>,
    pub allow_inheritance: Option<bool>,
    pub version: u64,
    pub variables: Vec<PromptVariable>,
    pub created_at: String,
    pub updated_at: String,
}
