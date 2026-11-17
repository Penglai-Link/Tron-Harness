use super::AssistantService;
use crate::{
    application::task::TaskService,
    domain::{agent::*, assistant::*},
};
use serde::Deserialize;
use serde_json::{Value, json};

fn invalid(message: &str) -> ToolError {
    ToolError::ExecutionFailed(message.into())
}
fn failure(error: AssistantError) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}
fn evidence(values: Vec<String>) -> Result<Vec<String>, ToolError> {
    let mut values: Vec<_> = values.into_iter().map(|v| v.trim().to_owned()).collect();
    if values.is_empty()
        || values.len() > 20
        || values
            .iter()
            .any(|v| v.is_empty() || v.chars().count() > 2000)
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
        .filter(|v| v.kind.eq_ignore_ascii_case("ASSISTANT") && context.is_owning_assistant())
        .map(|v| v.code.as_str())
        .ok_or_else(|| invalid("Only the owning PersonAI may assess work or publish notifications"))
}
pub(super) fn owned_task(
    tasks: &TaskService,
    assistant: &str,
    task: &str,
) -> Result<crate::domain::task::TaskDetail, ToolError> {
    let task = tasks
        .get_task(task)
        .map_err(|_| invalid("Task is not accessible"))?;
    if task.owner_assistant_code.as_deref() != Some(assistant) {
        return Err(invalid("Task is outside the current PersonAI"));
    }
    Ok(task)
}
fn snapshot(task: &crate::domain::task::TaskDetail) -> Value {
    json!({"status":task.status,"progress":task.progress,"executionResult":task.execution_result})
}
fn similar_summary(left: &str, right: &str) -> bool {
    let normalize = |text: &str| {
        text.chars()
            .flat_map(char::to_lowercase)
            .filter(|c| c.is_alphanumeric())
            .collect::<Vec<_>>()
    };
    let (left, right) = (normalize(left), normalize(right));
    if left.len().min(right.len()) < 8 {
        return false;
    }
    let grams = |text: &[char]| {
        text.windows(2)
            .map(|pair| (pair[0], pair[1]))
            .collect::<std::collections::BTreeSet<_>>()
    };
    let (left, right) = (grams(&left), grams(&right));
    2 * left.intersection(&right).count() * 100 >= 90 * (left.len() + right.len())
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
pub(super) struct AssistantWorkChangeTool {
    pub assistants: AssistantService,
    pub tasks: TaskService,
}
impl Tool for AssistantWorkChangeTool {
    fn name(&self) -> &str {
        "assistant_work_change"
    }
    fn exec(
        &self,
        request: ToolRequest,
        context: &AgentRunContext,
    ) -> Result<ToolExecutionResult, ToolError> {
        let assistant = owner(context)?;
        let input: WorkInput = serde_json::from_value(request.input)
            .map_err(|e| invalid(&format!("Invalid work change: {e}")))?;
        let value = match input {
            WorkInput::RecordResult { task_code, result } => {
                owned_task(&self.tasks, assistant, &task_code)?;
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
                owned_task(&self.tasks, assistant, &task_code)?;
                json!(
                    self.assistants
                        .repo()
                        .map_err(failure)?
                        .work_changes(assistant, &task_code)
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
                let task = owned_task(&self.tasks, assistant, &task_code)?;
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
                if requested_action
                    .as_ref()
                    .is_some_and(|v| v.chars().count() > 8000)
                    || (kind.actionable() && requested_action.is_none())
                {
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
                        "PersonAI must first assess the deliverable, record its result and explicitly close the task and its subtasks",
                    ));
                }
                if kind != WorkChangeKind::Completed
                    && task.status == crate::domain::task::CLOSED_STATUS_CODE
                {
                    return Err(invalid(
                        "Reopen the closed task with assistant_task_update using a non-CLOSED status and progress (use assistant_task_get to list its valid statuses) before reporting further progress or blockers",
                    ));
                }
                let change = AssistantWorkChange {
                    change_id: eliteforge_agent_core::SessionRecord::new("").id,
                    assistant_code: assistant.into(),
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
                } else if kind == WorkChangeKind::Progress {
                    // Task bookkeeping is not a new deliverable. Foreground,
                    // reminder and heartbeat runs share the concrete result.
                    json!({"kind":kind,"executionResult":task.execution_result.as_deref().map(str::trim)})
                } else {
                    json!({"kind":kind,"snapshot":change.task_snapshot})
                }
                .to_string();
                let mut repository = self.assistants.repo().map_err(failure)?;
                let saved = repository
                    .record_work_change(&change, &fingerprint)
                    .map_err(failure)?;
                let mut value = json!(saved);
                if saved.change_id == change.change_id {
                    let warnings = repository.work_changes(assistant, &change.task_code).map_err(failure)?
                        .into_iter().rev().filter(|previous| previous.change_id != change.change_id && previous.kind == kind
                            && similar_summary(&previous.summary, &change.summary)).take(3)
                        .map(|previous| json!({"code":"SIMILAR_WORK_CHANGE","existingChangeId":previous.change_id,
                            "existingNotificationId":previous.notification.map(|notification| notification.id),
                            "message":"A similar milestone already exists for this task. Review and reuse its receipt if this is the same outcome; do not send another notification just because this session used a different changeKey. Distinct outcomes remain separate."})).collect::<Vec<_>>();
                    if !warnings.is_empty() {
                        value["warnings"] = json!(warnings);
                    }
                }
                value
            }
            WorkInput::Resolve {
                change_id,
                resolution,
                evidence: refs,
            } => {
                let change = self
                    .assistants
                    .repo()
                    .map_err(failure)?
                    .work_change(assistant, &change_id)
                    .map_err(failure)?;
                owned_task(&self.tasks, assistant, &change.task_code)?;
                let refs = evidence(refs)?;
                if resolution.trim().is_empty() || resolution.chars().count() > 8000 {
                    return Err(invalid(
                        "Explain the decision or action that resolved this item",
                    ));
                }
                json!(
                    self.assistants
                        .repo()
                        .map_err(failure)?
                        .resolve_work_change(
                            assistant,
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
    assistants: &AssistantService,
    tasks: &TaskService,
    assistant: &str,
    id: &str,
    title: &str,
    body: &str,
) -> Result<PlatformNotification, ToolError> {
    let change = assistants
        .repo()
        .map_err(failure)?
        .work_change(assistant, id)
        .map_err(failure)?;
    let task = owned_task(tasks, assistant, &change.task_code)?;
    let stale = match change.kind {
        WorkChangeKind::Progress => {
            task.status == crate::domain::task::CLOSED_STATUS_CODE
                || task.execution_result.as_deref().map(str::trim)
                    != change.task_snapshot["executionResult"]
                        .as_str()
                        .map(str::trim)
        }
        WorkChangeKind::Completed => snapshot(&task) != change.task_snapshot,
        _ => false,
    };
    if change.notification.is_none() && stale {
        return Err(invalid(
            "Task changed since this assessment. Reassess and record its current change before notifying",
        ));
    }
    if title.trim().is_empty()
        || title.chars().count() > 200
        || body.trim().is_empty()
        || body.chars().count() > 8000
    {
        return Err(invalid("Notification title/body is empty or too long"));
    }
    assistants
        .repo()
        .map_err(failure)?
        .notify_work_change(
            assistant,
            id,
            title,
            body,
            chrono::Utc::now().timestamp_millis(),
        )
        .map_err(failure)
}
