use crate::application::short_term_memory::ShortTermMemoryService;
use crate::domain::{
    agent::{AgentError, AgentKind, AgentRunContext},
    short_term_memory::ShortTermMemoryError,
};

pub(super) fn is_context_overflow_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("context_length_exceeded")
        || normalized.contains("context length")
        || normalized.contains("context window")
        || normalized.contains("maximum context")
        || normalized.contains("max context")
        || normalized.contains("too many tokens")
        || normalized.contains("token limit")
        || normalized.contains("input too large")
}

pub(super) fn short_term_memory_error(error: ShortTermMemoryError) -> AgentError {
    AgentError::InvalidConfiguration(error.to_string())
}

pub(super) fn record_failure_on_short_term_memory_error<T>(
    short_term_memory: &ShortTermMemoryService,
    context: &AgentRunContext,
    reason: &str,
    result: Result<T, ShortTermMemoryError>,
) -> Result<T, AgentError> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            let _ = short_term_memory.record_run_failed(context, reason);
            Err(short_term_memory_error(error))
        }
    }
}

pub(super) fn agent_kind_label(agent_kind: AgentKind) -> &'static str {
    match agent_kind {
        AgentKind::React => "react",
        AgentKind::SystemCli => "system_cli",
    }
}

pub(super) fn compact_event_value(value: &str) -> String {
    const LIMIT: usize = 96;
    if value.len() <= LIMIT {
        return value.to_string();
    }

    let mut end = LIMIT.saturating_sub(3);
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    format!("{}...", &value[..end])
}

pub(super) fn compact_event_payload_value(value: &str) -> String {
    const LIMIT: usize = 4096;
    if value.len() <= LIMIT {
        return value.to_string();
    }

    let marker = format!("\n...[truncated: original_bytes={}]", value.len());
    let mut end = LIMIT.saturating_sub(marker.len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    format!("{}{}", &value[..end], marker)
}

pub(super) fn validate_context(context: &AgentRunContext) -> Result<(), AgentError> {
    if context.run_code.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "runCode is required".into(),
        ));
    }
    if context.workspace_code.trim().is_empty() {
        return Err(AgentError::InvalidConfiguration(
            "workspaceCode is required".into(),
        ));
    }
    // Person-AI's ChatAuto route intentionally has no physical model name;
    // the gateway selects the model from the authenticated provider session.
    let person_ai_chat_auto_route = context.model.model_code == "PERSON_AI:ROUTE:ChatAuto";
    if context.model.model_name.trim().is_empty() && !person_ai_chat_auto_route {
        return Err(AgentError::InvalidConfiguration(
            "modelName is required".into(),
        ));
    }
    if context.limits.max_steps == 0 {
        return Err(AgentError::InvalidConfiguration(
            "maxSteps must be greater than zero".into(),
        ));
    }
    if context.limits.max_tool_calls == 0 {
        return Err(AgentError::InvalidConfiguration(
            "maxToolCalls must be greater than zero".into(),
        ));
    }
    if context.limits.max_parallel_tool_calls == 0 {
        return Err(AgentError::InvalidConfiguration(
            "maxParallelToolCalls must be greater than zero".into(),
        ));
    }
    Ok(())
}
