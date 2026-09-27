pub async fn send_message(
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
    if let Err(response) = ensure_no_pending_user_input(&state, &session_code) {
        return response;
    }
    let resolved_runtime = match resolve_chat_agent_runtime(&state, &session, &request) {
        Ok(runtime) => runtime,
        Err(error) => return chat_send_error_response(error),
    };
    let run_code = generate_agent_run_code();
    request.run_code = Some(run_code.clone());
    let prepared_model = match prepare_send_model(&state, &session, &request, &resolved_runtime) {
        Ok(model) => model,
        Err(error) => return chat_send_error_response(error),
    };
    let auto_model_route = prepared_model.auto_route.clone();
    let auto_model_router = prepared_model.auto_router.clone();
    let resolved_model = prepared_model.config;
    if let Err(error) = state.model_providers.record_requested_runtime_snapshot(
        Some(&run_code),
        &session.session_code,
        &session.workspace_code,
        &resolved_model,
    ) {
        return chat_send_error_response(ChatSendError::Agent(
            crate::application::agent::AgentInvocationError::RuntimeFailed(error.to_string()),
        ));
    }
    let resolved_model = crate::adapters::outbound::model::with_person_ai_conversation(
        resolved_model,
        &session.session_code,
        Some(&run_code),
    );

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
    .with_framework_host(state.agent_framework.clone())
    .with_directory_inspector(Some(std::sync::Arc::new(
        crate::adapters::outbound::application_directory::GitApplicationDirectoryInspector,
    )))
    .with_short_term_memory(state.short_term_memory.clone())
    .with_active_runs(state.active_runs.clone())
    .with_prompt_service(state.prompts.clone())
    .with_auto_model_route(auto_model_route, auto_model_router);
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
    Json(request): Json<SendChatMessageCommand>,
) -> Response {
    stream_message_command(state, auth, session_code, request).await
}

pub(crate) async fn stream_message_command(
    state: AppState,
    auth: RequestAuthContext,
    session_code: String,
    mut request: SendChatMessageCommand,
) -> Response {
    let Ok(session) = state.chats.get_session(&session_code) else {
        return chat_error_response(ChatError::SessionNotFound);
    };
    if let Err(response) = ensure_no_active_run(&state, &session_code) {
        return response;
    }
    if let Err(response) = ensure_no_pending_user_input(&state, &session_code) {
        return response;
    }
    let resolved_runtime = match resolve_chat_agent_runtime(&state, &session, &request) {
        Ok(runtime) => runtime,
        Err(error) => return chat_send_error_response(error),
    };
    let system_agent_live_stream = uses_system_live_stream(&resolved_runtime);
    let supports_steer = matches!(resolved_runtime.runtime_type, AgentRuntimeType::React);
    let run_code = generate_agent_run_code();
    request.run_code = Some(run_code.clone());
    let prepared_model = match prepare_send_model(&state, &session, &request, &resolved_runtime) {
        Ok(model) => model,
        Err(error) => return chat_send_error_response(error),
    };
    let auto_model_route = prepared_model.auto_route.clone();
    let auto_model_router = prepared_model.auto_router.clone();
    let resolved_model = prepared_model.config;
    if let Err(error) = state.model_providers.record_requested_runtime_snapshot(
        Some(&run_code),
        &session.session_code,
        &session.workspace_code,
        &resolved_model,
    ) {
        return chat_send_error_response(ChatSendError::Agent(
            crate::application::agent::AgentInvocationError::RuntimeFailed(error.to_string()),
        ));
    }
    let resolved_model = crate::adapters::outbound::model::with_person_ai_conversation(
        resolved_model,
        &session.session_code,
        Some(&run_code),
    );

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
    live_replay_recorder.start();
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
    .with_framework_host(state.agent_framework.clone())
    .with_directory_inspector(Some(std::sync::Arc::new(
        crate::adapters::outbound::application_directory::GitApplicationDirectoryInspector,
    )))
    .with_short_term_memory(state.short_term_memory.clone())
    .with_active_runs(state.active_runs.clone())
    .with_prompt_service(state.prompts.clone())
    .with_auto_model_route(auto_model_route, auto_model_router);
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

        for payload in live_event_builder.start() {
            let data = run_started_with_supports_steer(
                &payload.sse_event,
                payload.data,
                supports_steer,
            );
            let key = format!("{}:{}", payload.sse_event, data);
            if seen.insert(key) {
                yield Ok::<Event, Infallible>(ChatSsePayload::agui(payload.sse_event, data).event());
            }
        }

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
                                | "ACTIVITY_SNAPSHOT"
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

pub(crate) fn ensure_no_pending_user_input(
    state: &AppState,
    session_code: &str,
) -> Result<(), Response> {
    match state.chats.list_user_input_requests(session_code) {
        Ok(requests)
            if requests.iter().any(|request| {
                request.status == crate::domain::chat::ChatUserInputRequestStatus::Pending
            }) =>
        {
            Err(chat_error_response(ChatError::UserInputPending))
        }
        Ok(_) => Ok(()),
        Err(error) => Err(chat_error_response(error)),
    }
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
