use rusqlite::Connection;

use crate::domain::short_term_memory::ShortTermMemoryError;

use super::super::super::short_term_memory_rows::{column_exists, storage_error, table_exists};

pub(super) fn repair_transcript_backfilled_ownership(
    connection: &Connection,
) -> Result<(), ShortTermMemoryError> {
    repair_workspace_from_chat_sessions(connection, "agent_transcript_inputs")?;
    repair_workspace_from_chat_sessions(connection, "agent_transcript_messages")?;
    repair_parts_from_parent_messages(connection)?;
    repair_events_from_session_parents(connection)?;
    repair_workspace_from_chat_sessions(connection, "agent_session_todos")?;
    repair_workspace_from_chat_sessions(connection, "agent_session_goals")?;
    repair_goal_children_from_parent_goals(connection)?;
    delete_unowned_session_rows(connection)?;
    Ok(())
}

fn repair_workspace_from_chat_sessions(
    connection: &Connection,
    table: &str,
) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, table)?
        || !column_exists(connection, table, "session_code")?
        || !column_exists(connection, table, "workspace_code")?
        || !table_exists(connection, "chat_sessions")?
    {
        return Ok(());
    }
    connection
        .execute(
            &format!(
                "UPDATE {table}
                 SET workspace_code = (
                     SELECT chat_sessions.workspace_code
                     FROM chat_sessions
                     WHERE chat_sessions.session_code = {table}.session_code
                 )
                 WHERE workspace_code = ''
                   AND session_code <> ''
                   AND EXISTS (
                       SELECT 1
                       FROM chat_sessions
                       WHERE chat_sessions.session_code = {table}.session_code
                   )"
            ),
            [],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn repair_parts_from_parent_messages(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, "agent_transcript_parts")?
        || !table_exists(connection, "agent_transcript_messages")?
        || !column_exists(connection, "agent_transcript_parts", "session_code")?
        || !column_exists(connection, "agent_transcript_parts", "workspace_code")?
    {
        return Ok(());
    }
    connection
        .execute_batch(
            "UPDATE agent_transcript_parts
             SET session_code = (
                 SELECT agent_transcript_messages.session_code
                 FROM agent_transcript_messages
                 WHERE agent_transcript_messages.message_code = agent_transcript_parts.message_code
             )
             WHERE session_code = ''
               AND EXISTS (
                   SELECT 1
                   FROM agent_transcript_messages
                   WHERE agent_transcript_messages.message_code = agent_transcript_parts.message_code
               );
             UPDATE agent_transcript_parts
             SET workspace_code = (
                 SELECT agent_transcript_messages.workspace_code
                 FROM agent_transcript_messages
                 WHERE agent_transcript_messages.message_code = agent_transcript_parts.message_code
             )
             WHERE workspace_code = ''
               AND EXISTS (
                   SELECT 1
                   FROM agent_transcript_messages
                   WHERE agent_transcript_messages.message_code = agent_transcript_parts.message_code
               );",
        )
        .map_err(storage_error)?;
    Ok(())
}

fn repair_events_from_session_parents(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, "agent_transcript_events")?
        || !column_exists(connection, "agent_transcript_events", "workspace_code")?
    {
        return Ok(());
    }
    if table_exists(connection, "agent_transcript_inputs")? {
        connection
            .execute(
                "UPDATE agent_transcript_events
                 SET workspace_code = (
                     SELECT agent_transcript_inputs.workspace_code
                     FROM agent_transcript_inputs
                     WHERE agent_transcript_inputs.input_code = agent_transcript_events.input_code
                 )
                 WHERE workspace_code = ''
                   AND input_code IS NOT NULL
                   AND EXISTS (
                       SELECT 1
                       FROM agent_transcript_inputs
                       WHERE agent_transcript_inputs.input_code = agent_transcript_events.input_code
                   )",
                [],
            )
            .map_err(storage_error)?;
    }
    if table_exists(connection, "agent_transcript_messages")? {
        connection
            .execute(
                "UPDATE agent_transcript_events
                 SET workspace_code = (
                     SELECT agent_transcript_messages.workspace_code
                     FROM agent_transcript_messages
                     WHERE agent_transcript_messages.message_code = agent_transcript_events.message_code
                 )
                 WHERE workspace_code = ''
                   AND message_code IS NOT NULL
                   AND EXISTS (
                       SELECT 1
                       FROM agent_transcript_messages
                       WHERE agent_transcript_messages.message_code = agent_transcript_events.message_code
                   )",
                [],
            )
            .map_err(storage_error)?;
    }
    repair_workspace_from_chat_sessions(connection, "agent_transcript_events")
}

fn repair_goal_children_from_parent_goals(
    connection: &Connection,
) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, "agent_session_goals")? {
        return Ok(());
    }
    if table_exists(connection, "agent_session_goal_criteria")?
        && column_exists(connection, "agent_session_goal_criteria", "session_code")?
    {
        connection
            .execute(
                "UPDATE agent_session_goal_criteria
                 SET session_code = (
                     SELECT agent_session_goals.session_code
                     FROM agent_session_goals
                     WHERE agent_session_goals.goal_code = agent_session_goal_criteria.goal_code
                 )
                 WHERE session_code = ''
                   AND EXISTS (
                       SELECT 1
                       FROM agent_session_goals
                       WHERE agent_session_goals.goal_code = agent_session_goal_criteria.goal_code
                   )",
                [],
            )
            .map_err(storage_error)?;
    }
    if table_exists(connection, "agent_session_goal_events")? {
        if column_exists(connection, "agent_session_goal_events", "session_code")? {
            connection
                .execute(
                    "UPDATE agent_session_goal_events
                     SET session_code = (
                         SELECT agent_session_goals.session_code
                         FROM agent_session_goals
                         WHERE agent_session_goals.goal_code = agent_session_goal_events.goal_code
                     )
                     WHERE session_code = ''
                       AND EXISTS (
                           SELECT 1
                           FROM agent_session_goals
                           WHERE agent_session_goals.goal_code = agent_session_goal_events.goal_code
                       )",
                    [],
                )
                .map_err(storage_error)?;
        }
        if column_exists(connection, "agent_session_goal_events", "workspace_code")? {
            connection
                .execute(
                    "UPDATE agent_session_goal_events
                     SET workspace_code = (
                         SELECT agent_session_goals.workspace_code
                         FROM agent_session_goals
                         WHERE agent_session_goals.goal_code = agent_session_goal_events.goal_code
                     )
                     WHERE workspace_code = ''
                       AND EXISTS (
                           SELECT 1
                           FROM agent_session_goals
                           WHERE agent_session_goals.goal_code = agent_session_goal_events.goal_code
                       )",
                    [],
                )
                .map_err(storage_error)?;
        }
    }
    Ok(())
}

fn delete_unowned_session_rows(connection: &Connection) -> Result<(), ShortTermMemoryError> {
    if !table_exists(connection, "chat_sessions")? {
        return Ok(());
    }
    for table in [
        "agent_transcript_inputs",
        "agent_transcript_events",
        "agent_transcript_messages",
        "agent_transcript_parts",
        "agent_transcript_context_epochs",
        "agent_session_todos",
        "agent_session_family_members",
        "agent_session_family_snapshots",
        "agent_session_goals",
        "agent_session_goal_criteria",
        "agent_session_goal_events",
        "agent_session_run_leases",
    ] {
        if table_exists(connection, table)? && column_exists(connection, table, "session_code")? {
            connection
                .execute(
                    &format!(
                        "DELETE FROM {table}
                         WHERE session_code = ''
                            OR NOT EXISTS (
                                SELECT 1
                                FROM chat_sessions
                                WHERE chat_sessions.session_code = {table}.session_code
                            )"
                    ),
                    [],
                )
                .map_err(storage_error)?;
        }
    }
    Ok(())
}
