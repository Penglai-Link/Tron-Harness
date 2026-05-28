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
                .attr("workspaceCode", detail.workspace_code.clone())
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
                        .map(|status| format!("{status:?}"))
                        .unwrap_or_else(|| "None".into()),
                ),
        );
    }
}
