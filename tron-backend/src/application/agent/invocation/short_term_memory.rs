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
        .any(|tool| tool.tool_name.eq_ignore_ascii_case("todo_update") || tool.tool_name.eq_ignore_ascii_case("dude_todo_update"))
}
