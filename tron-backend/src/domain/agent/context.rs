use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    AgentKind, AgentMessage, AgentModelConfig, ExpandedDomainPersistenceMode, ToolDomainRecord,
    ToolExposureMode, ToolObservation,
};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolExposureState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_exposure_mode: Option<ToolExposureMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_domain_persistence_mode: Option<ExpandedDomainPersistenceMode>,
    #[serde(default)]
    pub visible_domains: Vec<ToolDomainRecord>,
    /// Effective tool schemas withheld from the current model request until
    /// their domain is expanded. This is an exposure concern, not permission.
    #[serde(default)]
    pub hidden_tools: Vec<EffectiveTool>,
    #[serde(default)]
    pub expanded_domain_codes: Vec<String>,
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
    pub max_steps: u32,
    pub max_tool_calls: u32,
    #[serde(default = "default_max_parallel_tool_calls")]
    pub max_parallel_tool_calls: u32,
}

impl Default for AgentRunLimits {
    fn default() -> Self {
        Self {
            max_steps: 100,
            max_tool_calls: 100,
            max_parallel_tool_calls: default_max_parallel_tool_calls(),
        }
    }
}

fn default_max_parallel_tool_calls() -> u32 {
    4
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
    #[serde(default)]
    pub tool_exposure: AgentToolExposureState,
    pub execution_scope: AgentExecutionScope,
    pub limits: AgentRunLimits,
    pub messages: Vec<AgentMessage>,
    pub observations: Vec<ToolObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_term_memory: Option<AgentShortTermMemoryRunContext>,
    pub started_at: String,
}

impl AgentRunContext {
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
        let explicitly_authorized = self
            .authorized_tools()
            .iter()
            .any(|tool| tool.tool_name == tool_name);
        let hidden_but_policy_authorized = self.tool_exposure.tool_exposure_mode
            == Some(ToolExposureMode::DomainDiscovery)
            && self
                .tool_exposure
                .hidden_tools
                .iter()
                .any(|tool| tool.tool_name == tool_name);

        explicitly_authorized || hidden_but_policy_authorized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(name: &str, domain: &str) -> EffectiveTool {
        EffectiveTool {
            tool_name: name.to_string(),
            domain_code: Some(domain.to_string()),
            description: String::new(),
            input_schema: json!({"type": "object"}),
            skill_name: None,
            required_variable_keys: Vec::new(),
        }
    }

    #[test]
    fn domain_discovery_hidden_tools_remain_policy_authorized_without_effective_snapshot() {
        let context = AgentRunContext {
            collaboration_owner: None,
            agent_code: None,
            conversation_code: None,
            run_code: "run".into(),
            workspace_code: "workspace".into(),
            user_code: None,
            locale: "en-US".into(),
            request_id: None,
            stream_key: None,
            agent_kind: super::super::AgentKind::React,
            system_agent_session_id: None,
            model: AgentModelConfig {
                model_code: "test".into(),
                protocol: super::super::AgentModelProtocol::ChatCompletions,
                base_url: String::new(),
                api_path: None,
                api_key: String::new(),
                model_name: "test".into(),
                thinking_enabled: false,
                reasoning_effort: None,
                responses_stateful_enabled: false,
                temperature: None,
                max_tokens: None,
                context_window_tokens: None,
                top_p: None,
                max_retries: None,
            },
            auto_model_route: None,
            variables: Vec::new(),
            skills: Vec::new(),
            tools: vec![tool("discovery_expand_tool_domain", "discovery")],
            effective_tools: Vec::new(),
            tool_exposure: AgentToolExposureState {
                tool_exposure_mode: Some(ToolExposureMode::DomainDiscovery),
                hidden_tools: vec![tool("bash_exec", "bash")],
                ..AgentToolExposureState::default()
            },
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: Vec::new(),
            observations: Vec::new(),
            short_term_memory: None,
            started_at: String::new(),
        };

        assert!(context.is_tool_authorized("bash_exec"));
        assert!(context.is_tool_authorized("discovery_expand_tool_domain"));
        assert!(!context.is_tool_authorized("workspace_delete"));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutoModelCandidate {
    pub selection: crate::domain::model_provider::ModelSelection,
    pub config: super::AgentModelConfig,
}

#[cfg(test)]
mod auto_route_tests {
    use super::*;

    #[test]
    fn initial_snapshot_is_cloned_in_memory_but_never_persists_credentials() {
        let config: super::super::AgentModelConfig = serde_json::from_value(serde_json::json!({
            "modelCode": "local-model", "protocol": "CHAT_COMPLETIONS",
            "baseUrl": "https://fixture.test", "apiKey": "snapshot-secret", "modelName": "Local"
        }))
        .unwrap();
        let route = AutoModelRouteContext {
            candidate_model_codes: Vec::new(),
            authorization_scope: Some("account-scope".into()),
            authorized_candidates: vec![AutoModelCandidate {
                selection: crate::domain::model_provider::ModelSelection::fixed("local-model"),
                config,
            }],
            session_code: "session".into(),
            agent_code: "agent".into(),
            current_candidate_key: "candidate".into(),
            failed_candidate_keys: vec![],
            fallback_attempts: 0,
        };
        assert_eq!(route.clone().authorized_candidates.len(), 1);
        let json = serde_json::to_string(&route).unwrap();
        assert!(!json.contains("snapshot-secret"));
        assert!(!json.contains("authorizedCandidates"));
        let resumed: AutoModelRouteContext = serde_json::from_str(&json).unwrap();
        assert!(resumed.authorized_candidates.is_empty());
    }
}
