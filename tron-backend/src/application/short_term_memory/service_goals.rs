use serde_json::json;

use crate::{
    domain::short_term_memory::{
        ShortTermMemoryError, ShortTermMemoryGoal, ShortTermMemoryGoalEvent,
        ShortTermMemoryGoalView,
    },
    shared::time::current_timestamp,
};

use super::{GoalSessionCommand, SetGoalCommand, ShortTermMemoryService, UpdateGoalCommand};

#[cfg(test)]
#[path = "service_goals_tests.rs"]
mod service_goals_tests;

const MAX_GOAL_OBJECTIVE_CHARS: usize = 8_000;

impl ShortTermMemoryService {
    pub fn get_active_goal(
        &self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        self.ensure_session(session_code)?;
        self.repository().get_active_goal(session_code)
    }

    pub fn get_latest_goal(
        &self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        self.ensure_session(session_code)?;
        self.repository().get_latest_goal(session_code)
    }

    pub(super) fn load_active_goal_for_context(
        &self,
        session_code: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        self.repository().get_active_goal(session_code)
    }

    pub fn set_goal(
        &self,
        command: SetGoalCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.set_goal_with_operator(command, None)
    }

    pub fn set_goal_with_operator(
        &self,
        command: SetGoalCommand,
        operator_user_code: Option<String>,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        let session_code = command.session_code.trim();
        let workspace_code = self.ensure_session(session_code)?;
        let objective = command.objective.trim();
        if objective.is_empty() || objective.chars().count() > MAX_GOAL_OBJECTIVE_CHARS {
            return Err(ShortTermMemoryError::GoalInvalid);
        }

        let now = current_timestamp();
        let goal_code = self.next_code("GOAL")?;
        let goal = ShortTermMemoryGoal {
            id: 0,
            goal_code: goal_code.clone(),
            session_code: session_code.to_string(),
            workspace_code: workspace_code.clone(),
            user_code: operator_user_code.and_then(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
            task_code: None,
            active_family_code: None,
            objective_markdown: objective.to_string(),
            status: "ACTIVE".into(),
            revision: 0,
            runs_used: 0,
            max_runs: command
                .max_runs
                .unwrap_or(self.config.goal_loop.max_runs)
                .max(1),
            last_run_code: None,
            last_input_code: None,
            last_verdict: None,
            last_reason: None,
            blocked_reason: None,
            required_user_input: None,
            consecutive_judge_failures: 0,
            created_by_run_code: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            paused_at: None,
            blocked_at: None,
            completed_at: None,
            cancelled_at: None,
            cleared_at: None,
            metadata_json: Some(json!({
                "outerLoopStatus": "not_started",
                "startPolicy": "after_next_run",
            })),
        };
        let criteria = Vec::new();
        let event = self.new_goal_event(
            &goal,
            "GOAL_SET",
            Some("Goal was set"),
            Some(json!({
                "objective": goal.objective_markdown,
                "startPolicy": "after_next_run",
            })),
        )?;
        self.repository().create_goal(&goal, &criteria, &event)
    }

    pub fn pause_goal(
        &self,
        command: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition_goal(command, "PAUSED", "GOAL_PAUSED", "Goal was paused")
    }

    pub fn resume_goal(
        &self,
        command: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition_goal(command, "ACTIVE", "GOAL_RESUMED", "Goal was resumed")
    }

    pub fn cancel_goal(
        &self,
        command: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition_goal(command, "CANCELLED", "GOAL_CANCELLED", "Goal was cancelled")
    }

    pub fn clear_goal(
        &self,
        command: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition_goal(command, "CLEARED", "GOAL_CLEARED", "Goal was cleared")
    }

    pub fn update_goal(
        &self,
        command: UpdateGoalCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        let objective = command.objective.trim();
        if objective.is_empty() || objective.chars().count() > MAX_GOAL_OBJECTIVE_CHARS {
            return Err(ShortTermMemoryError::GoalInvalid);
        }

        let mut view = self.resolve_goal(&command.session_code, Some(&command.goal_code))?;
        if !is_unfinished_goal_status(&view.goal.status) {
            return Err(ShortTermMemoryError::GoalInvalid);
        }

        let expected_revision = view.goal.revision;
        let now = current_timestamp();
        view.goal.objective_markdown = objective.to_string();
        view.goal.max_runs = command.max_runs.unwrap_or(view.goal.max_runs).max(1);
        view.goal.revision = expected_revision.saturating_add(1);
        view.goal.updated_at = now;
        let criteria = Vec::new();
        let event = self.new_goal_event(
            &view.goal,
            "GOAL_UPDATED",
            Some("Goal was updated"),
            Some(json!({
                "objective": view.goal.objective_markdown,
                "maxRuns": view.goal.max_runs,
            })),
        )?;
        self.repository()
            .update_goal(&view.goal, &criteria, &event, expected_revision)
    }

    pub fn should_admit_goal_continuation(
        &self,
        session_code: &str,
        goal_code: &str,
    ) -> Result<bool, ShortTermMemoryError> {
        let active = self.repository().get_active_goal(session_code)?;
        Ok(active
            .as_ref()
            .is_some_and(|view| view.goal.goal_code == goal_code && view.goal.status == "ACTIVE"))
    }

    pub fn recover_goal_state_on_startup(&self) -> Result<(), ShortTermMemoryError> {
        const RECOVERY_BATCH_SIZE: usize = 100;
        let mut after_id = 0_i64;
        loop {
            let goals = self
                .repository()
                .list_open_goals_batch(after_id, RECOVERY_BATCH_SIZE)?;
            if goals.is_empty() {
                break;
            }
            for mut view in goals {
                after_id = after_id.max(view.goal.id);
                if view.goal.status != "ACTIVE" {
                    continue;
                }
                let expected_revision = view.goal.revision;
                if !self
                    .repository()
                    .active_run_exists(&view.goal.session_code)?
                {
                    continue;
                }
                let now = current_timestamp();
                view.goal.status = "PAUSED".into();
                view.goal.revision = expected_revision.saturating_add(1);
                view.goal.paused_at = Some(now.clone());
                view.goal.updated_at = now;
                view.goal.last_verdict = Some("SKIPPED".into());
                view.goal.last_reason = Some("recovery_required".into());
                view.goal.metadata_json = merge_goal_metadata(
                    view.goal.metadata_json.take(),
                    json!({"recoveryStatus": "recovery_required"}),
                );
                let event = self.new_goal_event(
                    &view.goal,
                    "RECOVERY_REQUIRED",
                    Some("Goal was paused because startup found an ambiguous in-flight run"),
                    Some(json!({"reason": "ambiguous_in_flight_run"})),
                )?;
                self.repository().update_goal(
                    &view.goal,
                    &view.criteria,
                    &event,
                    expected_revision,
                )?;
            }
        }
        loop {
            if self.repository().recover_stale_runs(RECOVERY_BATCH_SIZE)? == 0 {
                break;
            }
        }
        Ok(())
    }

    fn transition_goal(
        &self,
        command: GoalSessionCommand,
        status: &str,
        event_kind: &str,
        summary: &str,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        let mut view = self.resolve_goal(&command.session_code, command.goal_code.as_deref())?;
        let expected_revision = view.goal.revision;
        let now = current_timestamp();
        view.goal.status = status.into();
        view.goal.revision = expected_revision.saturating_add(1);
        view.goal.updated_at = now.clone();
        match status {
            "ACTIVE" => {
                view.goal.paused_at = None;
                view.goal.blocked_at = None;
                view.goal.blocked_reason = None;
                view.goal.required_user_input = None;
            }
            "PAUSED" => view.goal.paused_at = Some(now.clone()),
            "CANCELLED" => view.goal.cancelled_at = Some(now.clone()),
            "CLEARED" => view.goal.cleared_at = Some(now.clone()),
            _ => {}
        }
        let event = self.new_goal_event(&view.goal, event_kind, Some(summary), None)?;
        self.repository()
            .update_goal(&view.goal, &view.criteria, &event, expected_revision)
    }

    fn resolve_goal(
        &self,
        session_code: &str,
        goal_code: Option<&str>,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.ensure_session(session_code)?;
        let view = match goal_code {
            Some(goal_code) => self.repository().get_goal(goal_code)?,
            None => self.repository().get_active_goal(session_code)?,
        }
        .ok_or(ShortTermMemoryError::GoalNotFound)?;
        if view.goal.session_code != session_code {
            return Err(ShortTermMemoryError::GoalNotFound);
        }
        Ok(view)
    }

    pub(super) fn new_goal_event(
        &self,
        goal: &ShortTermMemoryGoal,
        event_kind: &str,
        summary: Option<&str>,
        payload_json: Option<serde_json::Value>,
    ) -> Result<ShortTermMemoryGoalEvent, ShortTermMemoryError> {
        Ok(ShortTermMemoryGoalEvent {
            id: 0,
            event_code: self.next_code("GOALEVENT")?,
            goal_code: goal.goal_code.clone(),
            session_code: goal.session_code.clone(),
            workspace_code: goal.workspace_code.clone(),
            run_code: goal.last_run_code.clone(),
            input_code: goal.last_input_code.clone(),
            event_kind: event_kind.into(),
            event_seq: 0,
            summary: summary.map(str::to_string),
            payload_json,
            created_at: current_timestamp(),
        })
    }
}

fn is_unfinished_goal_status(status: &str) -> bool {
    matches!(status, "ACTIVE" | "PAUSED" | "BLOCKED")
}

fn merge_goal_metadata(
    existing: Option<serde_json::Value>,
    patch: serde_json::Value,
) -> Option<serde_json::Value> {
    let mut object = existing.unwrap_or_else(|| json!({}));
    if let (Some(target), Some(source)) = (object.as_object_mut(), patch.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    Some(object)
}
