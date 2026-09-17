use serde_json::{Value, json};

use crate::{
    application::mcp::RuntimeMcpWarning,
    domain::agent::{AgentKind, AgentModelConfig},
    shared::event_bus::{BusEventDraft, BusOutcome, BusRequestRef, EventBusHandle, PublishOutcome},
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

pub(super) fn publish_mcp_runtime_warnings(
    event_bus: &EventBusHandle,
    context: &McpWarningPublishContext<'_>,
    warnings: &[RuntimeMcpWarning],
) {
    for (index, warning) in warnings.iter().enumerate() {
        let tool_call_id = format!("{}-mcp-warning-{}", context.run_code, index + 1);
        let tool_name = "MCP warning";
        let tool_args = json!({
            "target": warning.server_code,
            "serverName": warning.server_name,
            "stage": "discovery"
        });
        publish_mcp_runtime_tool_event(
            event_bus,
            context,
            "agent.tool.selected",
            BusOutcome::Progress,
            Some("mcp_warning"),
            &tool_call_id,
            tool_name,
            Some(&tool_args),
            None,
        );
        publish_mcp_runtime_tool_event(
            event_bus,
            context,
            "agent.tool.started",
            BusOutcome::Started,
            Some("mcp_warning"),
            &tool_call_id,
            tool_name,
            Some(&tool_args),
            None,
        );
        publish_mcp_runtime_tool_event(
            event_bus,
            context,
            "agent.tool.failed",
            BusOutcome::Failure,
            Some("mcp_warning"),
            &tool_call_id,
            tool_name,
            None,
            Some(warning.message.as_str()),
        );
    }
}

fn publish_mcp_runtime_tool_event(
    event_bus: &EventBusHandle,
    context: &McpWarningPublishContext<'_>,
    event_name: &'static str,
    outcome: BusOutcome,
    summary: Option<&str>,
    tool_call_id: &str,
    tool_name: &str,
    tool_input: Option<&Value>,
    tool_output: Option<&str>,
) {
    let mut payload_fields = vec![
        (
            "tool_call_id".to_string(),
            compact_event_value(tool_call_id),
        ),
        ("tool_name".to_string(), compact_event_value(tool_name)),
    ];
    if let Some(tool_input) = tool_input {
        let value = serde_json::to_string(tool_input).unwrap_or_else(|_| "{}".into());
        payload_fields.push(("tool_args_json".to_string(), value));
    }
    if let Some(tool_output) = tool_output {
        payload_fields.push(("tool_result_json".to_string(), tool_output.to_string()));
    }

    let mut draft = BusEventDraft::agui(event_name, "agent_runtime")
        .outcome(outcome)
        .subject("agent_run", context.run_code.to_string())
        .attr("workspace_code", context.workspace_code.to_string())
        .attr("agent_kind", agent_kind_label(context.agent_kind))
        .attr("runtime_kind", "in_process")
        .attr("model_code", context.model.model_code.clone());
    if !context.model.model_name.trim().is_empty() {
        draft = draft.payload_field("model_name", context.model.model_name.clone());
    }
    if let Some(context_window_tokens) = context.model.context_window_tokens {
        draft = draft.payload_field("context_window_limit", context_window_tokens.to_string());
    }
    if let Some(summary) = summary {
        draft = draft.summary(summary);
    }
    if let Some(conversation_code) = context.conversation_code {
        draft = draft.attr("conversation_code", conversation_code.to_string());
    }
    if let Some(user_code) = context.user_code {
        draft = draft.attr("user_code", user_code.to_string());
    }
    if let Some(app_code) = context.app_code {
        draft = draft.attr("app_code", app_code.to_string());
    }
    if let Some(task_code) = context.task_code {
        draft = draft.attr("task_code", task_code.to_string());
    }
    let stream_key = context.conversation_code.unwrap_or(context.run_code);
    draft = draft.request(BusRequestRef::with_stream_key(stream_key.to_string()));
    for (key, value) in payload_fields {
        draft = draft.payload_field(key, compact_event_payload_value(&value));
    }

    match event_bus.publish(draft) {
        PublishOutcome::Disabled | PublishOutcome::Accepted { .. } => {}
        PublishOutcome::RejectedInvalid { .. }
        | PublishOutcome::RejectedOversize { .. }
        | PublishOutcome::DroppedIngressFull { .. }
        | PublishOutcome::DroppedIngressDisconnected { .. } => {}
    }
}

fn agent_kind_label(agent_kind: AgentKind) -> &'static str {
    match agent_kind {
        AgentKind::React => "react",
        AgentKind::SystemCli => "system_cli",
    }
}

fn compact_event_value(value: &str) -> String {
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

fn compact_event_payload_value(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::agent::AgentModelProtocol,
        shared::{
            event_bus::{
                BusRouteClass, EventBusConfig, EventBusRuntimeBuilder, RecordingEventSink,
            },
            id::FixedSnowflakeIdProvider,
        },
    };
    use std::{sync::Arc, time::Duration};

    fn warning_events(model_name: &str) -> Vec<crate::shared::event_bus::BusEvent> {
        let sink = Arc::new(RecordingEventSink::with_route_class(
            "mcp-warning-test",
            BusRouteClass::Agui,
        ));
        let event_bus = EventBusRuntimeBuilder::new(
            EventBusConfig::default(),
            Arc::new(FixedSnowflakeIdProvider::new(100)),
        )
        .register_sink(sink.clone())
        .build();
        let model = AgentModelConfig {
            model_code: "PERSON_AI:ROUTE:ChatAuto".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key: String::new(),
            model_name: model_name.into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: None,
            context_window_tokens: None,
            top_p: None,
            max_retries: None,
        };
        publish_mcp_runtime_warnings(
            &event_bus,
            &McpWarningPublishContext {
                run_code: "RUN-1",
                conversation_code: Some("CONV-1"),
                workspace_code: "TRON_CORE",
                user_code: Some("USR-1"),
                agent_kind: AgentKind::React,
                model: &model,
                app_code: None,
                task_code: None,
            },
            &[RuntimeMcpWarning {
                server_code: "MCP-1".into(),
                server_name: "Example MCP".into(),
                message: "discovery failed".into(),
            }],
        );

        assert!(sink.wait_for_len(3, Duration::from_secs(2)));
        sink.events()
    }

    #[test]
    fn mcp_warning_events_omit_empty_model_name() {
        let events = warning_events("");

        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|event| {
            event
                .payload_fields
                .iter()
                .all(|field| field.key != "model_name")
        }));
        assert!(events.iter().all(|event| {
            event
                .attrs
                .iter()
                .any(|attr| attr.key == "model_code" && attr.value == "PERSON_AI:ROUTE:ChatAuto")
        }));
    }

    #[test]
    fn mcp_warning_events_keep_non_empty_model_name() {
        let events = warning_events("gpt-test");

        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|event| {
            event
                .payload_fields
                .iter()
                .any(|field| field.key == "model_name" && field.value == "gpt-test")
        }));
    }
}
