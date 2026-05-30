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
        agent::{AgentAguiStreamService, AgentInvocationService, generate_agent_run_code},
        chat::{
            ChatSendError, ChatSendResult, ChatSessionQuery, CreateChatSessionCommand,
            SendChatMessageCommand, UpdateChatSessionCommand,
        },
    },
    domain::{agent::ModelStreamEvent, chat::ChatError},
    response::ApiResponse,
};

use super::error_mapper::{chat_error_response, chat_send_error_response};

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
    Json(mut request): Json<SendChatMessageCommand>,
) -> Response {
    let Ok(session) = state.chats.get_session(&session_code) else {
        return chat_error_response(ChatError::SessionNotFound);
    };
    if request.model.is_none() {
        match state
            .models
            .text_agent_model_config(&session.workspace_code, request.model_code.as_deref())
        {
            Ok(model) => request.model = model,
            Err(_) => {
                return chat_send_error_response(ChatSendError::Chat(ChatError::StorageFailure));
            }
        }
    }

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
    );
    match state
        .chats
        .send_message(
            &session_code,
            request,
            Some(auth.user().user_code.clone()),
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
    if request.model.is_none() {
        match state
            .models
            .text_agent_model_config(&session.workspace_code, request.model_code.as_deref())
        {
            Ok(model) => request.model = model,
            Err(_) => {
                return chat_send_error_response(ChatSendError::Chat(ChatError::StorageFailure));
            }
        }
    }

    let run_code = generate_agent_run_code();
    request.run_code = Some(run_code.clone());

    let agui_stream = AgentAguiStreamService::new(state.agui_events.clone());
    let _ = agui_stream.activate(&session_code);

    let (completion_sender, mut completion_receiver) =
        tokio::sync::mpsc::channel::<ChatStreamCompletion>(1);
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
    );
    let sink = Arc::new(move |_event: ModelStreamEvent| {});

    tokio::spawn(async move {
        let result = chats
            .send_message_stream(
                &session_code_for_task,
                request,
                user_code,
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
        let mut drain_ticks = 0u8;
        let mut interval = tokio::time::interval(Duration::from_millis(25));

        loop {
            interval.tick().await;

            if completion.is_none() {
                if let Ok(received) = completion_receiver.try_recv() {
                    completion = Some(received);
                }
            }

            let mut emitted = false;
            if let Ok(events) = stream_service.protocol_events(&stream_key) {
                for event in events {
                    if !protocol_event_belongs_to_run(&event.data, &stream_run_code) {
                        continue;
                    }
                    if matches!(event.sse_event.as_str(), "RUN_FINISHED" | "RUN_ERROR") {
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
                if emitted {
                    drain_ticks = 0;
                    continue;
                }
                drain_ticks = drain_ticks.saturating_add(1);
                if drain_ticks < 4 {
                    continue;
                }
                yield Ok::<Event, Infallible>(done.payload(&stream_key, &stream_run_code).event());
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
