use rusqlite::{Connection, OptionalExtension, Row, params};
use std::{fs, path::Path};

use crate::{
    application::chat::ChatRepository,
    domain::chat::{
        ChatError, ChatMessage, ChatMessageChannel, ChatMessageRole, ChatSession,
        ChatSessionSourceType, ChatSessionStatus, ChatUserInputAnswer, ChatUserInputRequest,
        ChatUserInputRequestStatus,
    },
    shared::time::{timestamp_from_millis, timestamp_to_millis},
};

use super::chat_message_attachments::{attachments_from_storage, attachments_to_storage};
use super::migrations::{
    CHAT_MANAGEMENT_MIGRATION, CHAT_MESSAGE_ATTACHMENTS_MIGRATION,
    CHAT_MESSAGE_TOKEN_USAGE_MIGRATION, CHAT_SESSION_AGENT_CODE_MIGRATION,
    CHAT_SESSION_LEGACY_SOURCE_MIGRATION, CHAT_SESSION_SOURCE_MIGRATION,
    CHAT_SESSION_SYSTEM_AGENT_SESSION_ID_MIGRATION, CHAT_SESSION_VISIBILITY_MIGRATION,
    CHAT_USER_INPUT_REQUESTS_MIGRATION, CONVERSATION_RUNTIME_SESSIONS_MIGRATION,
};

pub struct SqliteChatRepository {
    connection: Connection,
}

impl SqliteChatRepository {
    pub fn connect(database_url: &str) -> Result<Self, ChatError> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(storage_error)?;
        let repository = Self { connection };
        repository
            .connection
            .execute_batch(CHAT_MANAGEMENT_MIGRATION)
            .map_err(storage_error)?;
        if !column_exists(&repository.connection, "chat_sessions", "agent_code")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_AGENT_CODE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(
            &repository.connection,
            "chat_sessions",
            "system_agent_session_id",
        )? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_SYSTEM_AGENT_SESSION_ID_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_sessions", "source")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_LEGACY_SOURCE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_sessions", "visibility")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_VISIBILITY_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_messages", "input_tokens")? {
            repository
                .connection
                .execute_batch(CHAT_MESSAGE_TOKEN_USAGE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_messages", "attachments_json")? {
            repository
                .connection
                .execute_batch(CHAT_MESSAGE_ATTACHMENTS_MIGRATION)
                .map_err(storage_error)?;
        }
        if !table_exists(&repository.connection, "conversation_runtime_sessions")? {
            repository
                .connection
                .execute_batch(CONVERSATION_RUNTIME_SESSIONS_MIGRATION)
                .map_err(storage_error)?;
        }
        if !column_exists(&repository.connection, "chat_sessions", "source_type")? {
            repository
                .connection
                .execute_batch(CHAT_SESSION_SOURCE_MIGRATION)
                .map_err(storage_error)?;
        }
        if !table_exists(&repository.connection, "chat_user_input_requests")? {
            repository
                .connection
                .execute_batch(CHAT_USER_INPUT_REQUESTS_MIGRATION)
                .map_err(storage_error)?;
        }
        Ok(repository)
    }
}

impl ChatRepository for SqliteChatRepository {
    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError> {
        self.connection
            .execute(
                "INSERT INTO chat_sessions (id, session_code, workspace_code, user_code, app_code, task_code, source_type, source_code, agent_code, system_agent_session_id, title, status, last_run_code, last_run_status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    session.id,
                    session.session_code,
                    session.workspace_code,
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
                "SELECT id, session_code, workspace_code, user_code, app_code, task_code, agent_code, title, status, last_run_code, last_run_status, created_at, updated_at
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
                "SELECT id, session_code, workspace_code, user_code, app_code, task_code, agent_code, title, status, last_run_code, last_run_status, created_at, updated_at
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
