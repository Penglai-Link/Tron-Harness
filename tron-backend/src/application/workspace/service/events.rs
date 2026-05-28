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
                )
                .attr("directUserCount", detail.direct_user_count.to_string())
                .attr("roleGrantCount", detail.role_grant_count.to_string())
                .attr(
                    "effectiveUserCount",
                    detail.effective_user_count.to_string(),
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

    pub(super) fn publish_workspace_access_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        workspace_code: &str,
        grant_count: usize,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "workspace_service")
                .subject("workspace", workspace_code.to_string())
                .actor(actor)
                .summary(summary)
                .attr("grantCount", grant_count.to_string()),
        );
    }
}
