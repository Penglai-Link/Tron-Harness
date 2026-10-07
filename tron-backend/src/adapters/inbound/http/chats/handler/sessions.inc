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
        Ok((records, total)) => Json(ApiResponse::success(ArchivedChatSessionPage {
            records,
            total,
            current,
            size,
        }))
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
        Ok(session) => Json(ApiResponse::success(session)).into_response(),
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
        let preflight_chats = chats.clone();
        let framework = state.agent_framework.clone();
        let chunk = chunk.to_vec();
        workers.spawn(async move {
            let codes_to_check = chunk.clone();
            let checked = tokio::task::spawn_blocking(move || {
                let mut eligible = Vec::new();
                let mut items = Vec::new();
                for session_code in codes_to_check {
                    match preflight_chats.prepare_session_deletion(&session_code) {
                        Ok(()) => eligible.push(session_code),
                        Err(error) => items.push(chat_delete_failure(session_code, error)),
                    }
                }
                (eligible, items)
            })
            .await;
            let Ok((eligible, mut items)) = checked else {
                return chunk
                    .into_iter()
                    .map(framework_delete_failure)
                    .collect::<Vec<_>>();
            };
            if eligible.is_empty() {
                return items;
            }

            let framework_result = framework.delete_persisted_sessions(eligible.clone()).await;
            let report = match framework_result {
                Ok(report) => report,
                Err(_) => {
                    items.extend(eligible.into_iter().map(framework_delete_failure));
                    return items;
                }
            };
            let framework_failures = report
                .failures
                .into_iter()
                .map(|failure| {
                    let item = if matches!(&failure.error, eliteforge_agent_core::StateError::Busy)
                    {
                        active_session_delete_failure(failure.session_id.clone())
                    } else {
                        framework_delete_failure(failure.session_id.clone())
                    };
                    (failure.session_id, item)
                })
                .collect::<BTreeMap<_, _>>();
            items.extend(framework_failures.values().cloned());
            let ready = eligible
                .into_iter()
                .filter(|session_code| !framework_failures.contains_key(session_code))
                .collect::<Vec<_>>();
            if ready.is_empty() {
                return items;
            }

            match tokio::task::spawn_blocking(move || {
                ready
                    .into_iter()
                    .map(|session_code| match chats.delete_session(&session_code) {
                        Ok(_) => BatchDeleteArchivedSessionItem {
                            session_code,
                            success: true,
                            error_code: None,
                            message: None,
                        },
                        Err(error) => chat_delete_failure(session_code, error),
                    })
                    .collect::<Vec<_>>()
            })
            .await
            {
                Ok(mut local_items) => items.append(&mut local_items),
                Err(_) => {}
            }
            items
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
    Json(ApiResponse::success(BatchDeleteArchivedSessionsResult {
        succeeded,
        failed,
        items,
    }))
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
    let chats = state.chats.clone();
    let session_code_for_preflight = session_code.clone();
    match tokio::task::spawn_blocking(move || {
        chats.prepare_session_deletion(&session_code_for_preflight)
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return chat_error_response(error),
        Err(_) => return chat_error_response(ChatError::StorageFailure),
    }
    match state
        .agent_framework
        .delete_persisted_session(&session_code)
        .await
    {
        Ok(_) => {}
        Err(error) => {
            if matches!(
                error.primary(),
                eliteforge_agent_core::AgentError::State(eliteforge_agent_core::StateError::Busy)
            ) {
                return chat_error_response(ChatError::SessionRunActive);
            }
            return chat_error_response(ChatError::StorageFailure);
        }
    }
    match tokio::task::spawn_blocking({
        let chats = state.chats.clone();
        move || chats.delete_session(&session_code)
    })
    .await
    {
        Ok(Ok(session)) => Json(ApiResponse::success(session)).into_response(),
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

fn active_session_delete_failure(session_code: String) -> BatchDeleteArchivedSessionItem {
    failed_delete_item(
        session_code,
        "CHAT_SESSION_RUN_ACTIVE",
        "chat session has an active Framework run",
    )
}

fn framework_delete_failure(session_code: String) -> BatchDeleteArchivedSessionItem {
    failed_delete_item(
        session_code,
        "CHAT_SESSION_DELETE_FAILED",
        "Framework session cleanup failed; retry this item",
    )
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
    let session = match state.chats.get_session(&session_code) {
        Ok(session) => session,
        Err(error) => return chat_error_response(error),
    };
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
    let native_id = request_code
        .strip_prefix("INPUT-FW-")
        .unwrap_or(&request_code);
    let is_native = eliteforge_agent_core::RunId::parse_str(native_id).is_ok()
        || request_code.starts_with("INPUT-FW-");
    if is_native {
        if request.resume.len() > 1
            || request.resume.first().is_some_and(|entry| {
                entry
                    .interrupt_id
                    .strip_prefix("INPUT-FW-")
                    .unwrap_or(&entry.interrupt_id)
                    != native_id
            })
        {
            return chat_error_response(ChatError::UserInputAnswerInvalid(
                "response identity must match the requested native interaction".into(),
            ));
        }
        let payload = match request.resume.first() {
            Some(entry) => entry.payload.clone(),
            None => match &answers {
                Some(answers) => json!({"answers":answers}),
                None => {
                    return chat_error_response(ChatError::UserInputAnswerInvalid(
                        "native response payload is required".into(),
                    ));
                }
            },
        };
        let entry = crate::application::chat::ChatResumeEntry {
            interrupt_id: request_code.clone(),
            status: "resolved".into(),
            payload,
        };
        if let Err(error) = state
            .agent_framework
            .validate_native_resume(&session_code, std::slice::from_ref(&entry))
            .await
        {
            return chat_error_response(ChatError::UserInputAnswerInvalid(error.to_string()));
        }
        // Existing ask-user rows keep their status and human-readable answer history.
        if let Some(answers) = answers {
            match state
                .chats
                .answer_user_input(&session_code, &request_code, answers.clone())
            {
                Ok(command) => {
                    return stream_message_command(state, auth, session_code, command).await;
                }
                Err(ChatError::UserInputRequestNotFound) => {}
                Err(ChatError::UserInputAlreadyAnswered) => {
                    let previous = match state.chats.list_user_input_requests(&session_code) {
                        Ok(requests) => requests
                            .into_iter()
                            .find(|request| request.request_code == request_code)
                            .and_then(|request| request.answers),
                        Err(error) => return chat_error_response(error),
                    };
                    if !previous.as_ref().is_some_and(|saved| {
                        saved.len() == answers.len()
                            && answers.iter().all(|answer| saved.contains(answer))
                    }) {
                        return chat_error_response(ChatError::UserInputAlreadyAnswered);
                    }
                    // Only an identical product audit can retry the native answer CAS.
                }
                Err(error) => return chat_error_response(error),
            }
        }
        let history = match session.last_run_code.as_deref() {
            Some(run) => match state.agent_runs.get_run(run) {
                Ok(record) => Some(record),
                Err(_) => return chat_error_response(ChatError::StorageFailure),
            },
            None => None,
        };
        let command = match state.chats.native_resume_command(
            &session_code,
            entry,
            history.as_ref().map(|record| record.model_code.clone()),
            history.and_then(|record| record.reasoning_effort),
        ) {
            Ok(command) => command,
            Err(error) => return chat_error_response(error),
        };
        return stream_message_command(state, auth, session_code, command).await;
    }
    let Some(answers) = answers else {
        return chat_error_response(ChatError::UserInputAnswerInvalid(
            "legacy response payload must contain answers".into(),
        ));
    };
    let command = match state
        .chats
        .answer_user_input(&session_code, &request_code, answers)
    {
        Ok(command) => command,
        Err(error) => return chat_error_response(error),
    };
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
