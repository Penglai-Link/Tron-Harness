use crate::{
    application::{
        agent::{AgentInvocationError, RuntimeAdapterSelection},
        short_term_memory::ShortTermMemoryService,
    },
    domain::agent::{
        AgentMessage, AgentPromptPrefixSource, AgentRunContext, AgentShortTermMemoryRunContext,
    },
};

pub(super) fn activate_short_term_memory(
    short_term_memory: Option<&ShortTermMemoryService>,
    context: &mut AgentRunContext,
    runtime_selection: &RuntimeAdapterSelection,
    requested_conversation_code: Option<&str>,
    agent_code: &str,
    prompt: &str,
    prompt_prefix_messages: Vec<AgentMessage>,
    prompt_prefix_sources: Vec<Option<AgentPromptPrefixSource>>,
    mut user_prompt_message: Option<AgentMessage>,
    input_kind: Option<&str>,
    input_metadata: Option<serde_json::Value>,
) -> Result<(), AgentInvocationError> {
    // Carry structured answer identity in the user input for both persistent
    // and ephemeral runs. It is not promoted into trusted instructions.
    if let Some(metadata) = input_metadata.as_ref().filter(|metadata| {
        metadata.get("resume").is_some() || metadata.get("a2uiClientCapabilities").is_some()
    }) {
        let part = crate::domain::agent::AgentMessagePart::Context {
            kind: "tron_interaction_response".into(),
            text: String::new(),
            // Only interaction transport belongs to this record. Copying the
            // whole invocation metadata would persist current event payloads in
            // history, bypassing the configured dynamic context contributor.
            metadata: Some(serde_json::Value::Object(
                ["resume", "a2uiClientCapabilities"]
                    .into_iter()
                    .filter_map(|key| metadata.get(key).map(|value| (key.into(), value.clone())))
                    .collect(),
            )),
        };
        if let Some(message) = user_prompt_message.as_mut() {
            message.parts.push(part.clone());
        }
        if let Some(message) = std::sync::Arc::make_mut(&mut context.messages)
            .iter_mut()
            .rev()
            .find(|message| message.role == "user")
        {
            message.parts.push(part);
        }
    }
    if !matches!(runtime_selection, RuntimeAdapterSelection::React)
        || requested_conversation_code.is_none()
    {
        return Ok(());
    }
    let Some(_) = short_term_memory else {
        return Ok(());
    };

    let input_kind = input_kind.unwrap_or("USER_PROMPT");
    let input_code = context.run_code.clone();
    context.messages = prompt_prefix_messages.into();
    let current_input_message =
        runtime_input_context_message(input_kind, prompt, &input_metadata).or(user_prompt_message);
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: requested_conversation_code
            .expect("conversation code should be present")
            .to_string(),
        input_code,
        input_kind: input_kind.to_string(),
        input_metadata,
        prompt_prefix: context.messages.as_ref().clone(),
        prompt_prefix_sources,
        current_input_message,
        agent_code: Some(agent_code.to_string()),
        todo_enabled: resolve_todo_enabled(context),
    });
    Ok(())
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
    context
        .tools
        .iter()
        .any(|tool| tool.tool_name.eq_ignore_ascii_case("todo_update"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveSkill, EffectiveTool, EffectiveVariable,
    };

    use super::resolve_todo_enabled;

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

    #[test]
    fn todo_enabled_uses_authorized_tool_catalog() {
        let context = context_with_tools(vec![tool("todo_update")]);
        assert!(resolve_todo_enabled(&context));
    }

    #[test]
    fn todo_disabled_when_catalog_omits_todo_update() {
        let context = context_with_tools(vec![tool("bash_exec")]);
        assert!(!resolve_todo_enabled(&context));
    }

    fn context_with_tools(tools: Vec<EffectiveTool>) -> AgentRunContext {
        AgentRunContext {
            system_agent_fast_mode: false,
            collaboration_owner: None,
            agent_code: None,
            auto_model_route: None,
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
                supports_developer_role: false,
                reasoning_effort: None,
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
            effective_tools: Vec::new(),
            tool_assignment: None,
            execution_scope: AgentExecutionScope::default(),
            limits: AgentRunLimits::default(),
            messages: vec![AgentMessage::user("hello")].into(),
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
