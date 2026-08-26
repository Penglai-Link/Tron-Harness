pub async fn list_agui_stream_events(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(stream_key): Path<String>,
) -> Response {
    let service = AgentAguiStreamService::with_replay_store(
        state.agui_events.clone(),
        state.agui_replay_store.clone(),
    );
    match service.events(&stream_key) {
        Ok(result) => Json(ApiResponse::success(
            OpenApiAguiEventSnapshotResponse::from(result),
        ))
        .into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}

pub async fn list_agui_stream_protocol_events(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(stream_key): Path<String>,
) -> Response {
    let service = AgentAguiStreamService::with_replay_store(
        state.agui_events.clone(),
        state.agui_replay_store.clone(),
    );
    match service.protocol_events(&stream_key) {
        Ok(result) => Json(ApiResponse::success(
            result
                .into_iter()
                .map(OpenApiAguiProtocolEventResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => agent_agui_error_response(error),
    }
}

fn map_send_request(
    request: OpenApiSendChatMessageRequest,
) -> crate::application::chat::SendChatMessageCommand {
    crate::application::chat::SendChatMessageCommand {
        run_code: request.run_code,
        content: request.content,
        workspace_code: request.workspace_code,
        app_code: request.app_code,
        task_code: request.task_code,
        agent_code: request.agent_code,
        model_code: request.model_code,
        system_model_id: request.system_model_id,
        reasoning_effort: request.reasoning_effort,
        limits: request.limits,
        tools: request.tools,
        attachments: request.attachments,
        client_capabilities: request.client_capabilities,
        resume: request.resume,
    }
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
    Success(crate::application::chat::ChatSendResult),
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
                    "result": OpenApiChatSendResultResponse::from(result.clone()),
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
