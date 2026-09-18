impl ChatRepository for SqliteChatRepository {
    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError> {
        self.connection
            .execute(
                "INSERT INTO chat_sessions (id, session_code, workspace_code, project_code, user_code, app_code, task_code, source_type, source_code, agent_code, system_agent_session_id, title, status, last_run_code, last_run_status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    session.id,
                    session.session_code,
                    session.workspace_code,
                    session.project_code,
                    session.user_code,
                    session.app_code,
                    session.task_code,
                    session_source_type_to_storage(session.source_type),
                    session.source_code,
                    session.agent_code,
                    session.system_agent_session_id,
                    session.title,
                    session_status_to_storage(session.status),
                    session.last_run_code,
                    session.last_run_status,
                    timestamp_to_millis(&session.created_at),
                    timestamp_to_millis(&session.updated_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn delete_session(&mut self, session_code: &str) -> Result<(), ChatError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        delete_short_term_memory_rows_for_session(&tx, session_code)?;
        tx.execute(
            "DELETE FROM chat_user_input_requests WHERE session_code = ?1",
            [session_code],
        )
        .map_err(storage_error)?;
        tx.execute(
            "DELETE FROM chat_messages WHERE session_code = ?1",
            [session_code],
        )
        .map_err(storage_error)?;
        let changed = tx
            .execute(
                "DELETE FROM chat_sessions WHERE session_code = ?1",
                [session_code],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        tx.commit().map_err(storage_error)?;
        Ok(())
    }

    fn list_sessions(&mut self) -> Result<Vec<ChatSession>, ChatError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, session_code, workspace_code, project_code, user_code, app_code, task_code, agent_code, title, status, last_run_code, last_run_status, created_at, updated_at
                        , system_agent_session_id, source_type, source_code
                 FROM chat_sessions
                 WHERE COALESCE(visibility, 'VISIBLE') != 'HIDDEN'
                 ORDER BY updated_at DESC, id DESC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([], row_to_session)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn get_session(&mut self, session_code: &str) -> Result<ChatSession, ChatError> {
        self.connection
            .query_row(
                "SELECT id, session_code, workspace_code, project_code, user_code, app_code, task_code, agent_code, title, status, last_run_code, last_run_status, created_at, updated_at
                        , system_agent_session_id, source_type, source_code
                 FROM chat_sessions
                 WHERE session_code = ?1",
                [session_code],
                row_to_session,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(ChatError::SessionNotFound)
    }

    fn update_session_status(
        &mut self,
        session_code: &str,
        status: ChatSessionStatus,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        let changed = self
            .connection
            .execute(
                "UPDATE chat_sessions
                 SET status = ?2, updated_at = ?3
                 WHERE session_code = ?1",
                params![
                    session_code,
                    session_status_to_storage(status),
                    timestamp_to_millis(updated_at),
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        Ok(())
    }

    fn update_session_title(
        &mut self,
        session_code: &str,
        title: &str,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        let changed = self
            .connection
            .execute(
                "UPDATE chat_sessions
                 SET title = ?2, updated_at = ?3
                 WHERE session_code = ?1",
                params![session_code, title, timestamp_to_millis(updated_at)],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        Ok(())
    }

    fn update_session_run(
        &mut self,
        session_code: &str,
        run_code: &str,
        run_status: &str,
        agent_code: Option<&str>,
        system_agent_session_id: Option<&str>,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        let changed = self
            .connection
            .execute(
                "UPDATE chat_sessions
                 SET last_run_code = ?2, last_run_status = ?3, agent_code = COALESCE(?4, agent_code), system_agent_session_id = COALESCE(?5, system_agent_session_id), updated_at = ?6
                 WHERE session_code = ?1",
                params![
                    session_code,
                    run_code,
                    run_status,
                    agent_code,
                    system_agent_session_id,
                    timestamp_to_millis(updated_at),
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            return Err(ChatError::SessionNotFound);
        }
        Ok(())
    }

    fn runtime_session_id(
        &mut self,
        conversation_code: &str,
        runtime_adapter_key: &str,
        agent_code: &str,
    ) -> Result<Option<String>, ChatError> {
        self.connection
            .query_row(
                "SELECT runtime_session_id
                 FROM conversation_runtime_sessions
                 WHERE conversation_code = ?1
                   AND runtime_adapter_key = ?2
                   AND agent_code = ?3
                   AND status = 'ACTIVE'",
                params![conversation_code, runtime_adapter_key, agent_code],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)
    }

    fn upsert_runtime_session(
        &mut self,
        conversation_code: &str,
        runtime_adapter_key: &str,
        agent_code: &str,
        runtime_session_id: &str,
        last_run_code: Option<&str>,
        updated_at: &str,
    ) -> Result<(), ChatError> {
        let next_id = next_id(&self.connection, "conversation_runtime_sessions")?;
        let updated_at_ms = timestamp_to_millis(updated_at);
        self.connection
            .execute(
                "INSERT INTO conversation_runtime_sessions (
                    id,
                    conversation_code,
                    runtime_adapter_key,
                    agent_code,
                    runtime_session_id,
                    status,
                    last_run_code,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'ACTIVE', ?6, ?7, ?8)
                 ON CONFLICT(conversation_code, runtime_adapter_key, agent_code)
                 DO UPDATE SET
                    runtime_session_id = excluded.runtime_session_id,
                    status = 'ACTIVE',
                    last_run_code = excluded.last_run_code,
                    updated_at = excluded.updated_at",
                params![
                    next_id,
                    conversation_code,
                    runtime_adapter_key,
                    agent_code,
                    runtime_session_id,
                    last_run_code,
                    updated_at_ms,
                    updated_at_ms,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn create_message(&mut self, message: &ChatMessage) -> Result<(), ChatError> {
        self.connection
            .execute(
                "INSERT INTO chat_messages (id, message_code, session_code, run_code, role, channel, actor, content, attachments_json, input_tokens, output_tokens, total_tokens, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    message.id,
                    message.message_code,
                    message.session_code,
                    message.run_code,
                    message_role_to_storage(message.role),
                    message_channel_to_storage(message.channel),
                    message.actor,
                    message.content,
                    attachments_to_storage(&message.attachments)?,
                    message.input_tokens,
                    message.output_tokens,
                    message.total_tokens,
                    timestamp_to_millis(&message.created_at),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_messages(&mut self, session_code: &str) -> Result<Vec<ChatMessage>, ChatError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, message_code, session_code, run_code, role, channel, actor, content, attachments_json, input_tokens, output_tokens, total_tokens, created_at
                 FROM chat_messages
                 WHERE session_code = ?1
                 ORDER BY created_at ASC, id ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([session_code], row_to_message)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn list_user_input_requests(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<ChatUserInputRequest>, ChatError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT request_code, session_code, run_code, assistant_message_code, intro,
                        questions_json, answers_json, model_code, reasoning_effort, status,
                        requested_at, answered_at
                 FROM chat_user_input_requests
                 WHERE session_code = ?1
                 ORDER BY requested_at ASC, id ASC",
            )
            .map_err(storage_error)?;
        statement
            .query_map([session_code], row_to_user_input_request)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn create_user_input_request(
        &mut self,
        request: &ChatUserInputRequest,
    ) -> Result<(), ChatError> {
        let id = next_id(&self.connection, "chat_user_input_requests")?;
        self.connection
            .execute(
                "INSERT INTO chat_user_input_requests (
                    id, request_code, session_code, run_code, assistant_message_code, intro,
                    questions_json, answers_json, model_code, reasoning_effort, status,
                    requested_at, answered_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    id,
                    request.request_code,
                    request.session_code,
                    request.run_code,
                    request.assistant_message_code,
                    request.intro,
                    serde_json::to_string(&request.questions)
                        .map_err(|_| ChatError::StorageFailure)?,
                    request
                        .answers
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(|_| ChatError::StorageFailure)?,
                    request.model_code,
                    request
                        .reasoning_effort
                        .map(|value| value.storage_value().to_string()),
                    user_input_status_to_storage(request.status),
                    timestamp_to_millis(&request.requested_at),
                    request.answered_at.as_deref().map(timestamp_to_millis),
                ],
            )
            .map_err(|error| {
                if matches!(
                    &error,
                    rusqlite::Error::SqliteFailure(
                        failure,
                        _
                    ) if failure.code == rusqlite::ErrorCode::ConstraintViolation
                ) {
                    ChatError::UserInputPending
                } else {
                    storage_error(error)
                }
            })?;
        Ok(())
    }

    fn answer_user_input_request(
        &mut self,
        session_code: &str,
        request_code: &str,
        answers: &[ChatUserInputAnswer],
        answered_at: &str,
    ) -> Result<ChatUserInputRequest, ChatError> {
        let answers_json = serde_json::to_string(answers).map_err(|_| ChatError::StorageFailure)?;
        let changed = self
            .connection
            .execute(
                "UPDATE chat_user_input_requests
                 SET answers_json = ?3, status = 'ANSWERED', answered_at = ?4
                 WHERE session_code = ?1 AND request_code = ?2 AND status = 'PENDING'",
                params![
                    session_code,
                    request_code,
                    answers_json,
                    timestamp_to_millis(answered_at),
                ],
            )
            .map_err(storage_error)?;
        if changed == 0 {
            let request = self
                .list_user_input_requests(session_code)?
                .into_iter()
                .find(|item| item.request_code == request_code);
            return Err(match request {
                Some(_) => ChatError::UserInputAlreadyAnswered,
                None => ChatError::UserInputRequestNotFound,
            });
        }
        self.list_user_input_requests(session_code)?
            .into_iter()
            .find(|item| item.request_code == request_code)
            .ok_or(ChatError::UserInputRequestNotFound)
    }

}
