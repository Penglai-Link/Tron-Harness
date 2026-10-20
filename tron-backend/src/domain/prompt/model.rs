use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptCategory {
    System,
    Developer,
    Persona,
    AgentRole,
    Tool,
    Template,
    Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub variable_type: PromptVariableType,
    pub description: String,
    pub source_kind: PromptVariableSourceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_variable_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_provider: Option<PromptRuntimeProvider>,
}

pub type PromptParameter = PromptVariable;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTranslation {
    pub locale: String,
    pub display_name: String,
    pub prompt_body: String,
    pub description: Option<String>,
    pub status: PromptTranslationStatus,
    pub source_version: u64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptTranslationStatus {
    Published,
    Draft,
    UpdateAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptVariableType {
    String,
    Number,
    Boolean,
    Json,
}

/// The authority that provides a prompt placeholder value.
///
/// Managed variables are resolved from Variable Management immediately before
/// the prompt is rendered. Runtime variables are reserved for explicitly
/// registered server-side providers and are never arbitrary caller input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptVariableSourceKind {
    Managed,
    Runtime,
}

impl PromptVariableSourceKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::Managed => "MANAGED",
            Self::Runtime => "RUNTIME",
        }
    }
}

/// A server-owned source that supplies a dynamic prompt value at execution
/// time. These providers are registered in the bundled prompt catalog; callers
/// cannot define arbitrary runtime sources in a prompt request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptRuntimeProvider {
    AgentVisibleDomains,
    AgentDomainPersistenceMode,
    ChatUserMessage,
    TerminalExplainCommand,
    TerminalExplainOutput,
    TerminalCurrentCommand,
    TerminalUserRequest,
    TaskContext,
    RagTreeTitle,
    RagTreeSourceText,
    RagTreeChildSummaries,
}

impl PromptRuntimeProvider {
    pub fn code(self) -> &'static str {
        match self {
            Self::AgentVisibleDomains => "AGENT_VISIBLE_DOMAINS",
            Self::AgentDomainPersistenceMode => "AGENT_DOMAIN_PERSISTENCE_MODE",
            Self::ChatUserMessage => "CHAT_USER_MESSAGE",
            Self::TerminalExplainCommand => "TERMINAL_EXPLAIN_COMMAND",
            Self::TerminalExplainOutput => "TERMINAL_EXPLAIN_OUTPUT",
            Self::TerminalCurrentCommand => "TERMINAL_CURRENT_COMMAND",
            Self::TerminalUserRequest => "TERMINAL_USER_REQUEST",
            Self::TaskContext => "TASK_CONTEXT",
            Self::RagTreeTitle => "RAG_TREE_TITLE",
            Self::RagTreeSourceText => "RAG_TREE_SOURCE_TEXT",
            Self::RagTreeChildSummaries => "RAG_TREE_CHILD_SUMMARIES",
        }
    }

    pub fn from_code(value: &str) -> Option<Self> {
        match value {
            "AGENT_VISIBLE_DOMAINS" => Some(Self::AgentVisibleDomains),
            "AGENT_DOMAIN_PERSISTENCE_MODE" => Some(Self::AgentDomainPersistenceMode),
            "CHAT_USER_MESSAGE" => Some(Self::ChatUserMessage),
            "TERMINAL_EXPLAIN_COMMAND" => Some(Self::TerminalExplainCommand),
            "TERMINAL_EXPLAIN_OUTPUT" => Some(Self::TerminalExplainOutput),
            "TERMINAL_CURRENT_COMMAND" => Some(Self::TerminalCurrentCommand),
            "TERMINAL_USER_REQUEST" => Some(Self::TerminalUserRequest),
            "TASK_CONTEXT" => Some(Self::TaskContext),
            "RAG_TREE_TITLE" => Some(Self::RagTreeTitle),
            "RAG_TREE_SOURCE_TEXT" => Some(Self::RagTreeSourceText),
            "RAG_TREE_CHILD_SUMMARIES" => Some(Self::RagTreeChildSummaries),
            _ => None,
        }
    }

    pub fn source_label(self) -> &'static str {
        match self {
            Self::AgentVisibleDomains => "Agent tool domains",
            Self::AgentDomainPersistenceMode => "Agent domain persistence mode",
            Self::ChatUserMessage => "Chat user message",
            Self::TerminalExplainCommand => "Terminal command",
            Self::TerminalExplainOutput => "Terminal output",
            Self::TerminalCurrentCommand => "Terminal current command",
            Self::TerminalUserRequest => "Terminal user request",
            Self::TaskContext => "Task context",
            Self::RagTreeTitle => "RAG tree title",
            Self::RagTreeSourceText => "RAG tree source text",
            Self::RagTreeChildSummaries => "RAG tree child summaries",
        }
    }
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
    pub prompt_code: String,
    pub display_name: String,
    pub category: PromptCategory,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: u64,
    pub variables: Vec<PromptVariable>,
    pub translations: Vec<PromptTranslation>,
    pub built_in: bool,
    pub runtime_required: bool,
    pub catalog_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptView {
    pub prompt_code: String,
    pub display_name: String,
    pub category: PromptCategory,
    pub prompt_body: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: u64,
    pub variables: Vec<PromptVariable>,
    pub translations: Vec<PromptTranslation>,
    pub built_in: bool,
    pub runtime_required: bool,
    pub catalog_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
