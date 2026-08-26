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
    let answers = request
        .answers
        .or_else(|| {
            request
                .resume
                .first()
                .and_then(|entry| entry.payload.get("answers"))
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        });
    let Some(answers) = answers else {
        return chat_error_response(ChatError::UserInputAnswerInvalid(
            "resume payload must contain answers".into(),
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
