use serde_json::json;

use crate::{
    domain::{
        agent::AgentRunContext,
        short_term_memory::{
            ShortTermMemoryError, ShortTermMemoryFamily, ShortTermMemoryFamilyMember,
        },
    },
    shared::time::current_timestamp,
};

use super::{
    ShortTermMemoryService,
    service_family_support::{agent_code, family_code, resolve_bindings, stable_hash},
};

impl ShortTermMemoryService {
    pub fn ensure_heartbeat_session(
        &self,
        workspace_code: &str,
        agent_code: &str,
    ) -> Result<String, ShortTermMemoryError> {
        let workspace_code = workspace_code.trim();
        let agent_code = agent_code.trim();
        if workspace_code.is_empty() || agent_code.is_empty() {
            return Err(ShortTermMemoryError::SessionNotFound);
        }
        let session_code = format!(
            "SESSION-HEARTBEAT-{}",
            stable_hash(&[workspace_code, agent_code, "WORKSPACE_HEARTBEAT"])
        );
        let now = current_timestamp();
        let id = self.next_id("chat_sessions")?;
        self.repository().ensure_hidden_heartbeat_session(
            workspace_code,
            agent_code,
            &session_code,
            id,
            &now,
        )
    }

    pub(super) fn ensure_family_memberships_for_run(
        &self,
        context: &AgentRunContext,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        let agent_code = agent_code(context);
        let bindings = resolve_bindings(context, &agent_code, &self.config);
        if bindings.is_empty() {
            return Ok(());
        }

        let now = current_timestamp();
        let mut families = Vec::with_capacity(bindings.len());
        let mut members = Vec::with_capacity(bindings.len());
        let mut next_family_id = self.next_id("agent_session_families")?;
        let mut next_member_id = self.next_id("agent_session_family_members")?;
        for binding in bindings {
            let family_code = family_code(binding.family_kind, &binding.family_key);
            let family_id = next_family_id;
            next_family_id += 1;
            let member_id = next_member_id;
            next_member_id += 1;
            families.push(ShortTermMemoryFamily {
                id: family_id,
                family_code: family_code.clone(),
                family_kind: binding.family_kind.into(),
                family_key: binding.family_key,
                workspace_code: context.workspace_code.clone(),
                agent_code: agent_code.clone(),
                user_code: context.user_code.clone(),
                app_code: context.execution_scope.app_code.clone(),
                requirement_code: binding.requirement_code.clone(),
                task_code: binding.task_code.clone(),
                parent_family_code: binding.parent_family_code.clone(),
                root_session_code: Some(memory.session_code.clone()),
                active_session_code: Some(memory.session_code.clone()),
                active_run_code: Some(context.run_code.clone()),
                latest_snapshot_code: None,
                title: None,
                status: "ACTIVE".into(),
                load_policy: binding.load_policy.into(),
                load_policy_json: Some(json!({
                    "defaultWeight": binding.load_weight,
                    "maxSessions": self.config.family_load.default_max_sessions,
                    "snapshotMaxChars": self.config.family_load.default_snapshot_max_chars,
                    "ttlHours": self.config.family_load.default_ttl_hours,
                    "goalStateEnabled": true,
                    "todoStateEnabled": true,
                })),
                metadata_json: Some(json!({"strategyVersion": "builtin-v1"})),
                created_at: now.clone(),
                updated_at: now.clone(),
                last_activity_at: now.clone(),
            });
            members.push(ShortTermMemoryFamilyMember {
                id: member_id,
                family_code,
                session_code: memory.session_code.clone(),
                membership_role: binding.membership_role.into(),
                parent_session_code: None,
                member_kind: binding.member_kind.into(),
                origin_kind: binding.origin_kind.into(),
                model_context_enabled: true,
                load_weight_override: None,
                load_limits_json: None,
                first_run_code: Some(context.run_code.clone()),
                last_run_code: Some(context.run_code.clone()),
                joined_at: now.clone(),
                last_activity_at: now.clone(),
            });
        }
        self.repository()
            .upsert_family_memberships(&families, &members)
    }
}
