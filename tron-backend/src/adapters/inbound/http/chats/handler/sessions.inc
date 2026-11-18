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

pub async fn list_archived_sessions(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Query(query): Query<ArchivedChatSessionQuery>,
) -> Response {
    let current = query.current.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(25).clamp(1, 100);
    match state
        .chats
        .list_archived_sessions(query.workspace_code.as_deref(), current, size)
    {
        Ok((records, total)) => Json(ApiResponse::protocol_success(
            super::response::ArchivedSessionPageResponse {
                records: records
                    .into_iter()
                    .map(super::response::ArchivedSessionResponse::from)
                    .collect(),
                total,
                current,
                size,
            },
        ))
        .into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn restore_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    match state.chats.restore_session(&session_code) {
        Ok(session) => Json(ApiResponse::protocol_success(
            super::response::ArchivedSessionResponse::from(session),
        ))
        .into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn batch_delete_archived_sessions(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    request: Result<Json<BatchDeleteArchivedSessionsCommand>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<EmptyData>::error(
                    "CHAT_BATCH_DELETE_INVALID",
                    error.body_text(),
                )),
            )
                .into_response();
        }
    };
    let mut codes = Vec::with_capacity(request.session_codes.len());
    let mut seen = BTreeSet::new();
    for code in request.session_codes {
        let code = code.trim().to_owned();
        if code.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<EmptyData>::error(
                    "CHAT_BATCH_DELETE_INVALID",
                    "sessionCodes must not contain empty values",
                )),
            )
                .into_response();
        }
        if seen.insert(code.clone()) {
            codes.push(code);
        }
    }
    if codes.is_empty() || codes.len() > MAX_ARCHIVED_SESSION_DELETE_BATCH {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<EmptyData>::error(
                "CHAT_BATCH_DELETE_INVALID",
                format!("sessionCodes must contain between 1 and {MAX_ARCHIVED_SESSION_DELETE_BATCH} unique values"),
            )),
        )
            .into_response();
    }

    let chunk_size = codes.len().div_ceil(ARCHIVED_SESSION_DELETE_WORKERS);
    let mut workers = JoinSet::new();
    for chunk in codes.chunks(chunk_size) {
        let chats = state.chats.clone();
        let chunk = chunk.to_vec();
        workers.spawn_blocking(move || {
            chunk
                .into_iter()
                .map(|session_code| match chats.delete_archived_session(&session_code) {
                    Ok(_) => BatchDeleteArchivedSessionItem {
                        session_code,
                        success: true,
                        error_code: None,
                        message: None,
                    },
                    Err(error) => chat_delete_failure(session_code, error),
                })
                .collect::<Vec<_>>()
        });
    }
    let mut items = Vec::with_capacity(codes.len());
    while let Some(result) = workers.join_next().await {
        match result {
            Ok(mut completed) => items.append(&mut completed),
            Err(_) => {}
        }
    }
    let completed_codes = items
        .iter()
        .map(|item| item.session_code.clone())
        .collect::<BTreeSet<_>>();
    for code in &codes {
        if !completed_codes.contains(code) {
            items.push(BatchDeleteArchivedSessionItem {
                session_code: code.clone(),
                success: false,
                error_code: Some("CHAT_SESSION_DELETE_FAILED".to_string()),
                message: Some("deletion worker failed; retry this item".to_string()),
            });
        }
    }
    let succeeded = items.iter().filter(|item| item.success).count() as u32;
    let failed = items.len() as u32 - succeeded;
    items.sort_by_key(|item| codes.iter().position(|code| code == &item.session_code));
    Json(ApiResponse::protocol_success(
        BatchDeleteArchivedSessionsResult {
            succeeded,
            failed,
            items,
        },
    ))
    .into_response()
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
    match tokio::task::spawn_blocking({
        let chats = state.chats.clone();
        move || chats.delete_session(&session_code)
    })
    .await
    {
        Ok(Ok(_)) => Json(ApiResponse::protocol_success(EmptyData {})).into_response(),
        Ok(Err(error)) => chat_error_response(error),
        Err(_) => chat_error_response(ChatError::StorageFailure),
    }
}

pub async fn delete_archived_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    match tokio::task::spawn_blocking({
        let chats = state.chats.clone();
        move || chats.delete_archived_session(&session_code)
    })
    .await
    {
        Ok(Ok(session)) => Json(ApiResponse::protocol_success(
            super::response::ArchivedSessionResponse::from(session),
        ))
        .into_response(),
        Ok(Err(error)) => chat_error_response(error),
        Err(_) => chat_error_response(ChatError::StorageFailure),
    }
}

fn chat_delete_failure(session_code: String, error: ChatError) -> BatchDeleteArchivedSessionItem {
    let (code, message) = match error {
        ChatError::SessionNotFound => ("CHAT_SESSION_NOT_FOUND", "chat session was not found"),
        ChatError::SessionMustBeArchived => (
            "CHAT_SESSION_MUST_BE_ARCHIVED",
            "only archived chat sessions can be permanently deleted",
        ),
        ChatError::SessionRunActive => {
            ("CHAT_SESSION_RUN_ACTIVE", "chat session has an active run")
        }
        ChatError::SessionArchived => ("CHAT_SESSION_ARCHIVED", "chat session is archived"),
        _ => (
            "CHAT_SESSION_DELETE_FAILED",
            "chat session cleanup failed; retry this item",
        ),
    };
    failed_delete_item(session_code, code, message)
}

fn failed_delete_item(
    session_code: String,
    error_code: &str,
    message: &str,
) -> BatchDeleteArchivedSessionItem {
    BatchDeleteArchivedSessionItem {
        session_code,
        success: false,
        error_code: Some(error_code.to_owned()),
        message: Some(message.to_owned()),
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

pub async fn download_archived_session_attachment(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session, attachment)): Path<(String, String)>,
) -> Response {
    match state.chats.get_session(&session) {
        Ok(record) if record.status == crate::domain::chat::ChatSessionStatus::Archived => {}
        Ok(_) => return chat_error_response(ChatError::SessionMustBeArchived),
        Err(error) => return chat_error_response(error),
    }
    download_session_attachment(State(state), auth, Path((session, attachment))).await
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMessagesQuery {
    #[serde(default)]
    pub artifacts_actions: Option<String>,
}

fn parse_artifact_actions(value: Option<&str>) -> Option<Vec<crate::domain::chat::ArtifactAction>> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    let mut actions = Vec::new();
    for raw in value.split(',') {
        let action = match raw.trim() {
            "preview" => crate::domain::chat::ArtifactAction::Preview,
            "open" => crate::domain::chat::ArtifactAction::Open,
            "reveal" => crate::domain::chat::ArtifactAction::Reveal,
            "save" => crate::domain::chat::ArtifactAction::Save,
            _ => continue,
        };
        if !actions.contains(&action) {
            actions.push(action);
        }
    }
    if actions.is_empty() { None } else { Some(actions) }
}

pub async fn list_messages(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
    Query(query): Query<ListMessagesQuery>,
) -> Response {
    if state.chats.get_session(&session_code).is_err() {
        return chat_error_response(ChatError::SessionNotFound);
    }
    let actions = parse_artifact_actions(query.artifacts_actions.as_deref());
    match state
        .chats
        .list_messages_with_actions(&session_code, actions.as_deref())
    {
        Ok(messages) => Json(ApiResponse::success(messages)).into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn list_user_input_requests(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    match state.chats.list_user_input_requests(&session_code) {
        Ok(requests) => Json(ApiResponse::success(
            requests
                .into_iter()
                .map(ChatUserInputRequestResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn answer_user_input(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Path((session_code, request_code)): Path<(String, String)>,
    request: Result<Json<AnswerChatUserInputRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match request {
        Ok(request) => request,
        Err(error) => return steer_chat_message_request_rejection_response(error),
    };
    if let Err(error) = state.chats.get_session(&session_code) {
        return chat_error_response(error);
    }
    if let Err(error) = state.chats.ensure_session_writable(&session_code) {
        return chat_error_response(error);
    }
    if let Err(response) = ensure_no_active_run(&state, &session_code) {
        return response;
    }
    let answers = request.answers.or_else(|| {
        request
            .resume
            .first()
            .and_then(|entry| entry.payload.get("answers"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    });
    if eliteforge_agent_core::RunId::parse_str(&request_code).is_err() {
        return chat_error_response(ChatError::UserInputRequestNotFound);
    }
    if !request.resume.is_empty()
        && !request.resume.iter().any(|entry| entry.interrupt_id == request_code)
    {
        return chat_error_response(ChatError::UserInputAnswerInvalid(
            "response identity does not match the interaction".into(),
        ));
    }
    let entries = if request.resume.is_empty() {
        match answers {
            Some(answers) => vec![crate::application::chat::ChatResumeEntry {
                interrupt_id: request_code,
                status: "resolved".into(),
                payload: json!({"answers":answers}),
            }],
            None => {
                return chat_error_response(ChatError::UserInputAnswerInvalid(
                    "interaction response is required".into(),
                ));
            }
        }
    } else {
        request.resume
    };
    let interaction_run = match state
        .agent_framework
        .validate_native_resume(&session_code, &entries)
        .await
    {
        Ok(Some(run)) => run.to_string(),
        Ok(None) => return chat_error_response(ChatError::UserInputRequestNotFound),
        Err(error) => {
            return chat_error_response(ChatError::UserInputAnswerInvalid(error.to_string()));
        }
    };
    let history = match state.agent_runs.get_run(&interaction_run) {
        Ok(record) => record,
        Err(error) => {
            tracing::error!(%session_code, run_code = %interaction_run, ?error,
                "failed to load interaction run for answer submission");
            return chat_error_response(ChatError::StorageFailure);
        }
    };
    let mut command = match state.chats.native_resume_command(
        &session_code,
        entries,
        Some(history.model_code),
        history.reasoning_effort,
    ) {
        Ok(command) => command,
        Err(error) => return chat_error_response(error),
    };
    if request.client_capabilities.is_some() {command.client_capabilities=request.client_capabilities;}
    command.a2ui_metadata=request.a2ui_metadata;
    stream_message_command(state, auth, session_code, command).await
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

pub async fn archived_session_detail(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(code): Path<String>,
) -> Response {
    let session = match state.chats.get_session(&code) {
        Ok(s) => s,
        Err(e) => return chat_error_response(e),
    };
    if session.status != crate::domain::chat::ChatSessionStatus::Archived {
        return chat_error_response(ChatError::SessionMustBeArchived);
    }
    let result = async {
        let framework = state.agent_framework.storage_framework().await.map_err(|_| ChatError::StorageFailure)?;
        let database = state.agent_framework.shared_database().await.map_err(|_| ChatError::StorageFailure)?;
        database.transaction(async {
        let native = framework.sessions().map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?.get(&code).await?;
        if native.status != eliteforge_agent_core::SessionStatus::Archived {
            return Err(eliteforge_agent_core::StateError::Invalid("session must be archived".into()));
        }
        let mut events = Vec::new();
        let mut after = 0;
        loop {
            let page = framework.sessions().map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?.store.list_events(&code, after, 500).await?;
            if page.is_empty() { break; }
            after = page.last().unwrap().sequence;
            events.extend(page);
        }
        let messages = events.iter().filter(|e| e.kind == "tron.message")
            .map(|e| serde_json::from_value::<crate::domain::chat::ChatMessage>(e.payload.clone()).map_err(|_| eliteforge_agent_core::StateError::Backend("invalid message event".into())))
            .collect::<Result<Vec<_>, _>>()?;
        let goal_events = events.into_iter().filter(|e| e.goal_id.is_some()).map(Into::into).collect();
        let goals = framework
            .goals()
            .map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?
            .list(&code)
            .await
            ?;
        let runs = framework
            .runs()
            .map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?
            .list(&code)
            .await
            ?;
        let conversations = framework
            .services()
            .map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?
            .get::<eliteforge_agent_core::ConversationStoreService>()
            .ok_or(eliteforge_agent_core::StateError::Backend("conversation store unavailable".into()))?
            .0
            .clone();
        let history = conversations
            .history(&code)
            .await
            .map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?;
        Ok::<_, eliteforge_agent_core::StateError>(super::response::ArchivedSessionDetailResponse {
            a2ui_state: match framework.services().map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?
                .get::<eliteforge_agent_official_plugin_a2ui::A2uiService>() {
                Some(service)=>service.state(&code).await.map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?,
                None=>eliteforge_agent_official_plugin_a2ui::A2uiState {session_id:code.clone(),revision:0,surfaces:vec![],pending_interactions:vec![]},
            },
            session: session.into(),
            messages: messages.into_iter().map(Into::into).collect(),
            goals: goals.into_iter().map(Into::into).collect(),
            goal_events,
            runs: runs.into_iter().map(Into::into).collect(),
            history: history.into_iter().map(Into::into).collect(),
        })
        }).await.map_err(crate::adapters::outbound::persistence::sqlite::framework_storage::chat_error)
    }
    .await;
    match result {
        Ok(detail) => Json(ApiResponse::protocol_success(detail)).into_response(),
        Err(error) => chat_error_response(error),
    }
}
