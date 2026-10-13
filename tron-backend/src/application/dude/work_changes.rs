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
        || values.iter().any(|v| v.is_empty() || v.len() > 2000)
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
                if result.trim().is_empty() || result.len() > 32000 {
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
                    || change_key.len() > 200
                    || summary.trim().is_empty()
                    || summary.len() > 8000
                {
                    return Err(invalid(
                        "A stable changeKey and a concrete summary are required",
                    ));
                }
                let requested_action = requested_action
                    .map(|v| v.trim().to_owned())
                    .filter(|v| !v.is_empty());
                if kind.actionable() && requested_action.as_ref().is_none_or(|v| v.len() > 8000) {
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
                if resolution.trim().is_empty() || resolution.len() > 8000 {
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
    if title.trim().is_empty() || title.len() > 800 || body.trim().is_empty() || body.len() > 32000
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

pub(crate) const WORK_PROTOCOL: &str = "\n工作与通知协议：Thread 一轮执行结束、事件处理完毕，只表示得到执行反馈，不表示工作事项完成。先识别所属 task；持续工作或需要用户处理的问题必须建立由你负责的 task。用 dude_work_change(operation=list,taskCode) 查看已有变化和未解决事项。你负责对照用户目标、交付结果和未完成子任务判断是否完成，不得仅凭 Thread Completed 或输出文本收尾。用 dude_work_change(operation=recordResult) 保存结果；它不会关闭任务。确认满足完成条件并处理完所有待确认/阻塞项后，显式调用 task_update 关闭 task，再记录 Completed 变化。\n用 dude_work_change(operation=record) 记录 Progress、Completed、NeedsDecision 或 Blocked，提供稳定 changeKey、具体变化摘要和可追溯 evidence（实际 Thread 结果、文件/制品、消息、interaction ID），不要把巡检、会话关闭、资源释放或‘无需处理’记为业务变化。NeedsDecision/Blocked 必须写清 requestedAction。同一变化沿用 changeKey 和证据，不要改写措辞或更换 key 重复汇报。平台按变化去重，不按执行轮次去重。\n记录变化不会自动通知。由你判断用户是否需要知道，并决定标题和内容，再调用 dude_notify(changeId,title,body)。不要每一步都汇报，合并为对用户有意义的里程碑；普通对话回复保留在会话中。没有实际变化不通知。Thread 的问题先尝试自主解决，确需用户决策才记录并通知 NeedsDecision。通知中说明具体成果、影响或用户要做的决定。\n用户回复不等于问题已经解决；应用回答/决策并确认阻碍解除后，调用 dude_work_change(operation=resolve,changeId,resolution,evidence)，平台同步将原通知标为已解决，不再发一条收尾通知。已读和归档不代表解决。执行失败、等待输入、恢复执行均不会自动发通知；需要提醒用户时必须由你按上述协议显式发布。";
