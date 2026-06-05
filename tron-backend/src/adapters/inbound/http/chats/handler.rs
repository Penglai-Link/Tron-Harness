use std::{collections::BTreeSet, convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde_json::{Value, json};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::{
        agent::{
            AgentAguiStreamService, AgentInvocationError, AgentInvocationService,
            ResolvedAgentRuntime, build_system_runtime_model_config, generate_agent_run_code,
        },
        chat::{
            ChatSendError, ChatSendResult, ChatSessionQuery, CreateChatSessionCommand,
            SendChatMessageCommand, UpdateChatSessionCommand,
        },
    },
    domain::agent::{AgentModelConfig, AgentRuntimeType, ModelStreamEvent},
    domain::{
        agent::ManagedSystemAgentRecord,
        chat::{ChatError, ChatSession},
    },
    response::ApiResponse,
};

use super::error_mapper::{chat_error_response, chat_send_error_response};
use super::protocol_replay::record_protocol_event;
use super::stream_live::SystemAgentLiveEventBuilder;

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
        tokio::sync::mpsc::channel::<ChatStreamCompletion>(1);
    let (model_event_sender, mut model_event_receiver) =
        tokio::sync::mpsc::unbounded_channel::<ModelStreamEvent>();
    let chats = state.chats.clone();
    let session_code_for_task = session_code.clone();
    let user_code = Some(auth.user().user_code.clone());
    let agent_service = AgentInvocationService::new(
        state.workspaces.clone(),
        state.apps.clone(),
        state.variables.clone(),
        state.skills.clone(),
        state.agent_management.clone(),
        state.agent_runs.clone(),
        state.event_bus.clone(),
        state.agent_model_client.clone(),
        state.agent_tool_registry.clone(),
        state.runtime_http_client.clone(),
    );
    let sink = Arc::new(move |event: ModelStreamEvent| {
        let _ = model_event_sender.send(event);
    });

    tokio::spawn(async move {
        let result = chats
            .send_message_stream(
                &session_code_for_task,
                request,
                resolved_model,
                user_code,
                Some(auth.locale.clone()),
                agent_service,
                sink,
            )
            .await;
        match result {
            Ok(result) => {
                let _ = completion_sender
                    .send(ChatStreamCompletion::Success(result))
                    .await;
            }
            Err(error) => {
                let code = chat_send_error_code(&error).to_string();
                let message = chat_send_error_message(error);
                let _ = completion_sender
                    .send(ChatStreamCompletion::Error { code, message })
                    .await;
            }
        }
    });

    let stream_key = session_code.clone();
    let stream_run_code = run_code.clone();
    let stream_service = agui_stream.clone();
    let stream = async_stream::stream! {
        let mut seen = BTreeSet::<String>::new();
        let mut completion: Option<ChatStreamCompletion> = None;
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
                        record_protocol_event(&stream_service, &stream_key, &payload.sse_event, &payload.data);
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
                        record_protocol_event(&stream_service, &stream_key, &payload.sse_event, &payload.data);
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
                let done_payload = done.payload(&stream_key, &stream_run_code);
                record_protocol_event(
                    &stream_service,
                    &stream_key,
                    &done_payload.event_name,
                    &done_payload.data,
                );
                yield Ok::<Event, Infallible>(done_payload.event());
                break;
            }
        }

        let _ = stream_service.deactivate(&stream_key);
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

fn protocol_event_belongs_to_run(data: &Value, run_code: &str) -> bool {
    data.get("runId").and_then(Value::as_str) == Some(run_code)
}

#[derive(Debug)]
struct ChatSsePayload {
    event_name: String,
    data: Value,
}

impl ChatSsePayload {
    fn agui(event_name: impl Into<String>, data: Value) -> Self {
        Self {
            event_name: event_name.into(),
            data,
        }
    }

    fn event(&self) -> Event {
        Event::default()
            .event(self.event_name.clone())
            .json_data(&self.data)
            .unwrap_or_else(|_| {
                Event::default()
                    .event("RUN_ERROR")
                    .data(json!({"type":"RUN_ERROR","code":"CHAT_STREAM_SERIALIZATION_FAILED","message":"chat stream serialization failed"}).to_string())
            })
    }
}

enum ChatStreamCompletion {
    Success(ChatSendResult),
    Error { code: String, message: String },
}

impl ChatStreamCompletion {
    fn payload(&self, session_code: &str, run_code: &str) -> ChatSsePayload {
        match self {
            Self::Success(result) => ChatSsePayload::agui(
                "RUN_FINISHED",
                json!({
                    "type": "RUN_FINISHED",
                    "threadId": session_code,
                    "runId": result.run.run_code,
                    "result": result,
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                }),
            ),
            Self::Error { code, message } => ChatSsePayload::agui(
                "RUN_ERROR",
                json!({
                    "type": "RUN_ERROR",
                    "threadId": session_code,
                    "runId": run_code,
                    "code": code,
                    "message": message,
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                }),
            ),
        }
    }
}

fn chat_send_error_code(error: &ChatSendError) -> &'static str {
    match error {
        ChatSendError::Chat(ChatError::MessageRequired) => "CHAT_MESSAGE_REQUIRED",
        ChatSendError::Chat(ChatError::SessionNotFound) => "CHAT_SESSION_NOT_FOUND",
        ChatSendError::Chat(ChatError::SessionTitleRequired) => "CHAT_SESSION_TITLE_REQUIRED",
        ChatSendError::Chat(ChatError::WorkspaceRequired) => "CHAT_WORKSPACE_REQUIRED",
        ChatSendError::Chat(ChatError::WorkspaceForbidden) => "CHAT_WORKSPACE_FORBIDDEN",
        ChatSendError::Chat(ChatError::StorageFailure) => "CHAT_STORAGE_FAILED",
        ChatSendError::Agent(error) => error.code(),
    }
}

fn chat_send_error_message(error: ChatSendError) -> String {
    match error {
        ChatSendError::Chat(ChatError::MessageRequired) => "message content is required".into(),
        ChatSendError::Chat(ChatError::SessionNotFound) => "chat session was not found".into(),
        ChatSendError::Chat(ChatError::SessionTitleRequired) => "session title is required".into(),
        ChatSendError::Chat(ChatError::WorkspaceRequired) => "workspaceCode is required".into(),
        ChatSendError::Chat(ChatError::WorkspaceForbidden) => {
            "workspace access is forbidden".into()
        }
        ChatSendError::Chat(ChatError::StorageFailure) => "chat storage failed".into(),
        ChatSendError::Agent(error) => error.message(),
    }
}

fn normalized_required(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalized_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn effective_agent_code(session: &ChatSession, request: &SendChatMessageCommand) -> String {
    let selected = if session.last_run_code.is_some() {
        session.agent_code.clone()
    } else {
        normalized_optional(request.agent_code.clone()).or(session.agent_code.clone())
    };
    selected.unwrap_or_else(|| "main-agent".to_string())
}

fn resolve_chat_agent_runtime(
    state: &AppState,
    session: &ChatSession,
    request: &SendChatMessageCommand,
) -> Result<ResolvedAgentRuntime, ChatSendError> {
    let effective_agent_code = effective_agent_code(session, request);
    state
        .agent_management
        .resolve_workspace_agent_runtime(
            &session.workspace_code,
            Some(effective_agent_code.as_str()),
        )
        .map_err(|error| match error {
            crate::domain::agent::AgentManagementError::InvalidWorkspace => {
                ChatSendError::Chat(ChatError::WorkspaceForbidden)
            }
            crate::domain::agent::AgentManagementError::SystemAgentNotFound => {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                    "agent `{effective_agent_code}` was not found"
                )))
            }
            crate::domain::agent::AgentManagementError::SystemAgentUnavailable => {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                    "agent `{effective_agent_code}` is unavailable or disabled for this workspace"
                )))
            }
            crate::domain::agent::AgentManagementError::InternalAgentNotFound
            | crate::domain::agent::AgentManagementError::InternalToolAssignmentStale(_)
            | crate::domain::agent::AgentManagementError::InternalToolUnknown(_) => {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                    "agent `{effective_agent_code}` is not available for this workspace"
                )))
            }
            crate::domain::agent::AgentManagementError::StorageFailure => ChatSendError::Agent(
                AgentInvocationError::RuntimeFailed("agent runtime resolution failed".into()),
            ),
        })
}

fn uses_system_live_stream(runtime: &ResolvedAgentRuntime) -> bool {
    !matches!(runtime.runtime_type, AgentRuntimeType::React)
}

fn prepare_send_model(
    state: &AppState,
    session: &ChatSession,
    request: &SendChatMessageCommand,
    runtime: &ResolvedAgentRuntime,
) -> Result<AgentModelConfig, ChatSendError> {
    if matches!(runtime.runtime_type, AgentRuntimeType::React) {
        return state
            .models
            .text_agent_model_config(&session.workspace_code, request.model_code.as_deref())
            .map_err(|_| ChatSendError::Chat(ChatError::StorageFailure))?
            .ok_or(ChatSendError::Agent(AgentInvocationError::ModelRequired));
    }

    let model_name = request
        .system_model_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(ChatSendError::Agent(AgentInvocationError::ModelRequired))?;

    let system_agent = state
        .agent_management
        .list_workspace_system_agents(&session.workspace_code)
        .map_err(|_| {
            ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                "workspace system agent policy load failed".into(),
            ))
        })?
        .into_iter()
        .find(|record| record.agent_code.eq_ignore_ascii_case(&runtime.agent_code))
        .ok_or_else(|| {
            ChatSendError::Agent(AgentInvocationError::RuntimeFailed(format!(
                "agent `{}` was not found",
                runtime.agent_code
            )))
        })?;

    let matched_model = validate_system_agent_model(&system_agent, &model_name)?;
    Ok(build_system_runtime_model_config(
        &model_name,
        matched_model.and_then(|item| item.context_window_tokens),
    ))
}

fn validate_system_agent_model(
    system_agent: &ManagedSystemAgentRecord,
    model_name: &str,
) -> Result<Option<crate::domain::agent::SystemAgentModelRecord>, ChatSendError> {
    if !system_agent.enabled || !system_agent.available {
        return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
            format!(
                "agent `{}` is unavailable or disabled for this workspace",
                system_agent.agent_code
            ),
        )));
    }

    if system_agent.models.is_empty() {
        return Ok(None);
    }

    let model_matched = system_agent
        .models
        .iter()
        .find(|item| item.model_id.eq_ignore_ascii_case(model_name));
    if let Some(model) = model_matched {
        return Ok(Some(model.clone()));
    }

    Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
        format!(
            "model `{model_name}` is not available for agent `{}`",
            system_agent.agent_code
        ),
    )))
}
