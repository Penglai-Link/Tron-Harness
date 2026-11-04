use serde_json::json;

use crate::{
    application::{mcp::RuntimeMcpWarning, short_term_memory::redact_sensitive_text},
    domain::agent::{AgentKind, AgentModelConfig},
    shared::event_bus::{BusEventDraft, BusOutcome, BusRequestRef, EventBusHandle},
};

pub(super) struct McpWarningPublishContext<'a> {
    pub run_code: &'a str,
    pub conversation_code: Option<&'a str>,
    pub workspace_code: &'a str,
    pub user_code: Option<&'a str>,
    pub agent_kind: AgentKind,
    pub model: &'a AgentModelConfig,
    pub app_code: Option<&'a str>,
    pub task_code: Option<&'a str>,
}

// Discovery warnings are product diagnostics, never a synthetic native tool invocation.
pub(super) fn publish_mcp_runtime_warnings(
    event_bus: &EventBusHandle,
    context: &McpWarningPublishContext<'_>,
    warnings: &[RuntimeMcpWarning],
) {
    for warning in warnings {
        let safe = |value: &str| {
            redact_sensitive_text(value)
                .chars()
                .take(1024)
                .collect::<String>()
        };
        let value = json!({"serverCode":safe(&warning.server_code),"serverName":safe(&warning.server_name),"stage":"discovery","message":safe(&warning.message)});
        let mut draft = BusEventDraft::agui("agent.mcp.warning", "agent_runtime")
            .outcome(BusOutcome::Rejected)
            .subject("agent_run", context.run_code.to_owned())
            .summary("MCP discovery unavailable")
            .attr("workspace_code", context.workspace_code.to_owned())
            .attr("agent_kind", agent_kind_label(context.agent_kind))
            .attr("runtime_kind", "in_process")
            .attr("model_code", context.model.model_code.clone())
            .payload_field("context_event_json", value.to_string())
            .request(BusRequestRef::with_stream_key(
                context.conversation_code.unwrap_or(context.run_code),
            ));
        if !context.model.model_name.trim().is_empty() {
            draft = draft.payload_field("model_name", context.model.model_name.clone());
        }
        for (key, value) in [
            ("conversation_code", context.conversation_code),
            ("user_code", context.user_code),
            ("app_code", context.app_code),
            ("task_code", context.task_code),
        ] {
            if let Some(value) = value {
                draft = draft.attr(key, value);
            }
        }
        // Rejection/drop feedback belongs to the existing replay sink contract.
        let _ = event_bus.publish(draft);
    }
}
fn agent_kind_label(agent_kind: AgentKind) -> &'static str {
    match agent_kind {
        AgentKind::React => "react",
        AgentKind::SystemCli => "system_cli",
    }
}
