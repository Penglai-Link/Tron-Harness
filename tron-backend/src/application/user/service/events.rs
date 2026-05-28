use super::*;

impl UserService {
    pub(super) fn publish_user_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        user: &User,
        priority: BusPriority,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "user_service")
                .priority(priority)
                .subject("user", user.user_code.clone())
                .actor(actor)
                .summary(summary)
                .attr("status", format!("{:?}", user.status))
                .attr("roleCount", user.role_codes.len().to_string())
                .attr("spaceCount", user.space_codes.len().to_string())
                .attr(
                    "passwordResetRequired",
                    user.password_reset_required.to_string(),
                ),
        );
    }

    pub(super) fn publish_auth_session_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        user: &AuthenticatedUser,
        auth_method: Option<AuthMethod>,
        provider_code: Option<&str>,
        session_code: Option<&str>,
        priority: BusPriority,
    ) {
        let mut draft = BusEventDraft::audit(event_name, "auth_service")
            .priority(priority)
            .subject("user", user.user_code.clone())
            .actor(user)
            .summary(summary)
            .attr(
                "authMethod",
                auth_method
                    .map(auth_method_name)
                    .unwrap_or("unknown")
                    .to_string(),
            )
            .attr(
                "providerCode",
                provider_code.unwrap_or("internal").to_string(),
            );
        if let Some(session_code) = session_code {
            draft = draft.attr("sessionCode", session_code.to_string());
        }
        let _ = self.event_bus.publish(draft);
    }
}
