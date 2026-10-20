use super::DudeService;
use crate::{
    application::task::TaskService,
    domain::{agent::*, dude::*},
};
use serde::Deserialize;
use serde_json::{Value, json};

fn invalid(message: &str) -> ToolError {
    ToolError::ExecutionFailed(message.into())
}
fn failure(error: DudeError) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
fn evidence(values: Vec<String>) -> Result<Vec<String>, ToolError> {
    let mut values: Vec<_> = values.into_iter().map(|v| v.trim().to_owned()).collect();
    if values.is_empty()
        || values.len() > 20
        || values.iter().any(|v| v.is_empty() || v.chars().count() > 2000)
    {
        return Err(invalid(
            "Provide 1-20 nonempty evidence references (result, artifact, decision or interaction IDs).",
        ));
    }
    values.sort();
    values.dedup();
    Ok(values)
}
pub(super) fn owner(context: &AgentRunContext) -> Result<&str, ToolError> {
    context
        .collaboration_owner
        .as_ref()
        .filter(|v| v.kind.eq_ignore_ascii_case("DUDE") && context.is_owning_dude())
        .map(|v| v.code.as_str())
        .ok_or_else(|| invalid("Only the owning Dude may assess work or publish notifications"))
}
pub(super) fn owned_task(
    tasks: &TaskService,
    dude: &str,
    task: &str,
) -> Result<crate::domain::task::TaskDetail, ToolError> {
    let task = tasks
        .get_task(task)
        .map_err(|_| invalid("Task is not accessible"))?;
    if task.owner_dude_code.as_deref() != Some(dude) {
        return Err(invalid("Task is outside the current Dude"));
    }
    Ok(task)
}
fn snapshot(task: &crate::domain::task::TaskDetail) -> Value {
    json!({"status":task.status,"progress":task.progress,"executionResult":task.execution_result})
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
enum WorkInput {
    #[serde(rename_all = "camelCase")]
    List { task_code: String },
    #[serde(rename_all = "camelCase")]
    RecordResult { task_code: String, result: String },
    #[serde(rename_all = "camelCase")]
    Record {
        task_code: String,
        change_key: String,
        kind: WorkChangeKind,
        summary: String,
        evidence: Vec<String>,
        requested_action: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Resolve {
        change_id: String,
        resolution: String,
        evidence: Vec<String>,
    },
}
pub(super) struct DudeWorkChangeTool {
    pub dudes: DudeService,
    pub tasks: TaskService,
}
impl Tool for DudeWorkChangeTool {
    fn name(&self) -> &str {
        "dude_work_change"
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let dude = owner(context)?;
        let input: WorkInput = serde_json::from_value(request.input)
            .map_err(|e| invalid(&format!("Invalid work change: {e}")))?;
        let value = match input {
            WorkInput::RecordResult { task_code, result } => {
                owned_task(&self.tasks, dude, &task_code)?;
                if result.trim().is_empty() || result.chars().count() > 32000 {
                    return Err(invalid("Provide a concrete execution result"));
                }
                json!(
                    self.tasks
                        .record_execution_result(&task_code, result)
                        .map_err(|e| invalid(&e.to_string()))?
                )
            }
            WorkInput::List { task_code } => {
                owned_task(&self.tasks, dude, &task_code)?;
                json!(
                    self.dudes
                        .repo()
                        .map_err(failure)?
                        .work_changes(dude, &task_code)
                        .map_err(failure)?
                )
            }
            WorkInput::Record {
                task_code,
                change_key,
                kind,
                summary,
                evidence: refs,
                requested_action,
            } => {
                let task = owned_task(&self.tasks, dude, &task_code)?;
                let refs = evidence(refs)?;
                if change_key.trim().is_empty()
                    || change_key.chars().count() > 200
                    || summary.trim().is_empty()
                    || summary.chars().count() > 8000
                {
                    return Err(invalid(
                        "A stable changeKey and a concrete summary are required",
                    ));
                }
                let requested_action = requested_action
                    .map(|v| v.trim().to_owned())
                    .filter(|v| !v.is_empty());
                if requested_action.as_ref().is_some_and(|v| v.chars().count() > 8000)
                    || (kind.actionable() && requested_action.is_none()) {
                    return Err(invalid(
                        "Actionable changes require a specific requestedAction",
                    ));
                }
                if kind == WorkChangeKind::Progress
                    && task
                        .execution_result
                        .as_ref()
                        .is_none_or(|v| v.trim().is_empty())
                {
                    return Err(invalid(
                        "Record the concrete work result before reporting a progress milestone",
                    ));
                }
                if kind == WorkChangeKind::Completed
                    && (task.status != crate::domain::task::CLOSED_STATUS_CODE
                        || task
                            .execution_result
                            .as_ref()
                            .is_none_or(|v| v.trim().is_empty())
                        || task
                            .subtasks
                            .iter()
                            .any(|s| s.status != crate::domain::task::CLOSED_STATUS_CODE))
                {
                    return Err(invalid(
                        "Dude must first assess the deliverable, record its result and explicitly close the task and its subtasks",
                    ));
                }
                if kind != WorkChangeKind::Completed
                    && task.status == crate::domain::task::CLOSED_STATUS_CODE
                {
                    return Err(invalid(
                        "Reopen a closed task before reporting further progress or blockers",
                    ));
                }
                let change = DudeWorkChange {
                    change_id: eliteforge_agent_core::SessionRecord::new("").id,
                    dude_code: dude.into(),
                    task_code,
                    change_key: change_key.trim().into(),
                    kind,
                    summary: summary.trim().into(),
                    evidence: refs,
                    requested_action,
                    session_code: context
                        .collaboration_owner
                        .as_ref()
                        .and_then(|v| v.origin_session_code.clone()),
                    task_snapshot: snapshot(&task),
                    created_at: chrono::Utc::now().timestamp_millis(),
                    resolved_at: None,
                    resolution: None,
                    resolution_evidence: vec![],
                    notification: None,
                };
                // Exclude wording and invocation IDs: paraphrases and later runs share a receipt.
                let fingerprint = if kind.actionable() {
                    json!({"kind":kind,"evidence":change.evidence})
                } else {
                    json!({"kind":kind,"snapshot":change.task_snapshot})
                }
                .to_string();
                json!(
                    self.dudes
                        .repo()
                        .map_err(failure)?
                        .record_work_change(&change, &fingerprint)
                        .map_err(failure)?
                )
            }
            WorkInput::Resolve {
                change_id,
                resolution,
                evidence: refs,
            } => {
                let change = self
                    .dudes
                    .repo()
                    .map_err(failure)?
                    .work_change(dude, &change_id)
                    .map_err(failure)?;
                owned_task(&self.tasks, dude, &change.task_code)?;
                let refs = evidence(refs)?;
                if resolution.trim().is_empty() || resolution.chars().count() > 8000 {
                    return Err(invalid(
                        "Explain the decision or action that resolved this item",
                    ));
                }
                json!(
                    self.dudes
                        .repo()
                        .map_err(failure)?
                        .resolve_work_change(
                            dude,
                            &change_id,
                            resolution.trim(),
                            &refs,
                            chrono::Utc::now().timestamp_millis()
                        )
                        .map_err(failure)?
                )
            }
        };
        Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: self.name().into(),
            output: value.to_string(),
        }))
    }
}
pub(super) fn publish(
    dudes: &DudeService,
    tasks: &TaskService,
    dude: &str,
    id: &str,
    title: &str,
    body: &str,
) -> Result<PlatformNotification, ToolError> {
    let change = dudes
        .repo()
        .map_err(failure)?
        .work_change(dude, id)
        .map_err(failure)?;
    let task = owned_task(tasks, dude, &change.task_code)?;
    if change.notification.is_none()
        && !change.kind.actionable()
        && snapshot(&task) != change.task_snapshot
    {
        return Err(invalid(
            "Task changed since this assessment. Reassess and record its current change before notifying",
        ));
    }
    if title.trim().is_empty() || title.chars().count() > 200 || body.trim().is_empty() || body.chars().count() > 8000
    {
        return Err(invalid("Notification title/body is empty or too long"));
    }
    dudes
        .repo()
        .map_err(failure)?
        .notify_work_change(
            dude,
            id,
            title.trim(),
            body.trim(),
            chrono::Utc::now().timestamp_millis(),
        )
        .map_err(failure)
}
