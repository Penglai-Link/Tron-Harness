fn row_to_session(row: &Row<'_>) -> rusqlite::Result<ChatSession> {
    let status: String = row.get(8)?;
    let source_type: String = row.get(14)?;
    Ok(ChatSession {
        id: row.get(0)?,
        session_code: row.get(1)?,
        workspace_code: row.get(2)?,
        user_code: row.get(3)?,
        app_code: row.get(4)?,
        task_code: row.get(5)?,
        source_type: session_source_type_from_storage(&source_type).map_err(chat_mapping_error)?,
        source_code: row.get(15)?,
        agent_code: row.get(6)?,
        system_agent_session_id: row.get(13)?,
        title: row.get(7)?,
        status: session_status_from_storage(&status).map_err(chat_mapping_error)?,
        last_run_code: row.get(9)?,
        last_run_status: row.get(10)?,
        created_at: timestamp_from_millis(row.get(11)?),
        updated_at: timestamp_from_millis(row.get(12)?),
    })
}

fn row_to_message(row: &Row<'_>) -> rusqlite::Result<ChatMessage> {
    let role: String = row.get(4)?;
    let channel: String = row.get(5)?;
    let attachments_json: Option<String> = row.get(8)?;
    Ok(ChatMessage {
        id: row.get(0)?,
        message_code: row.get(1)?,
        session_code: row.get(2)?,
        run_code: row.get(3)?,
        role: message_role_from_storage(&role).map_err(chat_mapping_error)?,
        channel: message_channel_from_storage(&channel).map_err(chat_mapping_error)?,
        actor: row.get(6)?,
        content: row.get(7)?,
        attachments: attachments_from_storage(attachments_json.as_deref())
            .map_err(chat_mapping_error)?,
        input_tokens: row.get(9)?,
        output_tokens: row.get(10)?,
        total_tokens: row.get(11)?,
        created_at: timestamp_from_millis(row.get(12)?),
    })
}

fn row_to_user_input_request(row: &Row<'_>) -> rusqlite::Result<ChatUserInputRequest> {
    let questions_json: String = row.get(5)?;
    let answers_json: Option<String> = row.get(6)?;
    let reasoning_effort: Option<String> = row.get(8)?;
    let status: String = row.get(9)?;
    Ok(ChatUserInputRequest {
        request_code: row.get(0)?,
        session_code: row.get(1)?,
        run_code: row.get(2)?,
        assistant_message_code: row.get(3)?,
        intro: row.get(4)?,
        questions: serde_json::from_str(&questions_json).map_err(storage_mapping_error)?,
        answers: answers_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(storage_mapping_error)?,
        model_code: row.get(7)?,
        reasoning_effort: reasoning_effort
            .as_deref()
            .and_then(crate::domain::reasoning::ReasoningEffort::from_storage_value),
        status: user_input_status_from_storage(&status).map_err(chat_mapping_error)?,
        requested_at: timestamp_from_millis(row.get(10)?),
        answered_at: row.get::<_, Option<i64>>(11)?.map(timestamp_from_millis),
    })
}

fn user_input_status_to_storage(status: ChatUserInputRequestStatus) -> &'static str {
    match status {
        ChatUserInputRequestStatus::Pending => "PENDING",
        ChatUserInputRequestStatus::Answered => "ANSWERED",
    }
}

fn user_input_status_from_storage(value: &str) -> Result<ChatUserInputRequestStatus, ChatError> {
    match value {
        "PENDING" => Ok(ChatUserInputRequestStatus::Pending),
        "ANSWERED" => Ok(ChatUserInputRequestStatus::Answered),
        _ => Err(ChatError::StorageFailure),
    }
}

fn storage_mapping_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, ChatError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(storage_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(columns.iter().any(|value| value == column_name))
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool, ChatError> {
    let table = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(storage_error)?;
    Ok(table.is_some())
}

fn delete_short_term_memory_rows_for_session(
    connection: &Connection,
    session_code: &str,
) -> Result<(), ChatError> {
    if table_exists(connection, "agent_session_families")? {
        connection
            .execute(
                "UPDATE agent_session_families
                 SET active_session_code = CASE WHEN active_session_code = ?1 THEN NULL ELSE active_session_code END,
                     root_session_code = CASE WHEN root_session_code = ?1 THEN NULL ELSE root_session_code END
                 WHERE active_session_code = ?1
                    OR root_session_code = ?1",
                [session_code],
            )
            .map_err(storage_error)?;
        if table_exists(connection, "agent_session_family_snapshots")? {
            connection
                .execute(
                    "UPDATE agent_session_families
                     SET latest_snapshot_code = NULL
                     WHERE latest_snapshot_code IN (
                         SELECT snapshot_code
                         FROM agent_session_family_snapshots
                         WHERE session_code = ?1
                     )",
                    [session_code],
                )
                .map_err(storage_error)?;
        }
    }

    for table in [
        "agent_session_goal_events",
        "agent_session_goal_criteria",
        "agent_session_goals",
        "agent_session_run_leases",
        "agent_session_todos",
        "agent_session_family_snapshots",
        "agent_session_family_members",
        "agent_transcript_context_epochs",
        "agent_transcript_parts",
        "agent_transcript_messages",
        "agent_transcript_events",
        "agent_transcript_inputs",
    ] {
        delete_session_rows_if_table_exists(connection, table, session_code)?;
    }
    Ok(())
}

fn delete_session_rows_if_table_exists(
    connection: &Connection,
    table: &str,
    session_code: &str,
) -> Result<(), ChatError> {
    if !table_exists(connection, table)? {
        return Ok(());
    }
    connection
        .execute(
            &format!("DELETE FROM {table} WHERE session_code = ?1"),
            [session_code],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn next_id(connection: &Connection, table: &str) -> Result<i64, ChatError> {
    let max_id = connection
        .query_row(&format!("SELECT max(id) FROM {table}"), [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .map_err(storage_error)?
        .unwrap_or(735_200_000_000_000_000);
    Ok(max_id + 1)
}

fn session_status_to_storage(status: ChatSessionStatus) -> &'static str {
    match status {
        ChatSessionStatus::Active => "ACTIVE",
        ChatSessionStatus::Archived => "ARCHIVED",
    }
}

fn session_status_from_storage(value: &str) -> Result<ChatSessionStatus, ChatError> {
    match value {
        "ACTIVE" => Ok(ChatSessionStatus::Active),
        "ARCHIVED" => Ok(ChatSessionStatus::Archived),
        _ => Err(ChatError::StorageFailure),
    }
}

fn session_source_type_to_storage(source_type: ChatSessionSourceType) -> &'static str {
    match source_type {
        ChatSessionSourceType::Manual => "MANUAL",
        ChatSessionSourceType::Task => "TASK",
        ChatSessionSourceType::Automation => "AUTOMATION",
        ChatSessionSourceType::Terminal => "TERMINAL",
    }
}

fn session_source_type_from_storage(value: &str) -> Result<ChatSessionSourceType, ChatError> {
    match value {
        "MANUAL" => Ok(ChatSessionSourceType::Manual),
        "TASK" => Ok(ChatSessionSourceType::Task),
        "AUTOMATION" => Ok(ChatSessionSourceType::Automation),
        "TERMINAL" => Ok(ChatSessionSourceType::Terminal),
        _ => Err(ChatError::StorageFailure),
    }
}

fn message_role_to_storage(role: ChatMessageRole) -> &'static str {
    match role {
        ChatMessageRole::User => "USER",
        ChatMessageRole::Assistant => "ASSISTANT",
    }
}

fn message_role_from_storage(value: &str) -> Result<ChatMessageRole, ChatError> {
    match value {
        "USER" => Ok(ChatMessageRole::User),
        "ASSISTANT" => Ok(ChatMessageRole::Assistant),
        _ => Err(ChatError::StorageFailure),
    }
}

fn message_channel_to_storage(channel: ChatMessageChannel) -> &'static str {
    match channel {
        ChatMessageChannel::MainAgent => "MAIN_AGENT",
        ChatMessageChannel::TaskAgent => "TASK_AGENT",
    }
}

fn message_channel_from_storage(value: &str) -> Result<ChatMessageChannel, ChatError> {
    match value {
        "MAIN_AGENT" => Ok(ChatMessageChannel::MainAgent),
        "TASK_AGENT" => Ok(ChatMessageChannel::TaskAgent),
        _ => Err(ChatError::StorageFailure),
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), ChatError> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|_| ChatError::StorageFailure)?;
        }
    }
    Ok(())
}

fn storage_error(error: rusqlite::Error) -> ChatError {
    eprintln!("chat sqlite error: {error}");
    ChatError::StorageFailure
}

fn chat_mapping_error(error: ChatError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
