use crate::{
    application::agent::{AgentInvocationError, RuntimeAdapterSelection},
    domain::agent::{
        AgentMessage, AgentRunContext, AgentShortTermMemoryRunContext, ToolExposureMode,
    },
};

use super::AgentInvocationService;

impl AgentInvocationService {
    pub(super) fn activate_short_term_memory(
        &self,
        context: &mut AgentRunContext,
        runtime_selection: &RuntimeAdapterSelection,
        requested_conversation_code: Option<&str>,
        agent_code: &str,
        prompt: &str,
        prompt_prefix_messages: Vec<AgentMessage>,
        user_prompt_message: Option<AgentMessage>,
        input_kind: Option<&str>,
        input_metadata: Option<serde_json::Value>,
    ) -> Result<(), AgentInvocationError> {
        if !matches!(runtime_selection, RuntimeAdapterSelection::React)
            || requested_conversation_code.is_none()
        {
            return Ok(());
        }
        let Some(short_term_memory) = self.short_term_memory.as_ref() else {
            return Ok(());
        };

        let input_kind = input_kind.unwrap_or("USER_PROMPT");
        let input_code = short_term_memory
            .record_invocation_input(context, prompt, input_kind, input_metadata.clone())
            .map_err(|error| AgentInvocationError::RuntimeFailed(error.to_string()))?;
        context.messages = prompt_prefix_messages;
        let current_input_message =
            runtime_input_context_message(input_kind, prompt, &input_metadata)
                .or(user_prompt_message);
        context.short_term_memory = Some(AgentShortTermMemoryRunContext {
            session_code: requested_conversation_code
                .expect("conversation code should be present")
                .to_string(),
            input_code,
            input_kind: input_kind.to_string(),
            input_metadata,
            prompt_prefix: context.messages.clone(),
            current_input_message,
            agent_code: Some(agent_code.to_string()),
            todo_enabled: resolve_todo_enabled(context),
        });
        Ok(())
    }
}

fn runtime_input_context_message(
    input_kind: &str,
    prompt: &str,
    metadata: &Option<serde_json::Value>,
) -> Option<AgentMessage> {
    if input_kind == "USER_PROMPT" {
        return None;
    }
    let source = metadata
        .as_ref()
        .and_then(|value| value.get("source"))
        .and_then(|value| value.as_str())
        .unwrap_or("RUNTIME");
    Some(AgentMessage::context_block(
        "runtime_invocation_input",
        format!(
            "<runtime-invocation-input kind=\"{input_kind}\" source=\"{source}\">\n{prompt}\n</runtime-invocation-input>"
        ),
    ))
}

fn resolve_todo_enabled(context: &AgentRunContext) -> bool {
    let visible_tools_contain_todo = context
        .tools
        .iter()
        .any(|tool| tool.tool_name.eq_ignore_ascii_case("todo.update"));
    if visible_tools_contain_todo {
        return true;
    }
    if context.tool_exposure.tool_exposure_mode != Some(ToolExposureMode::DomainDiscovery) {
        return false;
    }
    context
        .tool_exposure
        .hidden_tools
        .iter()
        .any(|tool| tool.tool_name.eq_ignore_ascii_case("todo.update"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, AgentToolExposureState, EffectiveSkill, EffectiveTool,
        EffectiveVariable, ExpandedDomainPersistenceMode, ToolExposureMode,
    };

    use super::resolve_todo_enabled;

    #[test]
    fn todo_enabled_uses_domain_discovery_hidden_tools() {
        let mut context = context_with_tools(vec![tool("domain.expand")]);
        context.tool_exposure = AgentToolExposureState {
            tool_exposure_mode: Some(ToolExposureMode::DomainDiscovery),
            expanded_domain_persistence_mode: Some(
                ExpandedDomainPersistenceMode::EphemeralExpanded,
            ),
            visible_domains: Vec::new(),
            hidden_tools: vec![tool("todo.update")],
            expanded_domain_codes: Vec::new(),
        };

        assert!(resolve_todo_enabled(&context));
    }

    #[test]
    fn todo_disabled_when_direct_list_omits_todo_update() {
        let mut context = context_with_tools(vec![tool("domain.expand")]);
        context.tool_exposure = AgentToolExposureState {
            tool_exposure_mode: Some(ToolExposureMode::DirectList),
            expanded_domain_persistence_mode: None,
            visible_domains: Vec::new(),
            hidden_tools: vec![tool("todo.update")],
            expanded_domain_codes: Vec::new(),
        };

        assert!(!resolve_todo_enabled(&context));
    }

    #[test]
    fn non_user_transcript_input_becomes_runtime_context_block() {
        let message = super::runtime_input_context_message(
            "CONTINUATION",
            "Continue from the active goal.",
            &Some(json!({"source": "GOAL_LOOP"})),
        )
        .expect("continuation should be model-visible as runtime context");

        assert_eq!(message.role, "user");
        assert!(message.text_content().contains("kind=\"CONTINUATION\""));
        assert!(message.text_content().contains("source=\"GOAL_LOOP\""));
        assert!(
            message
                .text_content()
                .contains("Continue from the active goal.")
        );
    }

    #[test]
    fn user_prompt_does_not_become_runtime_context_block() {
        assert!(
            super::runtime_input_context_message("USER_PROMPT", "hello", &None).is_none(),
            "ordinary user prompts are represented by transcript messages"
        );
    }

    fn context_with_tools(tools: Vec<EffectiveTool>) -> AgentRunContext {
        AgentRunContext {
            conversation_code: Some("SESSION-1".into()),
            run_code: "RUN-1".into(),
            workspace_code: "TRON_CORE".into(),
            user_code: Some("USR-1".into()),
            locale: "en-US".into(),
            request_id: None,
            stream_key: None,
            agent_kind: AgentKind::React,
            system_agent_session_id: None,
            model: AgentModelConfig {
                model_code: "MODEL-1".into(),
                protocol: AgentModelProtocol::ChatCompletions,
                base_url: "http://127.0.0.1:1/v1".into(),
                api_path: None,
                api_key: "sk-test".into(),
                model_name: "gpt-test".into(),
                thinking_enabled: false,
                responses_stateful_enabled: false,
                temperature: Some(0.2),
                max_tokens: Some(128),
                context_window_tokens: Some(8_192),
                top_p: Some(1.0),
                max_retries: Some(0),
            },
            variables: Vec::<EffectiveVariable>::new(),
            skills: Vec::<EffectiveSkill>::new(),
            tools,
            tool_exposure: AgentToolExposureState::default(),
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![AgentMessage::user("hello")],
            observations: Vec::new(),
            short_term_memory: None,
            started_at: "2026-06-17T00:00:00Z".into(),
        }
    }

    fn tool(name: &str) -> EffectiveTool {
        EffectiveTool {
            tool_name: name.into(),
            domain_code: None,
            description: name.into(),
            input_schema: json!({"type": "object"}),
            skill_name: None,
            required_variable_keys: Vec::new(),
        }
    }
}
