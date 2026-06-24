use std::sync::{Arc, Mutex};

use crate::{
    application::short_term_memory::{
        CompactionContextPolicy, ModelCompactionFailure, PreparedCompactionInput,
        ShortTermMemoryService,
    },
    domain::agent::{AgentError, AgentModelConfig, AgentRunContext},
};

use super::{
    ReactAgent,
    runtime_helpers::{is_context_overflow_error, short_term_memory_error},
};

impl ReactAgent {
    pub(super) async fn try_auto_context_compaction(
        &self,
        context: &AgentRunContext,
    ) -> Result<bool, AgentError> {
        let Some(short_term_memory) = self.short_term_memory.as_ref() else {
            return Ok(false);
        };
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(false);
        };
        let input = short_term_memory
            .prepare_automatic_model_compaction_input(
                &memory.session_code,
                "auto_over_budget",
                CompactionContextPolicy {
                    todo_enabled: memory.todo_enabled,
                },
                context.model.context_window_tokens,
                context.model.max_tokens,
                Some(&context.run_code),
            )
            .map_err(short_term_memory_error)?;
        let model = short_term_memory.compaction_model_config(&context.model);
        self.run_model_compaction(short_term_memory, input, &model)
            .await?;
        Ok(true)
    }

    pub(super) async fn try_recover_context_overflow(
        &self,
        context: &AgentRunContext,
        message: &str,
        output_started: &Arc<Mutex<bool>>,
        overflow_retries: &mut u32,
    ) -> Result<bool, AgentError> {
        if !is_context_overflow_error(message) {
            return Ok(false);
        }
        if *output_started
            .lock()
            .expect("agent output start mutex poisoned")
        {
            return Ok(false);
        }
        let Some(short_term_memory) = self.short_term_memory.as_ref() else {
            return Ok(false);
        };
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(false);
        };
        if *overflow_retries >= short_term_memory.overflow_retry_limit() {
            return Ok(false);
        }
        *overflow_retries = overflow_retries.saturating_add(1);
        let input = short_term_memory
            .prepare_overflow_model_compaction_input(
                &memory.session_code,
                message,
                CompactionContextPolicy {
                    todo_enabled: memory.todo_enabled,
                },
                context.model.context_window_tokens,
                context.model.max_tokens,
                Some(&context.run_code),
            )
            .map_err(|error| {
                AgentError::ModelFailure(format!("{message}; overflow compaction failed: {error}"))
            })?;
        let model = short_term_memory.compaction_model_config(&context.model);
        self.run_model_compaction(short_term_memory, input, &model)
            .await
            .map_err(|error| {
                AgentError::ModelFailure(format!("{message}; overflow compaction failed: {error}"))
            })?;
        Ok(true)
    }

    async fn run_model_compaction(
        &self,
        short_term_memory: &ShortTermMemoryService,
        input: PreparedCompactionInput,
        model: &AgentModelConfig,
    ) -> Result<(), AgentError> {
        let request = short_term_memory.compaction_model_request(&input);
        let response = match self.model_client.generate(model, &request).await {
            Ok(response) => response,
            Err(message) => {
                let failure = ModelCompactionFailure {
                    error_code: "COMPACTION_MODEL_REQUEST_FAILED".into(),
                    message,
                };
                short_term_memory
                    .record_compaction_failed(&input, &failure)
                    .map_err(short_term_memory_error)?;
                return Err(AgentError::ModelFailure(failure.message));
            }
        };
        let output = match short_term_memory.model_compaction_output(&response) {
            Ok(output) => output,
            Err(failure) => {
                short_term_memory
                    .record_compaction_failed(&input, &failure)
                    .map_err(short_term_memory_error)?;
                return Err(AgentError::ModelFailure(failure.message));
            }
        };
        short_term_memory
            .commit_model_compaction(input, output, model)
            .map_err(short_term_memory_error)?;
        Ok(())
    }
}
