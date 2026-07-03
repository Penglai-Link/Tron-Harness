use rusqlite::{Connection, params};

use crate::{
    domain::short_term_memory::{
        ShortTermMemoryError, ShortTermMemoryGoal, ShortTermMemoryGoalCriterion,
        ShortTermMemoryGoalEvent,
    },
    shared::time::timestamp_to_millis,
};

use super::{option_json_to_storage, option_timestamp_to_millis, storage_error};

fn validate_allowed(value: &str, allowed: &[&str]) -> Result<(), ShortTermMemoryError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ShortTermMemoryError::StorageFailure)
    }
}

fn validate_goal(goal: &ShortTermMemoryGoal) -> Result<(), ShortTermMemoryError> {
    validate_allowed(
        &goal.status,
        &[
            "ACTIVE",
            "PAUSED",
            "BLOCKED",
            "COMPLETED",
            "CANCELLED",
            "CLEARED",
        ],
    )
}

fn validate_goal_criterion(
    criterion: &ShortTermMemoryGoalCriterion,
) -> Result<(), ShortTermMemoryError> {
    validate_allowed(&criterion.status, &["ACTIVE", "REMOVED"])
}

pub(crate) fn insert_goal(
    connection: &Connection,
    goal: &ShortTermMemoryGoal,
) -> Result<(), ShortTermMemoryError> {
    validate_goal(goal)?;
    connection
        .execute(
            "INSERT INTO agent_session_goals (id, goal_code, session_code, workspace_code, user_code, task_code, active_family_code, objective_markdown, status, revision, runs_used, max_runs, last_run_code, last_input_code, last_verdict, last_reason, blocked_reason, required_user_input, consecutive_judge_failures, created_by_run_code, created_at, updated_at, paused_at, blocked_at, completed_at, cancelled_at, cleared_at, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)",
            params![
                goal.id,
                goal.goal_code,
                goal.session_code,
                goal.workspace_code,
                goal.user_code,
                goal.task_code,
                goal.active_family_code,
                goal.objective_markdown,
                goal.status,
                goal.revision,
                goal.runs_used,
                goal.max_runs,
                goal.last_run_code,
                goal.last_input_code,
                goal.last_verdict,
                goal.last_reason,
                goal.blocked_reason,
                goal.required_user_input,
                goal.consecutive_judge_failures,
                goal.created_by_run_code,
                timestamp_to_millis(&goal.created_at),
                timestamp_to_millis(&goal.updated_at),
                option_timestamp_to_millis(goal.paused_at.as_deref()),
                option_timestamp_to_millis(goal.blocked_at.as_deref()),
                option_timestamp_to_millis(goal.completed_at.as_deref()),
                option_timestamp_to_millis(goal.cancelled_at.as_deref()),
                option_timestamp_to_millis(goal.cleared_at.as_deref()),
                option_json_to_storage(goal.metadata_json.as_ref())?,
            ],
        )
        .map_err(goal_write_error)?;
    Ok(())
}

pub(crate) fn update_goal_row(
    connection: &Connection,
    goal: &ShortTermMemoryGoal,
    expected_revision: i64,
) -> Result<(), ShortTermMemoryError> {
    validate_goal(goal)?;
    let updated = connection
        .execute(
            "UPDATE agent_session_goals
             SET objective_markdown = ?2,
                 status = ?3,
                 revision = ?4,
                 runs_used = ?5,
                 max_runs = ?6,
                 last_run_code = ?7,
                 last_input_code = ?8,
                 last_verdict = ?9,
                 last_reason = ?10,
                 blocked_reason = ?11,
                 required_user_input = ?12,
                 consecutive_judge_failures = ?13,
                 updated_at = ?14,
                 paused_at = ?15,
                 blocked_at = ?16,
                 completed_at = ?17,
                 cancelled_at = ?18,
                 cleared_at = ?19,
                 metadata_json = ?20
             WHERE goal_code = ?1
               AND revision = ?21",
            params![
                goal.goal_code,
                goal.objective_markdown,
                goal.status,
                goal.revision,
                goal.runs_used,
                goal.max_runs,
                goal.last_run_code,
                goal.last_input_code,
                goal.last_verdict,
                goal.last_reason,
                goal.blocked_reason,
                goal.required_user_input,
                goal.consecutive_judge_failures,
                timestamp_to_millis(&goal.updated_at),
                option_timestamp_to_millis(goal.paused_at.as_deref()),
                option_timestamp_to_millis(goal.blocked_at.as_deref()),
                option_timestamp_to_millis(goal.completed_at.as_deref()),
                option_timestamp_to_millis(goal.cancelled_at.as_deref()),
                option_timestamp_to_millis(goal.cleared_at.as_deref()),
                option_json_to_storage(goal.metadata_json.as_ref())?,
                expected_revision,
            ],
        )
        .map_err(goal_write_error)?;
    if updated == 0 {
        return Err(ShortTermMemoryError::GoalStateConflict);
    }
    Ok(())
}

pub(crate) fn insert_goal_criterion(
    connection: &Connection,
    criterion: &ShortTermMemoryGoalCriterion,
) -> Result<(), ShortTermMemoryError> {
    validate_goal_criterion(criterion)?;
    connection
        .execute(
            "INSERT INTO agent_session_goal_criteria (id, criterion_code, goal_code, session_code, position, text, status, created_at, updated_at, removed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                criterion.id,
                criterion.criterion_code,
                criterion.goal_code,
                criterion.session_code,
                criterion.position,
                criterion.text,
                criterion.status,
                timestamp_to_millis(&criterion.created_at),
                timestamp_to_millis(&criterion.updated_at),
                option_timestamp_to_millis(criterion.removed_at.as_deref()),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn insert_goal_event(
    connection: &Connection,
    event: &ShortTermMemoryGoalEvent,
) -> Result<(), ShortTermMemoryError> {
    connection
        .execute(
            "INSERT INTO agent_session_goal_events (id, event_code, goal_code, session_code, workspace_code, run_code, input_code, event_kind, event_seq, summary, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                event.id,
                event.event_code,
                event.goal_code,
                event.session_code,
                event.workspace_code,
                event.run_code,
                event.input_code,
                event.event_kind,
                event.event_seq,
                event.summary,
                option_json_to_storage(event.payload_json.as_ref())?,
                timestamp_to_millis(&event.created_at),
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn goal_write_error(error: rusqlite::Error) -> ShortTermMemoryError {
    let message = error.to_string();
    if message.contains("UNIQUE constraint failed: agent_session_goals")
        || message.contains("idx_agent_session_goals_one_open")
    {
        eprintln!("short-term memory sqlite goal conflict: {message}");
        return ShortTermMemoryError::GoalConflict;
    }
    storage_error(error)
}
