//! Project SDK block compaction observations into the existing product usage audit.
use super::*;
use eliteforge_agent_core as fw;
impl ReactAgent {
    pub(crate) fn framework_memory_reduction_audit(
        &self,
        context: &AgentRunContext,
        block: &fw::ContextBlock,
        result: &eliteforge_agent_memory::MemoryCompactionResult,
    ) -> Result<(), AgentError> {
        let mut context = context.clone();
        context.model = self.framework_compaction_model_config(&context.model);
        let context = &context;
        let token_usage = crate::application::short_term_memory::memory_provider_usage(result);
        let response = ModelResponse::final_output_with_usage(&result.summary, token_usage);
        let estimate = crate::domain::agent::TokenEstimateSnapshot {
            estimated_input_tokens: result.usage.input_tokens.min(u64::from(u32::MAX)) as u32,
            message_token_estimate: result.usage.input_tokens.min(u64::from(u32::MAX)) as u32,
            tool_schema_token_estimate: 0,
            context_window_limit: context.model.context_window_tokens,
            effective_input_budget: context
                .model
                .context_window_tokens
                .map(|tokens| tokens.saturating_sub(context.model.max_tokens.unwrap_or(0))),
            output_token_reserve: context.model.max_tokens,
            rebased: true,
            over_budget: false,
            compaction_trigger_reached: true,
            checkpoint_code: None,
            model_code: context.model.model_code.clone(),
            model_name: context.model.model_name.clone(),
            model_protocol: format!("{:?}", context.model.protocol),
            usage_source: "local_estimate".into(),
            usage_confidence: "estimated".into(),
        };
        let audit = usage_audit_snapshot(estimate, &response, None);
        let mut payload = usage_event_payload("usage_audit_json", &audit);
        payload.push(("context_block_id".into(), block.id.clone()));
        payload.push(("context_event_json".into(), serde_json::json!({"blockId": block.id, "usageAudit": audit, "status": if result.summary.is_empty() { "failed_unavailable" } else { "completed" }}).to_string()));
        self.publish_with_payload(
            context,
            "agent.context.block_compressed",
            if result.summary.is_empty() {
                BusOutcome::Failure
            } else {
                BusOutcome::Success
            },
            None,
            payload,
        )?;
        if let Some(memory) = &self.short_term_memory {
            memory
                .record_framework_memory_reduction(context, &block.id, &audit)
                .map_err(short_term_memory_error)?;
        }
        Ok(())
    }
}
