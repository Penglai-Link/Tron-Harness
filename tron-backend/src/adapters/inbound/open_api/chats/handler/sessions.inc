pub async fn list_session_groups(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Query(query): Query<OpenApiChatSessionQuery>,
) -> Response {
    let Some(workspace_code) = normalized_required(query.workspace_code.clone()) else {
        return chat_error_response(ChatError::WorkspaceRequired);
    };

    match state.chats.list_session_groups(ChatSessionQuery {
        workspace_code: Some(workspace_code),
    }) {
        Ok(groups) => Json(ApiResponse::success(
            groups
                .into_iter()
                .map(|group| OpenApiChatSessionGroupResponse {
                    task_code: group.task_code,
                    task_title: group.task_title,
                    sessions: group.sessions.into_iter().map(Into::into).collect(),
                    pending_user_input_count: group.pending_user_input_count,
                    pending_user_input_session_codes: group.pending_user_input_session_codes,
                })
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn create_session(
    State(state): State<AppState>,
    auth: RequestAuthContext,
    Json(request): Json<OpenApiCreateChatSessionRequest>,
) -> Response {
    if normalized_required(request.workspace_code.clone()).is_none() {
        return chat_error_response(ChatError::WorkspaceRequired);
    };

    match state.chats.create_session(
        crate::application::chat::CreateChatSessionCommand {
            workspace_code: request.workspace_code,
            title: request.title,
            app_code: request.app_code,
            task_code: request.task_code,
            source_type: request.source_type,
            source_code: None,
            agent_code: request.agent_code,
        },
        Some(auth.user().user_code.clone()),
    ) {
        Ok(session) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(OpenApiChatSessionResponse::from(
                session,
            ))),
        )
            .into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn delete_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
) -> Response {
    match state.chats.delete_session(&session_code) {
        Ok(session) => Json(ApiResponse::success(OpenApiChatSessionResponse::from(
            session,
        )))
        .into_response(),
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
        Ok(messages) => Json(ApiResponse::success(
            messages
                .into_iter()
                .map(OpenApiChatMessageResponse::from)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => chat_error_response(error),
    }
}

pub async fn patch_session(
    State(state): State<AppState>,
    _auth: RequestAuthContext,
    Path(session_code): Path<String>,
    Json(request): Json<OpenApiUpdateChatSessionRequest>,
) -> Response {
    match state.chats.update_session(
        &session_code,
        crate::application::chat::UpdateChatSessionCommand {
            title: request.title,
        },
    ) {
        Ok(session) => Json(ApiResponse::success(OpenApiChatSessionResponse::from(
            session,
        )))
        .into_response(),
        Err(error) => chat_error_response(error),
    }
}
