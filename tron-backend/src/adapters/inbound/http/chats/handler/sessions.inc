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
    let session=match state.chats.get_session(&session_code) {
        Ok(session)=>session,Err(error)=>return chat_error_response(error),
    };
    if let Err(response)=ensure_no_active_run(&state,&session_code) { return response; }
    let answers=request.answers.or_else(||request.resume.first().and_then(|entry|entry.payload.get("answers"))
        .and_then(|value|serde_json::from_value(value.clone()).ok()));
    let native_id=request_code.strip_prefix("INPUT-FW-").unwrap_or(&request_code);
    let is_native=eliteforge_agent_core::RunId::parse_str(native_id).is_ok() || request_code.starts_with("INPUT-FW-");
    if is_native {
        if request.resume.len()>1 || request.resume.first().is_some_and(|entry| {
            entry.interrupt_id.strip_prefix("INPUT-FW-").unwrap_or(&entry.interrupt_id)!=native_id
        }) { return chat_error_response(ChatError::UserInputAnswerInvalid("response identity must match the requested native interaction".into())); }
        let payload=match request.resume.first() {
            Some(entry)=>entry.payload.clone(),
            None=>match &answers { Some(answers)=>json!({"answers":answers}),None=>return chat_error_response(ChatError::UserInputAnswerInvalid("native response payload is required".into())) },
        };
        let entry=crate::application::chat::ChatResumeEntry {interrupt_id:request_code.clone(),status:"resolved".into(),payload};
        if let Err(error)=state.agent_framework.validate_native_resume(&session_code,std::slice::from_ref(&entry)).await {
            return chat_error_response(ChatError::UserInputAnswerInvalid(error.to_string()));
        }
        // Existing ask-user rows keep their status and human-readable answer history.
        if let Some(answers)=answers {
            match state.chats.answer_user_input(&session_code,&request_code,answers.clone()) {
                Ok(command)=>return stream_message_command(state,auth,session_code,command).await,
                Err(ChatError::UserInputRequestNotFound)=>{},
                Err(ChatError::UserInputAlreadyAnswered)=>{
                    let previous=match state.chats.list_user_input_requests(&session_code) {
                        Ok(requests)=>requests.into_iter().find(|request|request.request_code==request_code).and_then(|request|request.answers),
                        Err(error)=>return chat_error_response(error),
                    };
                    if !previous.as_ref().is_some_and(|saved|saved.len()==answers.len() && answers.iter().all(|answer|saved.contains(answer))) {
                        return chat_error_response(ChatError::UserInputAlreadyAnswered);
                    }
                    // Only an identical product audit can retry the native answer CAS.
                },
                Err(error)=>return chat_error_response(error),
            }
        }
        let history=match session.last_run_code.as_deref() {
            Some(run)=>match state.agent_runs.get_run(run) {
                Ok(record)=>Some(record),Err(_)=>return chat_error_response(ChatError::StorageFailure),
            },None=>None,
        };
        let command=match state.chats.native_resume_command(&session_code,entry,
            history.as_ref().map(|record|record.model_code.clone()),history.and_then(|record|record.reasoning_effort)) {
            Ok(command)=>command,Err(error)=>return chat_error_response(error),
        };
        return stream_message_command(state,auth,session_code,command).await;
    }
    let Some(answers)=answers else { return chat_error_response(ChatError::UserInputAnswerInvalid("legacy response payload must contain answers".into())); };
    let command=match state.chats.answer_user_input(&session_code,&request_code,answers) {
        Ok(command)=>command,Err(error)=>return chat_error_response(error),
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
