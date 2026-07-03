use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::domain::short_term_memory::{
    ShortTermMemoryError, ShortTermMemoryGoal, ShortTermMemoryGoalCriterion,
    ShortTermMemoryGoalEvent, ShortTermMemoryGoalView,
};

use super::{
    super::short_term_memory_rows::{
        insert_goal, insert_goal_criterion, insert_goal_event, row_to_goal, row_to_goal_criterion,
        storage_error, update_goal_row,
    },
    repository_init::next_table_id,
};

pub(super) fn get_active_goal(
    connection: &Connection,
    session_code: &str,
) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
    let goal = connection
        .query_row(
            "SELECT id, goal_code, session_code, workspace_code, user_code, task_code, active_family_code, objective_markdown, status, revision, runs_used, max_runs, last_run_code, last_input_code, last_verdict, last_reason, blocked_reason, required_user_input, consecutive_judge_failures, created_by_run_code, created_at, updated_at, paused_at, blocked_at, completed_at, cancelled_at, cleared_at, metadata_json
             FROM agent_session_goals
             WHERE session_code = ?1
               AND status IN ('ACTIVE', 'PAUSED', 'BLOCKED')
             ORDER BY updated_at DESC, id DESC
             LIMIT 1",
            [session_code],
            row_to_goal,
        )
        .optional()
        .map_err(storage_error)?;
    goal.map(|goal| load_goal_view(connection, goal))
        .transpose()
}

pub(super) fn get_latest_goal(
    connection: &Connection,
    session_code: &str,
) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
    let goal = connection
        .query_row(
            "SELECT id, goal_code, session_code, workspace_code, user_code, task_code, active_family_code, objective_markdown, status, revision, runs_used, max_runs, last_run_code, last_input_code, last_verdict, last_reason, blocked_reason, required_user_input, consecutive_judge_failures, created_by_run_code, created_at, updated_at, paused_at, blocked_at, completed_at, cancelled_at, cleared_at, metadata_json
             FROM agent_session_goals
             WHERE session_code = ?1
             ORDER BY updated_at DESC, id DESC
             LIMIT 1",
            [session_code],
            row_to_goal,
        )
        .optional()
        .map_err(storage_error)?;
    goal.map(|goal| load_goal_view(connection, goal))
        .transpose()
}

pub(super) fn list_open_goals(
    connection: &Connection,
    after_id: i64,
    limit: usize,
) -> Result<Vec<ShortTermMemoryGoalView>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, goal_code, session_code, workspace_code, user_code, task_code, active_family_code, objective_markdown, status, revision, runs_used, max_runs, last_run_code, last_input_code, last_verdict, last_reason, blocked_reason, required_user_input, consecutive_judge_failures, created_by_run_code, created_at, updated_at, paused_at, blocked_at, completed_at, cancelled_at, cleared_at, metadata_json
             FROM agent_session_goals
             WHERE status IN ('ACTIVE', 'PAUSED', 'BLOCKED')
               AND id > ?1
             ORDER BY id ASC
             LIMIT ?2",
        )
        .map_err(storage_error)?;
    let goals = statement
        .query_map(params![after_id, limit.max(1) as i64], row_to_goal)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    drop(statement);
    goals
        .into_iter()
        .map(|goal| load_goal_view(connection, goal))
        .collect()
}

pub(super) fn get_goal(
    connection: &Connection,
    goal_code: &str,
) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
    let goal = connection
        .query_row(
            "SELECT id, goal_code, session_code, workspace_code, user_code, task_code, active_family_code, objective_markdown, status, revision, runs_used, max_runs, last_run_code, last_input_code, last_verdict, last_reason, blocked_reason, required_user_input, consecutive_judge_failures, created_by_run_code, created_at, updated_at, paused_at, blocked_at, completed_at, cancelled_at, cleared_at, metadata_json
             FROM agent_session_goals
             WHERE goal_code = ?1",
            [goal_code],
            row_to_goal,
        )
        .optional()
        .map_err(storage_error)?;
    goal.map(|goal| load_goal_view(connection, goal))
        .transpose()
}

pub(super) fn list_goal_events(
    connection: &Connection,
    goal_code: &str,
    limit: usize,
) -> Result<Vec<ShortTermMemoryGoalEvent>, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, event_code, goal_code, session_code, workspace_code, run_code, input_code, event_kind, event_seq, summary, payload_json, created_at
             FROM agent_session_goal_events
             WHERE goal_code = ?1
             ORDER BY event_seq DESC, id DESC
             LIMIT ?2",
        )
        .map_err(storage_error)?;
    let mut events = statement
        .query_map(params![goal_code, limit.max(1) as i64], row_to_goal_event)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    events.reverse();
    Ok(events)
}

pub(super) fn create_goal(
    connection: &mut Connection,
    goal: &ShortTermMemoryGoal,
    criteria: &[ShortTermMemoryGoalCriterion],
    event: &ShortTermMemoryGoalEvent,
) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let mut goal = goal.clone();
    assign_goal_storage_id(&tx, &mut goal)?;
    let mut criteria = criteria.to_vec();
    assign_goal_criteria_storage_ids(&tx, &mut criteria)?;
    let mut event = event.clone();
    assign_goal_event_storage_fields(&tx, &mut event)?;
    insert_goal(&tx, &goal)?;
    for criterion in &criteria {
        insert_goal_criterion(&tx, criterion)?;
    }
    insert_goal_event(&tx, &event)?;
    tx.commit().map_err(storage_error)?;
    Ok(ShortTermMemoryGoalView { goal, criteria })
}

pub(super) fn append_goal_event(
    connection: &mut Connection,
    event: &ShortTermMemoryGoalEvent,
) -> Result<(), ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let mut event = event.clone();
    assign_goal_event_storage_fields(&tx, &mut event)?;
    insert_goal_event(&tx, &event)?;
    tx.commit().map_err(storage_error)?;
    Ok(())
}

pub(super) fn update_goal(
    connection: &mut Connection,
    goal: &ShortTermMemoryGoal,
    criteria: &[ShortTermMemoryGoalCriterion],
    event: &ShortTermMemoryGoalEvent,
    expected_revision: i64,
) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage_error)?;
    let goal = goal.clone();
    let mut criteria = criteria.to_vec();
    assign_goal_criteria_storage_ids(&tx, &mut criteria)?;
    let mut event = event.clone();
    assign_goal_event_storage_fields(&tx, &mut event)?;
    update_goal_row(&tx, &goal, expected_revision)?;
    tx.execute(
        "DELETE FROM agent_session_goal_criteria WHERE goal_code = ?1",
        [goal.goal_code.as_str()],
    )
    .map_err(storage_error)?;
    for criterion in &criteria {
        insert_goal_criterion(&tx, criterion)?;
    }
    insert_goal_event(&tx, &event)?;
    tx.commit().map_err(storage_error)?;
    Ok(ShortTermMemoryGoalView { goal, criteria })
}

pub(super) fn next_goal_event_seq(
    connection: &Connection,
    goal_code: &str,
) -> Result<i64, ShortTermMemoryError> {
    let max_seq = connection
        .query_row(
            "SELECT max(event_seq) FROM agent_session_goal_events WHERE goal_code = ?1",
            [goal_code],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map_err(storage_error)?
        .unwrap_or(0);
    Ok(max_seq + 1)
}

pub(super) fn assign_goal_event_storage_fields(
    connection: &Connection,
    event: &mut ShortTermMemoryGoalEvent,
) -> Result<(), ShortTermMemoryError> {
    event.id = next_table_id(connection, "agent_session_goal_events")?;
    if event.event_code.is_empty() {
        event.event_code = format!("GOALEVENT-{}", event.id);
    }
    event.event_seq = next_goal_event_seq(connection, &event.goal_code)?;
    Ok(())
}

fn assign_goal_storage_id(
    connection: &Connection,
    goal: &mut ShortTermMemoryGoal,
) -> Result<(), ShortTermMemoryError> {
    if goal.id == 0 {
        goal.id = next_table_id(connection, "agent_session_goals")?;
    }
    Ok(())
}

fn assign_goal_criteria_storage_ids(
    connection: &Connection,
    criteria: &mut [ShortTermMemoryGoalCriterion],
) -> Result<(), ShortTermMemoryError> {
    let mut next_id = next_table_id(connection, "agent_session_goal_criteria")?;
    for criterion in criteria {
        if criterion.id == 0 {
            criterion.id = next_id;
            next_id += 1;
        }
        if criterion.criterion_code.is_empty() {
            criterion.criterion_code = format!("CRITERION-{}", criterion.id);
        }
    }
    Ok(())
}

fn row_to_goal_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<ShortTermMemoryGoalEvent> {
    Ok(ShortTermMemoryGoalEvent {
        id: row.get(0)?,
        event_code: row.get(1)?,
        goal_code: row.get(2)?,
        session_code: row.get(3)?,
        workspace_code: row.get(4)?,
        run_code: row.get(5)?,
        input_code: row.get(6)?,
        event_kind: row.get(7)?,
        event_seq: row.get(8)?,
        summary: row.get(9)?,
        payload_json: super::super::short_term_memory_rows::option_json_from_storage(
            row.get::<_, Option<String>>(10)?,
        )?,
        created_at: crate::shared::time::timestamp_from_millis(row.get(11)?),
    })
}

fn load_goal_view(
    connection: &Connection,
    goal: ShortTermMemoryGoal,
) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
    let mut statement = connection
        .prepare(
            "SELECT id, criterion_code, goal_code, session_code, position, text, status, created_at, updated_at, removed_at
             FROM agent_session_goal_criteria
             WHERE goal_code = ?1
             ORDER BY position ASC, id ASC",
        )
        .map_err(storage_error)?;
    let criteria = statement
        .query_map([goal.goal_code.as_str()], row_to_goal_criterion)
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(ShortTermMemoryGoalView { goal, criteria })
}
