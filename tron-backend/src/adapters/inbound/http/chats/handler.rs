use std::{collections::BTreeSet, convert::Infallible, sync::Arc, time::Duration};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::{
        agent::{AgentAguiStreamService, AgentInvocationService, generate_agent_run_code},
        chat::{
            ChatSessionQuery, CreateChatSessionCommand, SendChatMessageCommand,
            UpdateChatSessionCommand,
        },
    },
    domain::{agent::ModelStreamEvent, chat::ChatError},
    response::ApiResponse,
    shared::auth_holder::AuthHolder,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};

use super::error_mapper::{chat_error_response, chat_send_error_response};
use super::protocol_replay::record_protocol_event;
use super::stream_live::{SystemAgentLiveEventBuilder, SystemAgentLiveReplayRecorder};
use super::stream_task::{ChatStreamForwardingGate, ChatStreamLifecycleGuard};
mod shared;
use shared::{ChatSsePayload, ChatStreamCompletion};
pub(crate) use shared::{
    chat_send_error_code, chat_send_error_message, normalized_required, prepare_send_model,
    protocol_event_belongs_to_run, resolve_chat_agent_runtime, uses_system_live_stream,
};

const CHAT_STREAM_POLL_INTERVAL_MS: u64 = 25;
const CHAT_STREAM_DRAIN_AFTER_COMPLETION_TICKS: u16 = 80;

pub async fn list_session_groups(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Query(query): Query<ChatSessionQuery>,
) -> Response {
    let Some(workspace_code) = normalized_required(query.workspace_code.clone()) else {
        return chat_error_response(ChatError::WorkspaceRequired);
    };

    match state.chats.list_session_groups(ChatSessionQuery {
        workspace_code: Some(workspace_code),
    }) {
        Ok(groups) => Json(ApiResponse::success(groups)).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn create_session(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<CreateChatSessionCommand>,
) -> Response {
    if normalized_required(request.workspace_code.clone()).is_none() {
        return chat_error_response(ChatError::WorkspaceRequired);
    };

    match state
        .chats
        .create_session(request, Some(auth.user().user_code.clone()))
    {
        Ok(session) => (StatusCode::CREATED, Json(ApiResponse::success(session))).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn archive_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    match state.chats.archive_session(&session_code) {
        Ok(session) => Json(ApiResponse::success(session)).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn delete_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    match state.chats.delete_session(&session_code) {
        Ok(session) => Json(ApiResponse::success(session)).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn list_messages(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    if state.chats.get_session(&session_code).is_err() {
        return chat_error_response(ChatError::SessionNotFound);
    }
    match state.chats.list_messages(&session_code) {
        Ok(messages) => Json(ApiResponse::success(messages)).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn patch_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
    Json(request): Json<UpdateChatSessionCommand>,
) -> Response {
    match state.chats.update_session(&session_code, request) {
        Ok(session) => Json(ApiResponse::success(session)).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn send_message(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(session_code): Path<String>,
    Json(request): Json<SendChatMessageCommand>,
) -> Response {
    let Ok(session) = state.chats.get_session(&session_code) else {
        return chat_error_response(ChatError::SessionNotFound);
    };
    let resolved_runtime = match resolve_chat_agent_runtime(&state, &session, &request) {
        Ok(runtime) => runtime,
        Err(error) => return chat_send_error_response(error),
    };
    let resolved_model = match prepare_send_model(&state, &session, &request, &resolved_runtime) {
        Ok(model) => model,
        Err(error) => return chat_send_error_response(error),
    };

    let agent_service = AgentInvocationService::new(
        state.workspaces.clone(),
        state.apps.clone(),
        state.variables.clone(),
        state.skills.clone(),
        state.agent_management.clone(),
        state.agent_runs.clone(),
        state.mcp.clone(),
        state.event_bus.clone(),
        state.agent_model_client.clone(),
        state.agent_tool_registry.clone(),
        state.runtime_http_client.clone(),
    );
    match state
        .chats
        .send_message(
            &session_code,
            request,
            resolved_model,
            Some(auth.user().user_code.clone()),
            Some(auth.locale.clone()),
            Some(state.llm_runtime.clone()),
            agent_service,
        )
        .await
    {
        Ok(result) => Json(ApiResponse::success(result)).into_response(),
        Err(error) => chat_send_error_response(error),
    }
}

pub async fn stream_message(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path(session_code): Path<String>,
    Json(mut request): Json<SendChatMessageCommand>,
) -> Response {
    let Ok(session) = state.chats.get_session(&session_code) else {
        return chat_error_response(ChatError::SessionNotFound);
    };
    let resolved_runtime = match resolve_chat_agent_runtime(&state, &session, &request) {
        Ok(runtime) => runtime,
        Err(error) => return chat_send_error_response(error),
    };
    let system_agent_live_stream = uses_system_live_stream(&resolved_runtime);
    let resolved_model = match prepare_send_model(&state, &session, &request, &resolved_runtime) {
        Ok(model) => model,
        Err(error) => return chat_send_error_response(error),
    };

    let run_code = generate_agent_run_code();
    request.run_code = Some(run_code.clone());

    let agui_stream = AgentAguiStreamService::with_replay_store(
        state.agui_events.clone(),
        state.agui_replay_store.clone(),
    );
    let _ = agui_stream.activate(&session_code);

    let (completion_sender, mut completion_receiver) =
        tokio::sync::mpsc::channel::<ChatSsePayload>(1);
    let (model_event_sender, mut model_event_receiver) =
        tokio::sync::mpsc::unbounded_channel::<ModelStreamEvent>();
    let live_replay_recorder = SystemAgentLiveReplayRecorder::new(
        agui_stream.clone(),
        &session_code,
        &run_code,
        system_agent_live_stream,
    );
    let chats = state.chats.clone();
    let session_code_for_task = session_code.clone();
    let run_code_for_task = run_code.clone();
    let user_code = Some(auth.user().user_code.clone());
    let agent_service = AgentInvocationService::new(
        state.workspaces.clone(),
        state.apps.clone(),
        state.variables.clone(),
        state.skills.clone(),
        state.agent_management.clone(),
        state.agent_runs.clone(),
        state.mcp.clone(),
        state.event_bus.clone(),
        state.agent_model_client.clone(),
        state.agent_tool_registry.clone(),
        state.runtime_http_client.clone(),
    );
    let live_replay_recorder_for_sink = live_replay_recorder.clone();
    let forwarding_gate = ChatStreamForwardingGate::new();
    let forwarding_gate_for_sink = forwarding_gate.clone();
    let sink = Arc::new(move |event: ModelStreamEvent| {
        if !forwarding_gate_for_sink.is_open() {
            return;
        }
        live_replay_recorder_for_sink.ingest(event.clone());
        let _ = model_event_sender.send(event);
    });

    let replay_service_for_task = agui_stream.clone();
    let live_replay_recorder_for_task = live_replay_recorder.clone();
    let llm_runtime = state.llm_runtime.clone();
    let run_task = AuthHolder::spawn(async move {
        let result = chats
            .send_message_stream(
                &session_code_for_task,
                request,
                resolved_model,
                user_code,
                Some(auth.locale.clone()),
                Some(llm_runtime),
                agent_service,
                sink,
            )
            .await;
        let completion = match result {
            Ok(result) => ChatStreamCompletion::Success(result),
            Err(error) => {
                let code = chat_send_error_code(&error).to_string();
                let message = chat_send_error_message(error);
                ChatStreamCompletion::Error { code, message }
            }
        };
        live_replay_recorder_for_task.finalize();
        let done_payload = completion.payload(&session_code_for_task, &run_code_for_task);
        record_protocol_event(
            &replay_service_for_task,
            &session_code_for_task,
            &done_payload.event_name,
            &done_payload.data,
        );
        let _ = completion_sender.send(done_payload).await;
    });

    let stream_key = session_code.clone();
    let stream_run_code = run_code.clone();
    let stream_service = agui_stream.clone();
    let stream = async_stream::stream! {
        let mut lifecycle_guard = ChatStreamLifecycleGuard::new(
            run_task,
            forwarding_gate,
            stream_service.clone(),
            stream_key.clone(),
            stream_run_code.clone(),
        );
        let mut seen = BTreeSet::<String>::new();
        let mut completion: Option<ChatSsePayload> = None;
        let mut drain_ticks = 0u16;
        let mut interval = tokio::time::interval(Duration::from_millis(CHAT_STREAM_POLL_INTERVAL_MS));
        let mut live_event_builder = SystemAgentLiveEventBuilder::new(
            &stream_key,
            &stream_run_code,
            system_agent_live_stream,
        );

        loop {
            interval.tick().await;

            if completion.is_none() {
                if let Ok(received) = completion_receiver.try_recv() {
                    completion = Some(received);
                }
            }

            let mut emitted = false;
            while let Ok(event) = model_event_receiver.try_recv() {
                for payload in live_event_builder.ingest(event) {
                    let key = format!("{}:{}", payload.sse_event, payload.data);
                    if seen.insert(key) {
                        emitted = true;
                        yield Ok::<Event, Infallible>(
                            ChatSsePayload::agui(payload.sse_event, payload.data).event()
                        );
                    }
                }
            }
            if let Ok(events) = stream_service.protocol_events(&stream_key) {
                for event in events {
                    if !protocol_event_belongs_to_run(&event.data, &stream_run_code) {
                        continue;
                    }
                    if matches!(event.sse_event.as_str(), "RUN_FINISHED" | "RUN_ERROR") {
                        continue;
                    }
                    if live_event_builder.enabled()
                        && matches!(
                            event.sse_event.as_str(),
                            "RUN_STARTED"
                                | "REASONING_START"
                                | "REASONING_MESSAGE_START"
                                | "REASONING_MESSAGE_CONTENT"
                                | "REASONING_TEXT_CONTENT"
                                | "REASONING_MESSAGE_END"
                                | "REASONING_END"
                                | "TEXT_MESSAGE_START"
                                | "TEXT_MESSAGE_CONTENT"
                                | "TEXT_MESSAGE_END"
                        )
                    {
                        continue;
                    }
                    let key = format!("{}:{}", event.sse_event, event.data);
                    if seen.insert(key) {
                        emitted = true;
                        yield Ok::<Event, Infallible>(ChatSsePayload::agui(event.sse_event, event.data).event());
                    }
                }
            }

            if let Some(done) = completion.as_ref() {
                for payload in live_event_builder.finalize() {
                    let key = format!("{}:{}", payload.sse_event, payload.data);
                    if seen.insert(key) {
                        emitted = true;
                        yield Ok::<Event, Infallible>(
                            ChatSsePayload::agui(payload.sse_event, payload.data).event()
                        );
                    }
                }
                if emitted {
                    drain_ticks = 0;
                    continue;
                }
                drain_ticks = drain_ticks.saturating_add(1);
                if drain_ticks < CHAT_STREAM_DRAIN_AFTER_COMPLETION_TICKS {
                    continue;
                }
                let done_event = done.event();
                lifecycle_guard.complete();
                yield Ok::<Event, Infallible>(done_event);
                break;
            }
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}
