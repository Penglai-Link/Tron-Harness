use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::{
    application::variable::VariableCipher,
    application::short_term_memory::assembler::{
        assemble_context, checkpoint_message_from_parts, estimate_messages_tokens,
    },
    domain::{
        agent::{AgentRunContext, ModelResponse, ModelTokenUsage},
        short_term_memory::{ShortTermMemoryError, ShortTermMemoryMessage, ShortTermMemoryPart},
    },
    shared::{
        event_bus::EventBusHandle,
        id::{SnowflakeIdProvider, SnowflakedSnowflakeIdProvider},
        time::current_timestamp,
    },
};

use super::{
    ModelResponseTranscriptFacts, PreparedModelContext, ShortTermMemoryConfig,
    ShortTermMemoryRepository,
    history_retention::select_retained_history,
    service_budget::{
        annotate_compaction_budget, apply_request_budget_fields,
        current_session_history_budget_tokens, prompt_prefix_budget_exceeded,
        refresh_over_budget_reason,
    },
    service_parts::transcript_part_from_agent_part,
};

#[derive(Clone)]
pub struct ShortTermMemoryService {
    pub(super) repository: Arc<Mutex<Box<dyn ShortTermMemoryRepository>>>,
    pub(super) id_provider: Arc<dyn SnowflakeIdProvider>,
    pub(super) config: ShortTermMemoryConfig,
    pub(super) event_bus: EventBusHandle,
    pub(super) variable_cipher: Option<Arc<dyn VariableCipher>>,
}

impl ShortTermMemoryService {
    pub(crate) fn framework_context_config(&self) -> &ShortTermMemoryConfig { &self.config }
    pub fn new(
        repository: impl ShortTermMemoryRepository + 'static,
        config: ShortTermMemoryConfig,
    ) -> Self {
        Self::with_id_provider(
            repository,
            config,
            Arc::new(SnowflakedSnowflakeIdProvider::default()),
        )
    }

    pub fn with_id_provider(
        repository: impl ShortTermMemoryRepository + 'static,
        config: ShortTermMemoryConfig,
        id_provider: Arc<dyn SnowflakeIdProvider>,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            id_provider,
            config,
            event_bus: EventBusHandle::disabled(),
            variable_cipher: None,
        }
    }

    pub fn with_event_bus(mut self, event_bus: EventBusHandle) -> Self {
        self.event_bus = event_bus;
        self
    }

    pub fn with_variable_cipher(mut self, variable_cipher: Arc<dyn VariableCipher>) -> Self {
        self.variable_cipher = Some(variable_cipher);
        self
    }

    pub fn clone_with_id_provider(&self, id_provider: Arc<dyn SnowflakeIdProvider>) -> Self {
        Self {
            repository: self.repository.clone(),
            id_provider,
            config: self.config.clone(),
            event_bus: self.event_bus.clone(),
            variable_cipher: self.variable_cipher.clone(),
        }
    }

    pub fn record_run_started(
        &self,
        context: &AgentRunContext,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        self.ensure_family_memberships_for_run(context)?;
        self.append_run_lifecycle_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.run.started",
            "RUNTIME",
            json!({
                "runCode": context.run_code,
                "modelCode": context.model.model_code,
                "reasoningEffort": context.model.reasoning_effort,
            }),
            None,
        )
    }

    pub fn record_run_completed(
        &self,
        context: &AgentRunContext,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        self.append_run_lifecycle_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.run.completed",
            "RUNTIME",
            json!({"runCode": context.run_code}),
            None,
        )
    }

    /// A waiting logical run has ended this physical execution. Native interaction
    /// checkpoints remain resumable; its product execution lease must be released.
    pub fn record_run_awaiting_input(
        &self,
        context: &AgentRunContext,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        self.append_run_lifecycle_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.run.awaiting_input",
            "RUNTIME",
            json!({"runCode": context.run_code}),
            None,
        )
    }

    pub fn record_run_failed(
        &self,
        context: &AgentRunContext,
        reason: &str,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        self.append_run_lifecycle_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.run.failed",
            "RUNTIME",
            json!({"runCode": context.run_code, "reason": reason}),
            None,
        )
    }

    pub fn record_run_failed_with_budget(
        &self,
        context: &AgentRunContext,
        reason: &str,
        prepared_context: &PreparedModelContext,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        self.append_run_lifecycle_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.run.failed",
            "RUNTIME",
            json!({
                "runCode": context.run_code,
                "reason": reason,
                "budgetBreakdown": prepared_context.budget_breakdown.as_ref(),
                "overBudgetReason": prepared_context.over_budget_reason.as_deref(),
            }),
            None,
        )
    }
}
