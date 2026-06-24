use rusqlite::{Connection, params};

use crate::{
    domain::short_term_memory::{
        ShortTermMemoryCompaction, ShortTermMemoryContextEpoch, ShortTermMemoryError,
        ShortTermMemoryEvent, ShortTermMemoryFamily, ShortTermMemoryFamilyMember,
        ShortTermMemoryFamilySnapshot, ShortTermMemoryMessage, ShortTermMemoryPart,
        ShortTermMemoryTodo,
    },
    shared::time::timestamp_to_millis,
};

use super::{
    json_to_storage, next_id, option_json_to_storage, option_timestamp_to_millis, storage_error,
};

fn validate_allowed(value: &str, allowed: &[&str]) -> Result<(), ShortTermMemoryError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ShortTermMemoryError::StorageFailure)
    }
}

fn validate_message(message: &ShortTermMemoryMessage) -> Result<(), ShortTermMemoryError> {
    validate_allowed(
        &message.role,
        &[
            "system",
            "developer",
            "user",
            "assistant",
            "tool",
            "checkpoint",
        ],
    )?;
    validate_allowed(
        &message.message_type,
        &["normal", "tool_result", "compaction"],
    )?;
    validate_allowed(
        &message.status,
        &["pending", "in_progress", "completed", "failed", "cancelled"],
    )
}

fn validate_part(part: &ShortTermMemoryPart) -> Result<(), ShortTermMemoryError> {
    validate_allowed(
        &part.part_type,
        &[
            "text",
            "reasoning",
            "reasoning_summary",
            "tool_call",
            "tool_result",
            "context",
            "file",
            "compaction_summary",
            "compaction_recent",
        ],
    )?;
    validate_allowed(
        &part.status,
        &["pending", "in_progress", "completed", "failed", "cancelled"],
    )?;
    validate_allowed(&part.visibility, &["model", "ui", "internal", "audit"])
}

fn validate_family(family: &ShortTermMemoryFamily) -> Result<(), ShortTermMemoryError> {
    validate_allowed(
        &family.family_kind,
        &[
            "WORKSPACE_CHAT",
            "WORKSPACE_HEARTBEAT",
            "REQUIREMENT",
            "TASK",
            "FORK",
        ],
    )?;
    validate_allowed(
        &family.status,
        &["ACTIVE", "PAUSED", "COMPLETED", "DISABLED", "ARCHIVED"],
    )
}

fn validate_family_snapshot(
    snapshot: &ShortTermMemoryFamilySnapshot,
) -> Result<(), ShortTermMemoryError> {
    validate_allowed(&snapshot.snapshot_kind, &["RUN_HANDOFF"])?;
    validate_allowed(
        &snapshot.status,
        &["PENDING", "IN_PROGRESS", "COMPLETED", "FAILED", "EXPIRED"],
    )
}

pub(crate) fn insert_message(
    connection: &Connection,
    message: &ShortTermMemoryMessage,
) -> Result<(), ShortTermMemoryError> {
    validate_message(message)?;
    connection
        .execute(
            "INSERT INTO agent_transcript_messages (id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, agent_code, model_code, model_provider, model_name, model_protocol, finish_reason, error_json, input_tokens, output_tokens, reasoning_tokens, total_tokens, metadata_json, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
            params![
                message.id,
                message.message_code,
                message.session_code,
                message.workspace_code,
                message.run_code,
                message.seq,
                message.role,
                message.message_type,
                message.status,
                message.agent_code,
                message.model_code,
                message.model_provider,
                message.model_name,
                message.model_protocol,
                message.finish_reason,
                option_json_to_storage(message.error_json.as_ref())?,
                message.input_tokens,
                message.output_tokens,
                message.reasoning_tokens,
                message.total_tokens,
                option_json_to_storage(message.metadata_json.as_ref())?,
                timestamp_to_millis(&message.created_at),
                option_timestamp_to_millis(message.completed_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_part(
    connection: &Connection,
    part: &ShortTermMemoryPart,
) -> Result<(), ShortTermMemoryError> {
    validate_part(part)?;
    connection
        .execute(
            "INSERT INTO agent_transcript_parts (id, part_code, session_code, workspace_code, message_code, seq, part_type, status, content_text, content_json, provider_metadata_json, visibility, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                part.id,
                part.part_code,
                part.session_code,
                part.workspace_code,
                part.message_code,
                part.seq,
                part.part_type,
                part.status,
                part.content_text,
                option_json_to_storage(part.content_json.as_ref())?,
                option_json_to_storage(part.provider_metadata_json.as_ref())?,
                part.visibility,
                timestamp_to_millis(&part.created_at),
                option_timestamp_to_millis(part.completed_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_event(
    connection: &Connection,
    event: &ShortTermMemoryEvent,
) -> Result<(), ShortTermMemoryError> {
    connection
        .execute(
            "INSERT INTO agent_transcript_events (id, event_code, session_code, workspace_code, seq, run_code, input_code, message_code, part_code, event_type, event_source, payload_json, metadata_json, occurred_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                event.id,
                event.event_code,
                event.session_code,
                event.workspace_code,
                event.seq,
                event.run_code,
                event.input_code,
                event.message_code,
                event.part_code,
                event.event_type,
                event.event_source,
                json_to_storage(&event.payload_json)?,
                option_json_to_storage(event.metadata_json.as_ref())?,
                timestamp_to_millis(&event.occurred_at),
                timestamp_to_millis(&event.created_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_todo(
    connection: &Connection,
    todo: &ShortTermMemoryTodo,
) -> Result<(), ShortTermMemoryError> {
    connection
        .execute(
            "INSERT INTO agent_session_todos (id, todo_code, session_code, workspace_code, position, content, status, priority, created_by_run_code, updated_by_run_code, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                todo.id,
                todo.todo_code,
                todo.session_code,
                todo.workspace_code,
                todo.position,
                todo.content,
                todo.status.code(),
                todo.priority.code(),
                todo.created_by_run_code,
                todo.updated_by_run_code,
                timestamp_to_millis(&todo.created_at),
                timestamp_to_millis(&todo.updated_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_context_epoch(
    connection: &Connection,
    compaction: &ShortTermMemoryCompaction,
) -> Result<(), ShortTermMemoryError> {
    let next_id = next_id(connection, "agent_transcript_context_epochs")?;
    let replacement_seq = if compaction.selector_version >= 2 {
        compaction.source_end_seq
    } else {
        compaction.tail_start_seq
    };
    connection
        .execute(
            "INSERT INTO agent_transcript_context_epochs (id, epoch_code, session_code, workspace_code, baseline_seq, replacement_seq, snapshot_json, revision, created_at, updated_at)
             SELECT ?1, ?2, ?3, chat_sessions.workspace_code, ?4, ?5, ?6,
                    (SELECT COALESCE(max(revision), 0) + 1 FROM agent_transcript_context_epochs WHERE session_code = ?3),
                    ?7, ?8
             FROM chat_sessions
             WHERE chat_sessions.session_code = ?3",
            params![
                next_id,
                format!("ATCE-{next_id}"),
                compaction.session_code,
                compaction.source_end_seq,
                replacement_seq,
                json_to_storage(&serde_json::json!({
                    "checkpointMessageCode": compaction.checkpoint_message_code,
                    "sourceStartSeq": compaction.source_start_seq,
                    "sourceEndSeq": compaction.source_end_seq,
                    "tailStartSeq": compaction.tail_start_seq,
                    "selectorVersion": compaction.selector_version,
                    "coverage": compaction.coverage,
                }))?,
                timestamp_to_millis(&compaction.created_at),
                timestamp_to_millis(&compaction.created_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_context_epoch_record(
    connection: &Connection,
    epoch: &ShortTermMemoryContextEpoch,
) -> Result<(), ShortTermMemoryError> {
    connection
        .execute(
            "INSERT INTO agent_transcript_context_epochs (id, epoch_code, session_code, workspace_code, baseline_seq, replacement_seq, agent_code, prompt_prefix_message_count, prompt_prefix_hash, prompt_prefix_preview, snapshot_json, system_hash, developer_hash, tool_hash, skill_hash, variable_hash, execution_scope_hash, memory_hash, revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                epoch.id,
                epoch.epoch_code,
                epoch.session_code,
                epoch.workspace_code,
                epoch.baseline_seq,
                epoch.replacement_seq,
                epoch.agent_code,
                epoch.prompt_prefix_message_count,
                epoch.prompt_prefix_hash,
                epoch.prompt_prefix_preview,
                json_to_storage(&epoch.snapshot_json)?,
                epoch.system_hash,
                epoch.developer_hash,
                epoch.tool_hash,
                epoch.skill_hash,
                epoch.variable_hash,
                epoch.execution_scope_hash,
                epoch.memory_hash,
                epoch.revision,
                timestamp_to_millis(&epoch.created_at),
                timestamp_to_millis(&epoch.updated_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_family(
    connection: &Connection,
    family: &ShortTermMemoryFamily,
) -> Result<(), ShortTermMemoryError> {
    validate_family(family)?;
    connection
        .execute(
            "INSERT INTO agent_session_families (id, family_code, family_kind, family_key, workspace_code, agent_code, user_code, app_code, requirement_code, task_code, parent_family_code, root_session_code, active_session_code, active_run_code, latest_snapshot_code, title, status, load_policy, load_policy_json, metadata_json, created_at, updated_at, last_activity_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
             ON CONFLICT(family_kind, family_key) DO UPDATE SET
                active_session_code = excluded.active_session_code,
                active_run_code = excluded.active_run_code,
                latest_snapshot_code = COALESCE(agent_session_families.latest_snapshot_code, excluded.latest_snapshot_code),
                root_session_code = COALESCE(agent_session_families.root_session_code, excluded.root_session_code),
                title = COALESCE(agent_session_families.title, excluded.title),
                status = excluded.status,
                load_policy = excluded.load_policy,
                load_policy_json = excluded.load_policy_json,
                metadata_json = excluded.metadata_json,
                updated_at = excluded.updated_at,
                last_activity_at = excluded.last_activity_at",
            params![
                family.id,
                family.family_code,
                family.family_kind,
                family.family_key,
                family.workspace_code,
                family.agent_code,
                family.user_code,
                family.app_code,
                family.requirement_code,
                family.task_code,
                family.parent_family_code,
                family.root_session_code,
                family.active_session_code,
                family.active_run_code,
                family.latest_snapshot_code,
                family.title,
                family.status,
                family.load_policy,
                option_json_to_storage(family.load_policy_json.as_ref())?,
                option_json_to_storage(family.metadata_json.as_ref())?,
                timestamp_to_millis(&family.created_at),
                timestamp_to_millis(&family.updated_at),
                timestamp_to_millis(&family.last_activity_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_family_member(
    connection: &Connection,
    member: &ShortTermMemoryFamilyMember,
) -> Result<(), ShortTermMemoryError> {
    connection
        .execute(
            "INSERT INTO agent_session_family_members (id, family_code, session_code, membership_role, parent_session_code, member_kind, origin_kind, model_context_enabled, load_weight_override, load_limits_json, first_run_code, last_run_code, joined_at, last_activity_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(family_code, session_code) DO UPDATE SET
                membership_role = excluded.membership_role,
                parent_session_code = COALESCE(agent_session_family_members.parent_session_code, excluded.parent_session_code),
                member_kind = excluded.member_kind,
                origin_kind = excluded.origin_kind,
                model_context_enabled = excluded.model_context_enabled,
                load_weight_override = excluded.load_weight_override,
                load_limits_json = excluded.load_limits_json,
                first_run_code = COALESCE(agent_session_family_members.first_run_code, excluded.first_run_code),
                last_run_code = excluded.last_run_code,
                last_activity_at = excluded.last_activity_at",
            params![
                member.id,
                member.family_code,
                member.session_code,
                member.membership_role,
                member.parent_session_code,
                member.member_kind,
                member.origin_kind,
                if member.model_context_enabled { 1_i64 } else { 0_i64 },
                member.load_weight_override,
                option_json_to_storage(member.load_limits_json.as_ref())?,
                member.first_run_code,
                member.last_run_code,
                timestamp_to_millis(&member.joined_at),
                timestamp_to_millis(&member.last_activity_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_family_snapshot(
    connection: &Connection,
    snapshot: &ShortTermMemoryFamilySnapshot,
) -> Result<(), ShortTermMemoryError> {
    validate_family_snapshot(snapshot)?;
    connection
        .execute(
            "INSERT INTO agent_session_family_snapshots (id, snapshot_code, family_code, session_code, run_code, snapshot_kind, status, source_start_seq, source_end_seq, source_checkpoint_message_code, summary_markdown, token_estimate, error_code, error_message, metadata_json, created_at, completed_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
             ON CONFLICT(snapshot_code) DO UPDATE SET
                status = excluded.status,
                source_start_seq = excluded.source_start_seq,
                source_end_seq = excluded.source_end_seq,
                source_checkpoint_message_code = excluded.source_checkpoint_message_code,
                summary_markdown = excluded.summary_markdown,
                token_estimate = excluded.token_estimate,
                error_code = excluded.error_code,
                error_message = excluded.error_message,
                metadata_json = excluded.metadata_json,
                completed_at = excluded.completed_at,
                expires_at = excluded.expires_at",
            params![
                snapshot.id,
                snapshot.snapshot_code,
                snapshot.family_code,
                snapshot.session_code,
                snapshot.run_code,
                snapshot.snapshot_kind,
                snapshot.status,
                snapshot.source_start_seq,
                snapshot.source_end_seq,
                snapshot.source_checkpoint_message_code,
                snapshot.summary_markdown,
                snapshot.token_estimate,
                snapshot.error_code,
                snapshot.error_message,
                option_json_to_storage(snapshot.metadata_json.as_ref())?,
                timestamp_to_millis(&snapshot.created_at),
                option_timestamp_to_millis(snapshot.completed_at.as_deref()),
                option_timestamp_to_millis(snapshot.expires_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}
