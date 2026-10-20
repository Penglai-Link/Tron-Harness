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
        warning_events_with_details(model_name, "Example MCP", "discovery failed")
    }
    fn warning_events_with_details(
        model_name: &str,
        server_name: &str,
        message: &str,
    ) -> Vec<crate::shared::event_bus::BusEvent> {
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
            supports_developer_role: false,
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
                server_name: server_name.into(),
                message: message.into(),
            }],
        );

        assert!(sink.wait_for_len(1, Duration::from_secs(2)));
        sink.events()
    }

    #[test]
    fn mcp_warning_events_omit_empty_model_name() {
        let events = warning_events("");

        assert_eq!(events.len(), 1);
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

        assert_eq!(events.len(), 1);
        assert!(events.iter().all(|event| {
            event
                .payload_fields
                .iter()
                .any(|field| field.key == "model_name" && field.value == "gpt-test")
        }));
    }
    #[test]
    fn mcp_warning_is_a_redacted_durable_custom_diagnostic_without_fake_tool_calls() {
        use crate::{
            adapters::outbound::persistence::SqliteAgentEventReplayStore,
            application::agent::{AgentAguiReplayStore, agui_translate::translate_protocol_events},
            shared::event_bus::EventSink,
        };
        let events = warning_events_with_details(
            "gpt-test",
            "https://user:NAME_SENTINEL@example.test",
            "basic https://user:PASSWORD_SENTINEL@example.test/connect
query https://example.test?access_token=TOKEN_SENTINEL
Authorization Bearer AUTH_SENTINEL
use sk-API_SENTINEL now",
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "agent.mcp.warning");
        let protocol = translate_protocol_events("CONV-1", &events[0]);
        assert_eq!(protocol.len(), 1);
        assert_eq!(protocol[0].sse_event, "CUSTOM");
        assert_eq!(protocol[0].data["name"], "tron.mcp.runtime.warning");
        assert_eq!(protocol[0].data["value"]["serverCode"], "MCP-1");
        assert_eq!(protocol[0].data["value"]["stage"], "discovery");
        let serialized = serde_json::to_string(&protocol).unwrap();
        for secret in [
            "NAME_SENTINEL",
            "PASSWORD_SENTINEL",
            "TOKEN_SENTINEL",
            "AUTH_SENTINEL",
            "API_SENTINEL",
        ] {
            assert!(!serialized.contains(secret));
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("warning.sqlite3");
        let host = crate::application::agent::FrameworkHost::persistent(&path, None);
        crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(async {
            let framework = host.storage_framework().await.unwrap();
            let mut session = eliteforge_agent_core::SessionRecord::new("warning");
            session.id = "CONV-1".into();
            framework.sessions().unwrap().create(session).await.unwrap();
        });
        {
            let replay = SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
            replay.handle(Arc::new(events[0].clone()));
            replay.handle(Arc::new(events[0].clone()));
            assert_eq!(replay.replay_protocol_events("CONV-1").unwrap(), protocol);
            let audit = replay.replay_agui_events("CONV-1").unwrap();
            assert_eq!(audit.len(), 1);
            let serialized = serde_json::to_string(&audit).unwrap();
            for secret in [
                "NAME_SENTINEL",
                "PASSWORD_SENTINEL",
                "TOKEN_SENTINEL",
                "AUTH_SENTINEL",
                "API_SENTINEL",
            ] {
                assert!(!serialized.contains(secret));
            }
        }
        let reopened = SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
        assert_eq!(reopened.replay_protocol_events("CONV-1").unwrap(), protocol);
    }
}
