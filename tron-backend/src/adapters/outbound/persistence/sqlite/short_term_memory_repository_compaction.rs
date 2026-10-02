use rusqlite::TransactionBehavior;

use crate::domain::short_term_memory::{
    ShortTermMemoryCompaction, ShortTermMemoryError, ShortTermMemoryEvent, ShortTermMemoryMessage,
    ShortTermMemoryPart,
};

use super::super::{
    short_term_memory_repository_append::{
        assign_event_storage_fields, assign_message_storage_fields,
    },
    short_term_memory_rows::{
        insert_context_epoch, insert_event, insert_message, insert_part, storage_error,
    },
};
use super::{repository_context, repository_runs};

pub(super) fn persist_compaction(
    connection: &mut rusqlite::Connection,
    compaction: &ShortTermMemoryCompaction,
    message: &ShortTermMemoryMessage,
    parts: &[ShortTermMemoryPart],
    event: Option<&ShortTermMemoryEvent>,
    enforce_idle: bool,
    expected: Option<&eliteforge_agent_memory::MemoryRevision>,
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    if let Some(expected) = expected {
        if message.session_code != expected.scope.session()
            || message.workspace_code != expected.scope.namespace()
            || compaction.session_code != message.session_code
            || compaction.checkpoint_message_code != message.message_code
            || parts.iter().any(|part| {
                part.session_code != message.session_code
                    || part.workspace_code != message.workspace_code
                    || part.message_code != message.message_code
            })
            || event.is_some_and(|event| {
                event.session_code != message.session_code
                    || event.workspace_code != message.workspace_code
            })
        {
            return Err(ShortTermMemoryError::CompactionConflict);
        }

        let workspace: String = tx
            .query_row(
                "SELECT workspace_code FROM chat_sessions WHERE session_code=?1",
                [&message.session_code],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let source_revision:i64=tx.query_row("SELECT COALESCE(MAX(seq),0) FROM agent_transcript_messages WHERE session_code=?1 AND status='completed' AND message_type!='compaction'",[&message.session_code],|row|row.get(0)).map_err(storage_error)?;
        let generation = repository_context::latest_compaction(&tx, &message.session_code)?
            .map(|c| c.checkpoint_message_code)
            .unwrap_or_default();
        let current = eliteforge_agent_memory::MemoryRevision {
            scope: eliteforge_agent_memory::MemoryScope::new(workspace, &message.session_code)
                .map_err(|_| ShortTermMemoryError::StorageFailure)?,
            generation,
            revision: source_revision.max(0) as u64,
        };
        expected
            .ensure_current(&current)
            .map_err(|_| ShortTermMemoryError::CompactionConflict)?;
    }
    if enforce_idle
        && repository_runs::active_run_exists_in_transaction(&tx, &message.session_code)?
    {
        return Err(ShortTermMemoryError::CompactionConflict);
    }
    let mut message = message.clone();
    let mut parts = parts.to_vec();
    assign_message_storage_fields(&tx, &mut message, &mut parts)?;
    insert_message(&tx, &message)?;
    for part in &parts {
        insert_part(&tx, part)?;
    }
    insert_context_epoch(&tx, compaction)?;
    if let Some(event) = event {
        let mut event = event.clone();
        assign_event_storage_fields(&tx, &mut event)?;
        insert_event(&tx, &event)?;
    }
    tx.commit().map_err(storage_error)
}
