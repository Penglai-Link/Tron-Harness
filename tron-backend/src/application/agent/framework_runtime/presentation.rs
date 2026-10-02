//! Consume the SDK protocol stream; the product owns metadata and durable replay only.
use crate::{
    application::agent::AgentAguiStreamService,
    domain::agent::{AgentAguiProtocolEvent, AgentRunContext},
};
use eliteforge_agent_official_plugin_agui::AgUiStream;
use serde_json::{Value, json};

// Bound a stalled SQLite acknowledgment independently of SDK execution budgets.
const REPLAY_WRITE_ACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Drain to the SDK stream boundary even after persistence failure. A failed presentation
/// never changes the result returned by the independently owned Agent::wait task.
pub(super) async fn consume_native_agui(
    stream: AgUiStream,
    context: &AgentRunContext,
    service: Option<AgentAguiStreamService>,
) {
    consume_native_agui_with_deadline(stream, context, service, REPLAY_WRITE_ACK_TIMEOUT).await;
}

async fn consume_native_agui_with_deadline(
    stream: AgUiStream,
    context: &AgentRunContext,
    service: Option<AgentAguiStreamService>,
    deadline: std::time::Duration,
) {
    let stream_key = context
        .stream_key
        .as_deref()
        .or(context.conversation_code.as_deref())
        .unwrap_or(&context.run_code);
    consume_native_projected(stream, stream_key, service, deadline, |data, ordinal| {
        project_native_event(data, context, ordinal)
    })
    .await;
}

pub(crate) async fn consume_native_agui_group(
    stream: AgUiStream,
    session: &str,
    parent_product_run: &str,
    service: Option<AgentAguiStreamService>,
) {
    consume_native_projected(
        stream,
        session,
        service,
        REPLAY_WRITE_ACK_TIMEOUT,
        |data, ordinal| project_native_group_event(data, session, parent_product_run, ordinal),
    )
    .await;
}

async fn consume_native_projected(
    mut stream: AgUiStream,
    stream_key: &str,
    service: Option<AgentAguiStreamService>,
    deadline: std::time::Duration,
    mut project: impl FnMut(Value, u64) -> Option<AgentAguiProtocolEvent>,
) {
    let mut persistence_stalled = false;
    let mut source = None;
    let mut ordinal = 0u64;
    while let Some(event) = stream.recv().await {
        let mut data = match serde_json::to_value(event) {
            Ok(data) => data,
            Err(error) => {
                if let Some(service) = &service {
                    service.mark_incomplete(stream_key, &error.to_string());
                }
                continue;
            }
        };
        crate::application::agent::agui_stream::project_product_visibility(&mut data);
        let next_source = data
            .pointer("/metadata/framework/sourceSequence")
            .and_then(Value::as_u64);
        if source == next_source {
            ordinal = ordinal.saturating_add(1);
        } else {
            source = next_source;
            ordinal = 0;
        }
        let Some(event) = project(data, ordinal) else {
            if let Some(service) = &service {
                service.mark_incomplete(stream_key, "native event metadata is unavailable");
            }
            continue;
        };
        if event.sse_event == "RUN_ERROR"
            && event
                .data
                .get("code")
                .and_then(Value::as_str)
                .is_some_and(|code| code.starts_with("AGUI_"))
        {
            if let Some(service) = &service {
                service.mark_incomplete(stream_key, "native AG-UI output is incomplete");
            }
        }
        let Some(service) = &service else {
            continue;
        };
        if persistence_stalled {
            continue;
        }
        let recorder = service.clone();
        let record_key = stream_key.to_owned();
        let write =
            tokio::task::spawn_blocking(move || recorder.record_protocol_event(&record_key, event));
        match tokio::time::timeout(deadline, write).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(error))) => service.mark_incomplete(stream_key, &format!("{error:?}")),
            Ok(Err(_)) => service.mark_incomplete(
                stream_key,
                "native presentation persistence task ended without acknowledgment",
            ),
            Err(_) => {
                // A blocking database operation cannot be forcibly cancelled. Leave at
                // most this single write in flight, and drop later presentation frames
                // while draining the SDK channel to completion.
                persistence_stalled = true;
                service.mark_incomplete(
                    stream_key,
                    "native presentation persistence acknowledgment timed out",
                );
            }
        }
    }
}

fn project_native_event(
    mut data: Value,
    context: &AgentRunContext,
    ordinal: u64,
) -> Option<AgentAguiProtocolEvent> {
    let sse_event = data.get("type")?.as_str()?.to_owned();
    let execution = data
        .pointer("/metadata/framework/executionId")?
        .as_str()?
        .to_owned();
    let sequence = data
        .pointer("/metadata/framework/sourceSequence")?
        .as_u64()?;
    let native_run = data.get("runId").cloned();
    let object = data.as_object_mut()?;
    // Product API control uses its stable runCode. Native logical run/execution IDs
    // remain unmodified in SDK metadata, and native top-level identity is retained too.
    object.insert("runId".into(), json!(context.run_code));
    object.insert(
        "threadId".into(),
        json!(
            context
                .conversation_code
                .as_deref()
                .unwrap_or(&context.run_code)
        ),
    );
    let metadata = object
        .entry("metadata")
        .or_insert_with(|| json!({}))
        .as_object_mut()?;
    metadata.insert("tron".into(), json!({
        "eventId": format!("native:{execution}:{sequence}:{ordinal}"), "sourceOrdinal": ordinal,
        "runCode": context.run_code, "nativeRunId": native_run,
        "workspaceCode": context.workspace_code, "userCode": context.user_code,
        "appCode": context.execution_scope.app_code, "taskCode": context.execution_scope.task_code,
    }));
    Some(AgentAguiProtocolEvent { sse_event, data })
}

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;

fn project_native_group_event(
    mut data: Value,
    session: &str,
    parent_product_run: &str,
    ordinal: u64,
) -> Option<AgentAguiProtocolEvent> {
    let sse_event = data.get("type")?.as_str()?.to_owned();
    let native_run = data
        .pointer("/metadata/framework/runId")?
        .as_str()?
        .to_owned();
    let execution = data
        .pointer("/metadata/framework/executionId")?
        .as_str()?
        .to_owned();
    let sequence = data
        .pointer("/metadata/framework/sourceSequence")?
        .as_u64()?;
    let run_code = format!("goal:{native_run}");
    let object = data.as_object_mut()?;
    object.insert("runId".into(), json!(run_code));
    object.insert("threadId".into(), json!(session));
    let metadata = object
        .entry("metadata")
        .or_insert_with(|| json!({}))
        .as_object_mut()?;
    metadata.insert("tron".into(),json!({"eventId":format!("native:{execution}:{sequence}:{ordinal}"),"sourceOrdinal":ordinal,"runCode":run_code,"nativeRunId":native_run,"parentProductRunCode":parent_product_run}));
    Some(AgentAguiProtocolEvent { sse_event, data })
}
