//! Read-only dynamic provider source checks; transcript persistence stays authoritative.
use super::{
    ShortTermMemoryService,
    assembler::{checkpoint_message_from_parts, working_state_message},
};
use crate::domain::{
    agent::{AgentMessage, AgentRunContext},
    short_term_memory::ShortTermMemoryError,
};
impl ShortTermMemoryService {
    pub(crate) fn framework_history_service(&self) -> Self {
        let mut service = self.clone();
        service.config.compaction.retained_history_ratio_bps =
            self.config.context.providers.history.target_ratio_bps;
        service
    }

    pub(crate) fn framework_history_reducible(
        &self,
        context: &AgentRunContext,
        target: u32,
    ) -> Result<bool, ShortTermMemoryError> {
        let memory = context
            .short_term_memory
            .as_ref()
            .ok_or(ShortTermMemoryError::StorageFailure)?;
        if self.ensure_session(&memory.session_code)? != context.workspace_code {
            return Err(ShortTermMemoryError::StorageFailure);
        }
        let checkpoint = self.repository().latest_compaction(&memory.session_code)?;
        let recent = self.repository().list_messages_after_seq(
            &memory.session_code,
            Self::checkpoint_replay_min_seq(checkpoint.as_ref()),
        )?;
        Ok(!super::history_retention::select_retained_history(
            recent,
            target,
            Some(&context.run_code),
            self.config.context.reasoning_summary_replay_enabled,
        )
        .head_messages
        .is_empty())
    }
    pub(crate) fn record_framework_context(
        &self,
        context: &AgentRunContext,
        prepared: &super::PreparedModelContext,
        report: &eliteforge_agent_core::ContextAssemblyReport,
    ) -> Result<(), ShortTermMemoryError> {
        self.record_context_epoch(
            context,
            prepared,
            &self.model_context_budget_plan(&context.model),
            prepared.messages.len(),
            prepared.estimated_tokens,
            prepared.rebased,
            prepared.over_budget,
        )?;
        if let Some(memory) = &context.short_term_memory {
            self.append_event(&memory.session_code, &context.workspace_code, Some(&context.run_code), Some(&memory.input_code), None, None, "agent.context.assembled", "RUNTIME", serde_json::json!({"estimatedTokens": prepared.estimated_tokens, "budgetBreakdown": prepared.budget_breakdown, "contextReport": report, "source": "framework.default-context-assembler"}), None)?;
        }
        Ok(())
    }
    pub(crate) fn record_framework_memory_reduction(
        &self,
        context: &AgentRunContext,
        block: &str,
        audit: &crate::domain::agent::TokenUsageAuditSnapshot,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = &context.short_term_memory else {
            return Ok(());
        };
        self.append_event(&memory.session_code, &context.workspace_code, Some(&context.run_code), Some(&memory.input_code), None, None, "agent.context.block_compressed", "MODEL", serde_json::json!({"blockId": block, "usageAudit": audit}), Some(serde_json::json!({"usage_audit_json": serde_json::to_string(audit).map_err(|_| ShortTermMemoryError::StorageFailure)?})))
    }
    pub(crate) fn framework_data_source(
        &self,
        context: &AgentRunContext,
        id: &str,
    ) -> Result<(Vec<AgentMessage>, Option<String>), ShortTermMemoryError> {
        let memory = context
            .short_term_memory
            .as_ref()
            .ok_or(ShortTermMemoryError::StorageFailure)?;
        if self.ensure_session(&memory.session_code)? != context.workspace_code {
            return Err(ShortTermMemoryError::StorageFailure);
        }
        let checkpoint = self.repository().latest_compaction(&memory.session_code)?;
        let code = checkpoint
            .as_ref()
            .map(|c| c.checkpoint_message_code.clone());
        let entry = match id {
            "current_session_checkpoint" => checkpoint
                .map(|checkpoint| {
                    self.repository()
                        .message_parts(&checkpoint.checkpoint_message_code)
                        .map(|parts| {
                            checkpoint_message_from_parts(
                                &parts,
                                self.config.context.reasoning_summary_replay_enabled,
                            )
                        })
                })
                .transpose()?
                .flatten(),
            "session_family_context" => self.load_session_family_context(
                &memory.session_code,
                &self.model_context_budget_plan(&context.model),
                memory.todo_enabled,
            )?,
            "current_session_working_state" => {
                let todos = if memory.todo_enabled {
                    self.repository().list_todos(&memory.session_code)?
                } else {
                    vec![]
                };
                let goal = self.load_active_goal_for_context(&memory.session_code)?;
                working_state_message(goal.as_ref(), &todos, memory.todo_enabled, u32::MAX)
            }
            _ => return Err(ShortTermMemoryError::StorageFailure),
        };
        Ok((entry.into_iter().collect(), code))
    }
}
/// Provider observations retain optional cache/reasoning metrics only when known.
pub(crate) fn memory_provider_usage(
    result: &eliteforge_agent_memory::MemoryCompactionResult,
) -> Option<crate::domain::agent::ModelTokenUsage> {
    result.provider_usage.as_ref().map(|usage| {
        let limit = |v: u64| v.min(u64::from(u32::MAX)) as u32;
        crate::domain::agent::ModelTokenUsage {
            input_tokens: limit(usage.input_tokens),
            output_tokens: limit(usage.output_tokens),
            total_tokens: limit(usage.input_tokens.saturating_add(usage.output_tokens)),
            cached_input_tokens: result
                .provider_metrics
                .as_ref()
                .and_then(|m| m.cached_input_tokens)
                .map(limit),
            reasoning_tokens: result
                .provider_metrics
                .as_ref()
                .and_then(|m| m.reasoning_tokens)
                .map(limit),
        }
    })
}
