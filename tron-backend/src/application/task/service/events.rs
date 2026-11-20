use super::*;

impl TaskService {
    pub(super) fn publish_task_detail_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        detail: &TaskDetail,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "task_service")
                .subject("task", detail.task_code.clone())
                .summary(summary)
                .attr(
                    "workspaceCode",
                    detail.workspace_code.clone().unwrap_or_default(),
                )
                .attr(
                    "ownerAssistantCode",
                    detail.owner_assistant_code.clone().unwrap_or_default(),
                )
                .attr("scheduleVersion", detail.schedule_version.to_string())
                .attr(
                    "workSessionCode",
                    detail.work_session_code.clone().unwrap_or_default(),
                )
                .attr("status", format!("{:?}", detail.status))
                .attr("progress", detail.progress.to_string())
                .attr(
                    "assigneeCount",
                    detail.assignee_user_codes.len().to_string(),
                )
                .attr("agentCount", detail.agent_codes.len().to_string())
                .attr("appCount", detail.app_codes.len().to_string()),
        );
    }

    pub(super) fn publish_task_deleted_event(&self, task: &Task) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit("task.deleted", "task_service")
                .subject("task", task.task_code.clone())
                .summary("Deleted task")
                .attr(
                    "workspaceCode",
                    task.workspace_code.clone().unwrap_or_default(),
                )
                .attr("status", task.status.clone()),
        );
    }

    pub(super) fn publish_task_lifecycle_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        result: &TaskLifecycleResult,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "task_service")
                .subject("task", result.task_code.clone())
                .summary(summary)
                .attr("status", format!("{:?}", result.status))
                .attr(
                    "previousStatus",
                    result
                        .previous_status
                        .as_ref()
                        .map(|status| status.to_string())
                        .unwrap_or_else(|| "None".into()),
                ),
        );
    }
}
