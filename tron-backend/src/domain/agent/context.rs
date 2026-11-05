use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{AgentKind, AgentMessage, AgentModelConfig, ToolObservation};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolExecutionMode {
    #[default]
    Controlled,
    AutoApprove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveVariable {
    pub key: String,
    pub value: String,
    pub source: String,
    #[serde(default)]
    pub sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveSkill {
    pub skill_name: String,
    pub description: String,
    pub instruction: Option<String>,
    pub tool_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveTool {
    pub tool_name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_code: Option<String>,
    pub description: String,
    pub input_schema: Value,
    pub skill_name: Option<String>,
    pub required_variable_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecutionScope {
    #[serde(default)]
    pub applications: Vec<super::ApplicationDirectoryContext>,
    #[serde(default)]
    pub project_code: Option<String>,
    pub app_code: Option<String>,
    pub repository_url: Option<String>,
    pub local_directory: Option<String>,
    pub branch_name: Option<String>,
    pub task_code: Option<String>,
    #[serde(default)]
    pub allowed_local_directories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentShortTermMemoryRunContext {
    pub session_code: String,
    pub input_code: String,
    pub input_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_metadata: Option<Value>,
    pub prompt_prefix: Vec<AgentMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_prefix_sources: Vec<Option<AgentPromptPrefixSource>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_input_message: Option<AgentMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_code: Option<String>,
    #[serde(default)]
    pub todo_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPromptPrefixSource {
    pub catalog_key: String,
    pub prompt_code: String,
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoModelRouteContext {
    #[serde(default)]
    pub candidate_model_codes: Vec<String>,
    /// In-memory initial authorization snapshot; never persist credentials.
    /// Resumed runs without a snapshot must re-read authorization.
    #[serde(skip)]
    pub authorized_candidates: Vec<AutoModelCandidate>,
    #[serde(skip)]
    pub authorization_scope: Option<String>,
    pub session_code: String,
    pub agent_code: String,
    pub current_candidate_key: String,
    #[serde(default)]
    pub failed_candidate_keys: Vec<String>,
    #[serde(default)]
    pub fallback_attempts: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunLimits {
    /// Zero means unlimited model steps for this run.
    pub max_steps: u32,
    /// Zero means unlimited tool invocations for this run.
    pub max_tool_calls: u32,
    #[serde(default = "default_max_parallel_tool_calls")]
    pub max_parallel_tool_calls: u32,
}

impl Default for AgentRunLimits {
    fn default() -> Self {
        Self {
            max_steps: 0,
            max_tool_calls: 0,
            max_parallel_tool_calls: default_max_parallel_tool_calls(),
        }
    }
}

fn default_max_parallel_tool_calls() -> u32 {
    10
}

/// Trusted ownership supplied by the host; never accepted from tool arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCollaborationOwner {
    pub kind: String,
    pub code: String,
    pub origin_session_code: Option<String>,
    pub task_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunContext {
    /// Selected by the user on the owning session, never by tool arguments.
    #[serde(default)]
    pub tool_execution_mode: ToolExecutionMode,
    #[serde(default)]
    pub collaboration_owner: Option<AgentCollaborationOwner>,
    /// Server-resolved identity; older saved runs omit it and receive no KB context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_code: Option<String>,
    pub conversation_code: Option<String>,
    pub run_code: String,
    pub workspace_code: String,
    pub user_code: Option<String>,
    pub locale: String,
    pub request_id: Option<String>,
    pub stream_key: Option<String>,
    pub agent_kind: AgentKind,
    pub system_agent_session_id: Option<String>,
    #[serde(default)]
    pub system_agent_fast_mode: bool,
    pub model: AgentModelConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_model_route: Option<AutoModelRouteContext>,
    pub variables: Vec<EffectiveVariable>,
    pub skills: Vec<EffectiveSkill>,
    /// Tool schemas currently exposed to the model.
    pub tools: Vec<EffectiveTool>,
    /// Complete tool set authorized by the resolved agent policy.
    ///
    /// Older manually-constructed contexts may omit this field. In that case
    /// `authorized_tools` falls back to the exposed tool set.
    #[serde(default)]
    pub effective_tools: Vec<EffectiveTool>,
    /// Server-resolved selection rule and optional request narrowing for this run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_assignment: Option<AgentToolAssignment>,
    #[serde(default)]
    pub execution_scope: AgentExecutionScope,
    pub limits: AgentRunLimits,
    /// Shared transcript snapshot. Clones across preparation, turn state and
    /// the model call are pointer copies instead of O(history) deep copies;
    /// mutations go through `Arc::make_mut` (copy-on-write only when shared).
    pub messages: std::sync::Arc<Vec<AgentMessage>>,
    pub observations: Vec<ToolObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_term_memory: Option<AgentShortTermMemoryRunContext>,
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolAssignment {
    pub mode: super::ToolAssignmentMode,
    #[serde(default)]
    pub selected_category_codes: Vec<String>,
    pub selected_tool_codes: Vec<String>,
    /// Empty means no additional request restriction, never an empty grant.
    pub requested_tool_names: Vec<String>,
}

impl AgentToolAssignment {
    pub fn selects(&self, tool: &super::ToolCatalogRecord) -> bool {
        self.mode.selects_policy(
            tool,
            &self.selected_tool_codes,
            &self.selected_category_codes,
        ) && (self.requested_tool_names.is_empty()
            || self.requested_tool_names.contains(&tool.tool_name))
    }
}

impl AgentRunContext {
    pub(crate) fn is_owning_dude(&self) -> bool {
        self.collaboration_owner.as_ref().is_some_and(|owner| {
            owner.kind == "DUDE"
                && self.agent_code.as_deref() == Some(owner.code.as_str())
                && owner.origin_session_code.is_some()
                && self.conversation_code == owner.origin_session_code
        })
    }

    pub(crate) fn is_platform_dude(&self) -> bool {
        self.workspace_code.trim().is_empty()
            && self
                .collaboration_owner
                .as_ref()
                .is_some_and(|owner| owner.kind == "DUDE" && !owner.code.trim().is_empty())
    }

    pub(crate) fn uses_platform_agent_definition(&self) -> bool {
        self.workspace_code.is_empty()
            || self.collaboration_owner.as_ref().is_some_and(|owner| {
                owner.kind.eq_ignore_ascii_case("DUDE")
                    && self.agent_code.as_deref() == Some(owner.code.as_str())
            })
    }

    pub(crate) fn memory_scope(
        &self,
        session: &str,
    ) -> Result<eliteforge_agent_memory::MemoryScope, eliteforge_agent_core::MemoryError> {
        if self.workspace_code.is_empty() {
            eliteforge_agent_memory::MemoryScope::platform(session)
        } else {
            eliteforge_agent_memory::MemoryScope::new(&self.workspace_code, session)
        }
    }

    pub fn authorized_tools(&self) -> &[EffectiveTool] {
        if self.effective_tools.is_empty() {
            &self.tools
        } else {
            &self.effective_tools
        }
    }

    pub fn is_tool_authorized(&self, tool_name: &str) -> bool {
        self.authorized_tools()
            .iter()
            .any(|tool| tool.tool_name == tool_name)
    }
}



#[derive(Debug, Clone, PartialEq)]
pub struct AutoModelCandidate {
    pub selection: crate::domain::model_provider::ModelSelection,
    pub config: super::AgentModelConfig,
}
