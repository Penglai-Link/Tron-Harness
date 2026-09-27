use super::*;

impl WorkspaceService {
    pub(super) fn publish_workspace_detail_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        detail: &WorkspaceDetail,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "workspace_service")
                .subject("workspace", detail.workspace_code.clone())
                .actor(actor)
                .summary(summary)
                .attr("status", format!("{:?}", detail.status))
                .attr("depth", detail.depth.to_string())
                .attr(
                    "parentWorkspaceCode",
                    detail
                        .parent_workspace_code
                        .clone()
                        .unwrap_or_else(|| "ROOT".into()),
                ),
        );
    }

    pub(super) fn publish_workspace_lifecycle_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        result: &WorkspaceLifecycleResult,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "workspace_service")
                .subject("workspace", result.workspace_code.clone())
                .actor(actor)
                .summary(summary)
                .attr("status", format!("{:?}", result.status))
                .attr(
                    "affectedWorkspaceCount",
                    result.affected_workspace_count.to_string(),
                ),
        );
    }
}
