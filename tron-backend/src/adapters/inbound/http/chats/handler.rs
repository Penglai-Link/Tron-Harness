use std::{collections::BTreeSet, convert::Infallible, io::Seek, sync::Arc, time::Duration};

use crate::{
    adapters::inbound::http::auth::RequestAuthContext,
    app::AppState,
    application::{
        agent::{AgentAguiStreamService, AgentInvocationService, generate_agent_run_code},
        chat::{
            ChatSendError, ChatSessionAttachmentUploadCommand, ChatSessionQuery,
            CreateChatSessionCommand, SendChatMessageCommand, SteerChatMessageCommand,
            UpdateChatSessionCommand,
        },
    },
    domain::{
        agent::{AgentRuntimeType, ModelStreamEvent},
        chat::ChatError,
    },
    response::ApiResponse,
    shared::auth_holder::AuthHolder,
};
use axum::{
    Json,
    body::Body,
    extract::{
        DefaultBodyLimit, Multipart, Path, Query, State, multipart::MultipartRejection,
        rejection::JsonRejection,
    },
    http::{
        HeaderValue, StatusCode,
        header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use serde_json::{Value, json};

use super::error_mapper::{
    chat_error_response, chat_send_error_response, chat_steer_error_response,
    steer_chat_message_request_rejection_response,
};
use super::protocol_replay::record_protocol_event;
use super::request::SteerChatMessageRequest;
use super::response::ChatSteerResultResponse;
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

pub async fn upload_session_attachment(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
    request: Result<Multipart, MultipartRejection>,
) -> Response {
    let multipart = match request {
        Ok(request) => request,
        Err(error) => return chat_attachment_request_error(error.status()),
    };
    let maximum = state.resources.max_chat_session_attachment_bytes();
    let form = match parse_session_attachment_form(multipart, maximum).await {
        Ok(form) => form,
        Err(error) => return chat_attachment_form_error_response(error),
    };
    let chats = state.chats.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut file = form.file;
        file.seek(std::io::SeekFrom::Start(0))
            .map_err(|_| ChatError::StorageFailure)?;
        chats.store_session_attachment_from_reader(
            &session_code,
            ChatSessionAttachmentUploadCommand {
                kind: form.kind,
                summary: form.summary,
                metadata: form.metadata,
                file_name: form.file_name,
                content_type: form.content_type,
            },
            &mut file,
        )
    })
    .await;
    match result {
        Ok(Ok(attachment)) => {
            (StatusCode::CREATED, Json(ApiResponse::success(attachment))).into_response()
        }
        Ok(Err(error)) => chat_error_response(error),
        Err(_) => chat_error_response(ChatError::StorageFailure),
    }
}

pub async fn download_session_attachment(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path((session_code, attachment_code)): Path<(String, String)>,
) -> Response {
    let (record, bytes) = match state
        .chats
        .read_session_attachment(&session_code, &attachment_code)
    {
        Ok(result) => result,
        Err(error) => return chat_error_response(error),
    };
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&record.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(value) = HeaderValue::from_str(&record.byte_size.to_string()) {
        response.headers_mut().insert(CONTENT_LENGTH, value);
    }
    if let Ok(value) = chat_attachment_content_disposition(&record.file_name) {
        response.headers_mut().insert(CONTENT_DISPOSITION, value);
    }
    response
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
    if let Err(response) = ensure_no_active_run(&state, &session_code) {
        return response;
    }
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
    )
    .with_short_term_memory(state.short_term_memory.clone())
    .with_active_runs(state.active_runs.clone());
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

pub(crate) fn ensure_no_active_run(state: &AppState, session_code: &str) -> Result<(), Response> {
    match state.active_runs.snapshot(session_code) {
        Ok(None) => Ok(()),
        Ok(Some(active_run)) => Err(chat_send_error_response(ChatSendError::RunAlreadyActive {
            active_run_code: Some(active_run.run_code),
        })),
        Err(error) => Err(chat_send_error_response(ChatSendError::Agent(
            crate::application::agent::AgentInvocationError::RuntimeFailed(error.to_string()),
        ))),
    }
}

pub async fn steer_message(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    request: Result<Json<SteerChatMessageRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(error) => return steer_chat_message_request_rejection_response(error),
    };
    match state.chats.steer_message(SteerChatMessageCommand {
        session_code: request.session_code,
        expected_run_code: request.expected_run_code,
        content: request.content,
        attachments: request.attachments,
    }) {
        Ok(result) => {
            Json(ApiResponse::success(ChatSteerResultResponse::from(result))).into_response()
        }
        Err(error) => chat_steer_error_response(error),
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
    if let Err(response) = ensure_no_active_run(&state, &session_code) {
        return response;
    }
    let resolved_runtime = match resolve_chat_agent_runtime(&state, &session, &request) {
        Ok(runtime) => runtime,
        Err(error) => return chat_send_error_response(error),
    };
    let system_agent_live_stream = uses_system_live_stream(&resolved_runtime);
    let supports_steer = matches!(resolved_runtime.runtime_type, AgentRuntimeType::React);
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
    )
    .with_short_term_memory(state.short_term_memory.clone())
    .with_active_runs(state.active_runs.clone());
    let live_replay_recorder_for_sink = live_replay_recorder.clone();
    let forwarding_gate = ChatStreamForwardingGate::new();
    let forwarding_gate_for_sink = forwarding_gate.clone();
    let sink = Arc::new(move |event: ModelStreamEvent| {
        live_replay_recorder_for_sink.ingest(event.clone());
        if forwarding_gate_for_sink.is_open() {
            let _ = model_event_sender.send(event);
        }
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
    let lifecycle_guard = ChatStreamLifecycleGuard::new(
        run_task,
        forwarding_gate,
        stream_service.clone(),
        stream_key.clone(),
        stream_run_code.clone(),
    );
    let stream = async_stream::stream! {
        let mut lifecycle_guard = lifecycle_guard;
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
                    let data = run_started_with_supports_steer(
                        &payload.sse_event,
                        payload.data,
                        supports_steer,
                    );
                    let key = format!("{}:{}", payload.sse_event, data);
                    if seen.insert(key) {
                        emitted = true;
                        yield Ok::<Event, Infallible>(
                            ChatSsePayload::agui(payload.sse_event, data).event()
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
                    let data = run_started_with_supports_steer(
                        &event.sse_event,
                        event.data,
                        supports_steer,
                    );
                    let key = format!("{}:{}", event.sse_event, data);
                    if seen.insert(key) {
                        emitted = true;
                        yield Ok::<Event, Infallible>(ChatSsePayload::agui(event.sse_event, data).event());
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

pub(crate) fn run_started_with_supports_steer(
    sse_event: &str,
    mut data: Value,
    supports_steer: bool,
) -> Value {
    if sse_event == "RUN_STARTED"
        && let Some(object) = data.as_object_mut()
    {
        object.insert("supportsSteer".to_string(), Value::Bool(supports_steer));
    }
    data
}

const MAX_CHAT_ATTACHMENT_BODY_OVERHEAD_BYTES: u64 = 1_048_576;
const MAX_CHAT_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
const MAX_CHAT_ATTACHMENT_KIND_BYTES: usize = 64;
const MAX_CHAT_ATTACHMENT_SUMMARY_BYTES: usize = 2_048;
const MAX_CHAT_ATTACHMENT_METADATA_BYTES: usize = 256 * 1024;

struct SessionAttachmentForm {
    kind: String,
    summary: Option<String>,
    metadata: Value,
    file_name: String,
    content_type: String,
    file: std::fs::File,
}

enum SessionAttachmentFormError {
    Invalid,
    TooLarge,
    Storage,
}

pub fn chat_attachment_body_limit() -> DefaultBodyLimit {
    let maximum = MAX_CHAT_ATTACHMENT_BYTES
        .saturating_add(MAX_CHAT_ATTACHMENT_BODY_OVERHEAD_BYTES)
        .min(usize::MAX as u64) as usize;
    DefaultBodyLimit::max(maximum)
}

async fn parse_session_attachment_form(
    mut multipart: Multipart,
    maximum_bytes: u64,
) -> Result<SessionAttachmentForm, SessionAttachmentFormError> {
    let mut seen = BTreeSet::new();
    let mut kind = None;
    let mut summary = None;
    let mut metadata = None;
    let mut file = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| SessionAttachmentFormError::Invalid)?
    {
        let name = field
            .name()
            .map(str::to_owned)
            .ok_or(SessionAttachmentFormError::Invalid)?;
        if !seen.insert(name.clone()) {
            return Err(SessionAttachmentFormError::Invalid);
        }
        match name.as_str() {
            "kind" => {
                kind = Some(
                    read_session_attachment_text(field, MAX_CHAT_ATTACHMENT_KIND_BYTES).await?,
                );
            }
            "summary" => {
                summary = Some(
                    read_session_attachment_text(field, MAX_CHAT_ATTACHMENT_SUMMARY_BYTES).await?,
                );
            }
            "metadata" => {
                let value =
                    read_session_attachment_text(field, MAX_CHAT_ATTACHMENT_METADATA_BYTES).await?;
                metadata = Some(
                    serde_json::from_str(&value)
                        .map_err(|_| SessionAttachmentFormError::Invalid)?,
                );
            }
            "file" => {
                let file_name = field
                    .file_name()
                    .map(str::to_owned)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(SessionAttachmentFormError::Invalid)?;
                let content_type = field
                    .content_type()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                let mut output = tokio::fs::File::from_std(
                    tempfile::tempfile().map_err(|_| SessionAttachmentFormError::Storage)?,
                );
                let mut received = 0_u64;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|_| SessionAttachmentFormError::Invalid)?
                {
                    received = received
                        .checked_add(chunk.len() as u64)
                        .ok_or(SessionAttachmentFormError::TooLarge)?;
                    if received > maximum_bytes {
                        return Err(SessionAttachmentFormError::TooLarge);
                    }
                    tokio::io::AsyncWriteExt::write_all(&mut output, &chunk)
                        .await
                        .map_err(|_| SessionAttachmentFormError::Storage)?;
                }
                tokio::io::AsyncWriteExt::flush(&mut output)
                    .await
                    .map_err(|_| SessionAttachmentFormError::Storage)?;
                file = Some((file_name, content_type, output.into_std().await));
            }
            _ => return Err(SessionAttachmentFormError::Invalid),
        }
    }

    let (file_name, content_type, file) = file.ok_or(SessionAttachmentFormError::Invalid)?;
    let kind = kind
        .unwrap_or_else(|| {
            if content_type.starts_with("image/") {
                "image".to_string()
            } else {
                "file".to_string()
            }
        })
        .trim()
        .to_ascii_lowercase();
    if !matches!(kind.as_str(), "file" | "image" | "inspection") {
        return Err(SessionAttachmentFormError::Invalid);
    }
    let summary = summary.filter(|value| !value.trim().is_empty());
    Ok(SessionAttachmentForm {
        kind,
        summary,
        metadata: metadata.unwrap_or_else(|| json!({})),
        file_name,
        content_type,
        file,
    })
}

async fn read_session_attachment_text(
    mut field: axum::extract::multipart::Field<'_>,
    maximum: usize,
) -> Result<String, SessionAttachmentFormError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|_| SessionAttachmentFormError::Invalid)?
    {
        let length = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or(SessionAttachmentFormError::TooLarge)?;
        if length > maximum {
            return Err(SessionAttachmentFormError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| SessionAttachmentFormError::Invalid)
}

fn chat_attachment_form_error_response(error: SessionAttachmentFormError) -> Response {
    match error {
        SessionAttachmentFormError::Invalid => chat_error_response(ChatError::AttachmentInvalid(
            "attachment upload form is invalid".to_string(),
        )),
        SessionAttachmentFormError::TooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiResponse::error(
                "CHAT_ATTACHMENT_TOO_LARGE",
                "chat attachment exceeds the maximum allowed size",
            )),
        )
            .into_response(),
        SessionAttachmentFormError::Storage => chat_error_response(ChatError::StorageFailure),
    }
}

fn chat_attachment_request_error(status: StatusCode) -> Response {
    if status == StatusCode::PAYLOAD_TOO_LARGE {
        return chat_attachment_form_error_response(SessionAttachmentFormError::TooLarge);
    }
    chat_attachment_form_error_response(SessionAttachmentFormError::Invalid)
}

fn chat_attachment_content_disposition(
    file_name: &str,
) -> Result<HeaderValue, axum::http::header::InvalidHeaderValue> {
    let mut encoded = String::new();
    for byte in file_name.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                *byte,
                b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            )
        {
            encoded.push(*byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{encoded}"))
}
