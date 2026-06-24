use chrono::{Duration, SecondsFormat, Utc};
use serde_json::json;

use crate::{
    domain::{
        agent::AgentRunContext,
        short_term_memory::{
            ShortTermMemoryError, ShortTermMemoryFamily, ShortTermMemoryFamilyMember,
            ShortTermMemoryFamilySnapshot,
        },
    },
    shared::time::current_timestamp,
};

use super::{ShortTermMemoryService, service_parts::estimate_text_tokens};

use super::{
    service_family_snapshot::{checkpoint_summary_for_family, family_handoff_summary},
    service_family_support::{
        agent_code, family_code, resolve_bindings, snapshot_code, stable_hash, truncate_chars,
    },
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
                    "maxSnapshots": self.config.family_load.default_max_snapshots,
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

    pub(super) fn create_run_family_snapshots(
        &self,
        context: &AgentRunContext,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        let members = self
            .repository()
            .list_enabled_family_members(&memory.session_code)?;
        if members.is_empty() {
            return Ok(());
        }
        let messages = self
            .repository()
            .list_recent_messages(&memory.session_code, 0, 200)?;
        let source_start_seq = messages
            .first()
            .map(|(message, _)| message.seq)
            .unwrap_or(0);
        let source_end_seq = messages.last().map(|(message, _)| message.seq).unwrap_or(0);
        let latest_checkpoint = self.repository().latest_compaction(&memory.session_code)?;
        let latest_checkpoint_summary = latest_checkpoint
            .as_ref()
            .and_then(|checkpoint| {
                self.repository()
                    .message_parts(&checkpoint.checkpoint_message_code)
                    .ok()
            })
            .and_then(|parts| checkpoint_summary_for_family(&parts));
        let summary = family_handoff_summary(
            &messages,
            latest_checkpoint_summary.as_deref(),
            &context.run_code,
            "completed",
        );
        let now = current_timestamp();
        let mut next_snapshot_id = self.next_id("agent_session_family_snapshots")?;
        let snapshots = members
            .iter()
            .map(|(family, _)| {
                let snapshot_id = next_snapshot_id;
                next_snapshot_id += 1;
                ShortTermMemoryFamilySnapshot {
                    id: snapshot_id,
                    snapshot_code: snapshot_code(
                        &family.family_code,
                        &memory.session_code,
                        &context.run_code,
                    ),
                    family_code: family.family_code.clone(),
                    session_code: memory.session_code.clone(),
                    run_code: Some(context.run_code.clone()),
                    snapshot_kind: "RUN_HANDOFF".into(),
                    status: "COMPLETED".into(),
                    source_start_seq: Some(source_start_seq),
                    source_end_seq: Some(source_end_seq),
                    source_checkpoint_message_code: latest_checkpoint
                        .as_ref()
                        .map(|checkpoint| checkpoint.checkpoint_message_code.clone()),
                    summary_markdown: Some(summary.clone()),
                    token_estimate: Some(estimate_text_tokens(&summary)),
                    error_code: None,
                    error_message: None,
                    metadata_json: Some(json!({
                        "source": "run_completion",
                        "strategyVersion": "family-handoff-sanitized-v1",
                        "rawTranscriptOmitted": true,
                    })),
                    created_at: now.clone(),
                    completed_at: Some(now.clone()),
                    expires_at: expires_at_after_hours(self.config.family_load.default_ttl_hours),
                }
            })
            .collect::<Vec<_>>();
        let snapshot_result = self.repository().create_family_snapshots(&snapshots);
        match snapshot_result {
            Ok(()) => Ok(()),
            Err(error) => {
                let bounded_error = truncate_chars(&error.to_string(), 256);
                let failed_snapshots = members
                    .into_iter()
                    .map(|(family, _)| {
                        let snapshot_id = next_snapshot_id;
                        next_snapshot_id += 1;
                        ShortTermMemoryFamilySnapshot {
                            id: snapshot_id,
                            snapshot_code: snapshot_code(
                                &family.family_code,
                                &memory.session_code,
                                &context.run_code,
                            ),
                            family_code: family.family_code,
                            session_code: memory.session_code.clone(),
                            run_code: Some(context.run_code.clone()),
                            snapshot_kind: "RUN_HANDOFF".into(),
                            status: "FAILED".into(),
                            source_start_seq: Some(source_start_seq),
                            source_end_seq: Some(source_end_seq),
                            source_checkpoint_message_code: latest_checkpoint
                                .as_ref()
                                .map(|checkpoint| checkpoint.checkpoint_message_code.clone()),
                            summary_markdown: None,
                            token_estimate: None,
                            error_code: Some("SNAPSHOT_WRITE_FAILED".into()),
                            error_message: Some(bounded_error.clone()),
                            metadata_json: Some(json!({
                                "source": "run_completion",
                                "strategyVersion": "family-handoff-sanitized-v1",
                                "rawTranscriptOmitted": true,
                            })),
                            created_at: now.clone(),
                            completed_at: Some(now.clone()),
                            expires_at: expires_at_after_hours(
                                self.config.family_load.default_ttl_hours,
                            ),
                        }
                    })
                    .collect::<Vec<_>>();
                let _ = self.repository().create_family_snapshots(&failed_snapshots);
                let _ = self.append_event(
                    &memory.session_code,
                    &context.workspace_code,
                    Some(&context.run_code),
                    Some(&memory.input_code),
                    None,
                    None,
                    "agent.session_family.snapshot_failed",
                    "RUNTIME",
                    json!({
                        "runCode": context.run_code,
                        "errorCode": "SNAPSHOT_WRITE_FAILED",
                        "errorMessage": bounded_error,
                    }),
                    None,
                );
                Ok(())
            }
        }
    }
}

fn expires_at_after_hours(hours: u32) -> Option<String> {
    if hours == 0 {
        return None;
    }
    Some(
        (Utc::now() + Duration::hours(i64::from(hours))).to_rfc3339_opts(SecondsFormat::Secs, true),
    )
}
