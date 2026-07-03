use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::{
    application::short_term_memory::ShortTermMemoryRepository,
    domain::short_term_memory::{
        ShortTermMemoryCompaction, ShortTermMemoryContextEpoch, ShortTermMemoryError,
        ShortTermMemoryEvent, ShortTermMemoryFamily, ShortTermMemoryFamilyMember,
        ShortTermMemoryFamilySnapshot, ShortTermMemoryGoal, ShortTermMemoryGoalCriterion,
        ShortTermMemoryGoalEvent, ShortTermMemoryGoalView, ShortTermMemoryInput,
        ShortTermMemoryMessage, ShortTermMemoryPart, ShortTermMemoryTodo,
    },
    shared::time::timestamp_to_millis,
};

use super::short_term_memory_rows::{
    insert_context_epoch, insert_event, insert_family, insert_family_member,
    insert_family_snapshot, insert_message, insert_part, list_parts, option_timestamp_to_millis,
    row_to_family, row_to_family_member, row_to_family_snapshot, storage_error,
};

#[path = "short_term_memory_repository_context.rs"]
mod repository_context;
#[path = "short_term_memory_repository_family.rs"]
mod repository_family;
#[path = "short_term_memory_repository_goal.rs"]
mod repository_goal;
#[path = "short_term_memory_repository_init.rs"]
mod repository_init;
#[path = "short_term_memory_repository_input.rs"]
mod repository_input;
#[path = "short_term_memory_repository_runs.rs"]
mod repository_runs;
#[path = "short_term_memory_repository_todos.rs"]
mod repository_todos;
#[path = "short_term_memory_repository_tool_output.rs"]
mod repository_tool_output;

use super::short_term_memory_repository_append::{
    assign_event_storage_fields, assign_message_storage_fields,
};
use super::short_term_memory_repository_messages as repository_messages;
use repository_init::open_initialized_connection;

pub struct SqliteShortTermMemoryRepository {
    connection: Connection,
}

type GoalLookupResult = Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError>;

impl SqliteShortTermMemoryRepository {
    pub fn connect(database_url: &str) -> Result<Self, ShortTermMemoryError> {
        Ok(Self {
            connection: open_initialized_connection(database_url)?,
        })
    }
}

impl ShortTermMemoryRepository for SqliteShortTermMemoryRepository {
    fn session_workspace(
        &mut self,
        session_code: &str,
    ) -> Result<Option<String>, ShortTermMemoryError> {
        self.connection
            .query_row(
                "SELECT workspace_code FROM chat_sessions WHERE session_code = ?1",
                [session_code],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)
    }

    fn next_id(&mut self, table: &str) -> Result<i64, ShortTermMemoryError> {
        repository_init::next_table_id(&self.connection, table)
    }

    fn next_seq(&mut self, session_code: &str) -> Result<i64, ShortTermMemoryError> {
        let max_seq = self
            .connection
            .query_row(
                "SELECT max(seq) FROM (
                    SELECT seq FROM agent_transcript_events WHERE session_code = ?1
                    UNION ALL
                    SELECT seq FROM agent_transcript_messages WHERE session_code = ?1
                )",
                [session_code],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(storage_error)?
            .unwrap_or(0);
        Ok(max_seq + 1)
    }

    fn input_code_by_idempotency_key(
        &mut self,
        session_code: &str,
        idempotency_key: &str,
    ) -> Result<Option<String>, ShortTermMemoryError> {
        repository_input::input_code_by_idempotency_key(
            &self.connection,
            session_code,
            idempotency_key,
        )
    }

    fn create_input(&mut self, input: &ShortTermMemoryInput) -> Result<(), ShortTermMemoryError> {
        repository_input::create_input(&mut self.connection, input)
    }

    fn create_input_with_message_and_events(
        &mut self,
        input: &ShortTermMemoryInput,
        message: Option<&ShortTermMemoryMessage>,
        parts: &[ShortTermMemoryPart],
        events: &[ShortTermMemoryEvent],
    ) -> Result<(), ShortTermMemoryError> {
        repository_input::create_input_with_message_and_events(
            &mut self.connection,
            input,
            message,
            parts,
            events,
        )
    }

    fn list_inputs_for_run(
        &mut self,
        session_code: &str,
        run_code: &str,
    ) -> Result<Vec<ShortTermMemoryInput>, ShortTermMemoryError> {
        repository_input::list_inputs_for_run(&self.connection, session_code, run_code)
    }

    fn create_event(&mut self, event: &ShortTermMemoryEvent) -> Result<(), ShortTermMemoryError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let mut event = event.clone();
        assign_event_storage_fields(&tx, &mut event)?;
        insert_event(&tx, &event)?;
        tx.commit().map_err(storage_error)
    }

    fn create_event_and_update_run_lease(
        &mut self,
        event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError> {
        repository_runs::create_event_and_update_run_lease(&mut self.connection, event)
    }

    fn create_message(
        &mut self,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
    ) -> Result<(), ShortTermMemoryError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let mut message = message.clone();
        let mut parts = parts.to_vec();
        assign_message_storage_fields(&tx, &mut message, &mut parts)?;
        insert_message(&tx, &message)?;
        for part in &parts {
            insert_part(&tx, part)?;
        }
        tx.commit().map_err(storage_error)
    }

    fn create_message_with_event(
        &mut self,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
        event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let mut message = message.clone();
        let mut parts = parts.to_vec();
        let mut event = event.clone();
        assign_message_storage_fields(&tx, &mut message, &mut parts)?;
        insert_message(&tx, &message)?;
        for part in &parts {
            insert_part(&tx, part)?;
        }
        assign_event_storage_fields(&tx, &mut event)?;
        insert_event(&tx, &event)?;
        tx.commit().map_err(storage_error)
    }

    fn create_message_with_events(
        &mut self,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
        events: &[ShortTermMemoryEvent],
    ) -> Result<(), ShortTermMemoryError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        let mut message = message.clone();
        let mut parts = parts.to_vec();
        assign_message_storage_fields(&tx, &mut message, &mut parts)?;
        insert_message(&tx, &message)?;
        for part in &parts {
            insert_part(&tx, part)?;
        }
        for event in events {
            let mut event = event.clone();
            assign_event_storage_fields(&tx, &mut event)?;
            insert_event(&tx, &event)?;
        }
        tx.commit().map_err(storage_error)
    }

    fn create_context_epoch(
        &mut self,
        epoch: &ShortTermMemoryContextEpoch,
    ) -> Result<(), ShortTermMemoryError> {
        repository_context::create_context_epoch(&mut self.connection, epoch)
    }

    fn create_context_epoch_with_event(
        &mut self,
        epoch: &ShortTermMemoryContextEpoch,
        event: &ShortTermMemoryEvent,
    ) -> Result<ShortTermMemoryContextEpoch, ShortTermMemoryError> {
        repository_context::create_context_epoch_with_event(&mut self.connection, epoch, event)
    }

    fn latest_context_epoch_revision(
        &mut self,
        session_code: &str,
    ) -> Result<i64, ShortTermMemoryError> {
        repository_context::latest_context_epoch_revision(&self.connection, session_code)
    }

    fn list_messages_after_seq(
        &mut self,
        session_code: &str,
        min_seq: i64,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
        repository_messages::list_messages_after_seq(&self.connection, session_code, min_seq)
    }

    fn list_messages_for_run(
        &mut self,
        session_code: &str,
        run_code: &str,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
        repository_messages::list_messages_for_run(&self.connection, session_code, run_code)
    }

    fn list_recent_messages(
        &mut self,
        session_code: &str,
        min_seq: i64,
        limit: usize,
    ) -> Result<Vec<(ShortTermMemoryMessage, Vec<ShortTermMemoryPart>)>, ShortTermMemoryError> {
        repository_messages::list_recent_messages(&self.connection, session_code, min_seq, limit)
    }

    fn message_parts(
        &mut self,
        message_code: &str,
    ) -> Result<Vec<ShortTermMemoryPart>, ShortTermMemoryError> {
        list_parts(&self.connection, message_code)
    }

    fn list_todos(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
        repository_todos::list_todos(&self.connection, session_code)
    }

    fn replace_todos(
        &mut self,
        session_code: &str,
        workspace_code: &str,
        todos: &[ShortTermMemoryTodo],
    ) -> Result<(), ShortTermMemoryError> {
        repository_todos::replace_todos(&mut self.connection, session_code, workspace_code, todos)
    }

    fn replace_todos_with_event(
        &mut self,
        session_code: &str,
        workspace_code: &str,
        todos: &[ShortTermMemoryTodo],
        event: &ShortTermMemoryEvent,
    ) -> Result<(), ShortTermMemoryError> {
        repository_todos::replace_todos_with_event(
            &mut self.connection,
            session_code,
            workspace_code,
            todos,
            event,
        )
    }

    fn latest_compaction(
        &mut self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryCompaction>, ShortTermMemoryError> {
        repository_context::latest_compaction(&self.connection, session_code)
    }

    fn create_compaction(
        &mut self,
        compaction: &ShortTermMemoryCompaction,
        message: &ShortTermMemoryMessage,
        parts: &[ShortTermMemoryPart],
        event: Option<&ShortTermMemoryEvent>,
        enforce_idle: bool,
    ) -> Result<(), ShortTermMemoryError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
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

    fn active_run_exists(&mut self, session_code: &str) -> Result<bool, ShortTermMemoryError> {
        repository_runs::active_run_exists(&self.connection, session_code)
    }

    fn upsert_family_memberships(
        &mut self,
        families: &[ShortTermMemoryFamily],
        members: &[ShortTermMemoryFamilyMember],
    ) -> Result<(), ShortTermMemoryError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        for family in families {
            insert_family(&tx, family)?;
        }
        for member in members {
            insert_family_member(&tx, member)?;
        }
        tx.commit().map_err(storage_error)
    }

    fn list_enabled_family_members(
        &mut self,
        session_code: &str,
    ) -> Result<Vec<(ShortTermMemoryFamily, ShortTermMemoryFamilyMember)>, ShortTermMemoryError>
    {
        let mut member_statement = self
            .connection
            .prepare(
                "SELECT id, family_code, session_code, membership_role, parent_session_code, member_kind, origin_kind, model_context_enabled, load_weight_override, load_limits_json, first_run_code, last_run_code, joined_at, last_activity_at
                 FROM agent_session_family_members
                 WHERE session_code = ?1
                   AND model_context_enabled = 1
                 ORDER BY last_activity_at DESC, id DESC",
            )
            .map_err(storage_error)?;
        let members = member_statement
            .query_map([session_code], row_to_family_member)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        drop(member_statement);

        let mut result = Vec::with_capacity(members.len());
        for member in members {
            let family = self
                .connection
                .query_row(
                    "SELECT id, family_code, family_kind, family_key, workspace_code, agent_code, user_code, app_code, requirement_code, task_code, parent_family_code, root_session_code, active_session_code, active_run_code, latest_snapshot_code, title, status, load_policy, load_policy_json, metadata_json, created_at, updated_at, last_activity_at
                     FROM agent_session_families
                     WHERE family_code = ?1
                       AND status IN ('ACTIVE', 'PAUSED')",
                    [member.family_code.as_str()],
                    row_to_family,
                )
                .optional()
                .map_err(storage_error)?;
            if let Some(family) = family {
                result.push((family, member));
            }
        }
        Ok(result)
    }

    fn list_completed_family_snapshots(
        &mut self,
        family_code: &str,
        exclude_session_code: &str,
        limit: usize,
    ) -> Result<Vec<ShortTermMemoryFamilySnapshot>, ShortTermMemoryError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, snapshot_code, family_code, session_code, run_code, snapshot_kind, status, source_start_seq, source_end_seq, source_checkpoint_message_code, summary_markdown, token_estimate, error_code, error_message, metadata_json, created_at, completed_at, expires_at
                 FROM agent_session_family_snapshots
                 WHERE family_code = ?1
                   AND session_code != ?2
                   AND status = 'COMPLETED'
                   AND (expires_at IS NULL OR expires_at > CAST(strftime('%s','now') AS INTEGER) * 1000)
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?3",
            )
            .map_err(storage_error)?;
        statement
            .query_map(
                params![family_code, exclude_session_code, limit as i64],
                row_to_family_snapshot,
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)
    }

    fn list_family_session_codes(
        &mut self,
        family_code: &str,
        exclude_session_code: &str,
        limit: usize,
    ) -> Result<Vec<String>, ShortTermMemoryError> {
        repository_family::list_family_session_codes(
            &self.connection,
            family_code,
            exclude_session_code,
            limit,
        )
    }

    fn create_family_snapshots(
        &mut self,
        snapshots: &[ShortTermMemoryFamilySnapshot],
    ) -> Result<(), ShortTermMemoryError> {
        let tx = self.connection.transaction().map_err(storage_error)?;
        for snapshot in snapshots {
            insert_family_snapshot(&tx, snapshot)?;
            if snapshot.status == "COMPLETED" {
                tx.execute(
                    "UPDATE agent_session_families
                     SET latest_snapshot_code = ?1,
                         last_activity_at = ?2,
                         updated_at = ?2
                     WHERE family_code = ?3",
                    params![
                        snapshot.snapshot_code,
                        option_timestamp_to_millis(Some(snapshot.created_at.as_str())),
                        snapshot.family_code,
                    ],
                )
                .map_err(storage_error)?;
            }
        }
        tx.commit().map_err(storage_error)
    }

    fn ensure_hidden_heartbeat_session(
        &mut self,
        workspace_code: &str,
        agent_code: &str,
        session_code: &str,
        id: i64,
        now: &str,
    ) -> Result<String, ShortTermMemoryError> {
        let existing = self
            .connection
            .query_row(
                "SELECT session_code
                 FROM chat_sessions
                 WHERE session_code = ?1",
                [session_code],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        if let Some(existing) = existing {
            return Ok(existing);
        }
        self.connection
            .execute(
                "INSERT INTO chat_sessions (id, session_code, workspace_code, user_code, app_code, task_code, agent_code, system_agent_session_id, title, status, last_run_code, last_run_status, created_at, updated_at, source, visibility)
                 VALUES (?1, ?2, ?3, NULL, NULL, NULL, ?4, NULL, ?5, 'active', NULL, NULL, ?6, ?6, 'HEARTBEAT', 'HIDDEN')",
                params![
                    id,
                    session_code,
                    workspace_code,
                    agent_code,
                    format!("Heartbeat {agent_code}"),
                    timestamp_to_millis(now),
                ],
            )
            .map_err(storage_error)?;
        Ok(session_code.to_string())
    }

    fn get_active_goal(&mut self, session_code: &str) -> GoalLookupResult {
        repository_goal::get_active_goal(&self.connection, session_code)
    }

    fn get_latest_goal(&mut self, session_code: &str) -> GoalLookupResult {
        repository_goal::get_latest_goal(&self.connection, session_code)
    }

    fn list_open_goals_batch(
        &mut self,
        after_id: i64,
        limit: usize,
    ) -> Result<Vec<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        repository_goal::list_open_goals(&self.connection, after_id, limit)
    }

    fn get_goal(&mut self, goal_code: &str) -> GoalLookupResult {
        repository_goal::get_goal(&self.connection, goal_code)
    }

    fn list_goal_events(
        &mut self,
        goal_code: &str,
        limit: usize,
    ) -> Result<Vec<ShortTermMemoryGoalEvent>, ShortTermMemoryError> {
        repository_goal::list_goal_events(&self.connection, goal_code, limit)
    }

    fn append_goal_event(
        &mut self,
        event: &ShortTermMemoryGoalEvent,
    ) -> Result<(), ShortTermMemoryError> {
        repository_goal::append_goal_event(&mut self.connection, event)
    }

    fn create_goal(
        &mut self,
        goal: &ShortTermMemoryGoal,
        criteria: &[ShortTermMemoryGoalCriterion],
        event: &ShortTermMemoryGoalEvent,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        repository_goal::create_goal(&mut self.connection, goal, criteria, event)
    }

    fn update_goal(
        &mut self,
        goal: &ShortTermMemoryGoal,
        criteria: &[ShortTermMemoryGoalCriterion],
        event: &ShortTermMemoryGoalEvent,
        expected_revision: i64,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        repository_goal::update_goal(
            &mut self.connection,
            goal,
            criteria,
            event,
            expected_revision,
        )
    }

    fn recover_stale_runs(&mut self, limit: usize) -> Result<usize, ShortTermMemoryError> {
        repository_runs::recover_stale_runs(&mut self.connection, limit)
    }

    fn cleanup_event_log_before(
        &mut self,
        cutoff_created_at_ms: i64,
        batch_size: usize,
        max_rows: usize,
    ) -> Result<usize, ShortTermMemoryError> {
        repository_runs::cleanup_event_log_before(
            &mut self.connection,
            cutoff_created_at_ms,
            batch_size,
            max_rows,
        )
    }

    fn list_externalized_tool_output_paths(
        &mut self,
        root_path: &str,
    ) -> Result<Vec<String>, ShortTermMemoryError> {
        repository_tool_output::list_externalized_tool_output_paths(&self.connection, root_path)
    }
}
