//! Product compaction selection and persistence. Framework executes the model
//! and controls reassembly, cancellation and bounded overflow recovery.
use super::{ModelClient, ReactAgent, runtime_helpers::short_term_memory_error};
use crate::{
    application::short_term_memory::{
        CompactionContextPolicy, ModelCompactionFailure, PreparedCompactionInput,
        PreparedModelContext, ShortTermMemoryService,
    },
    domain::agent::{AgentError, AgentModelConfig, AgentRunContext, ModelRequest, ModelResponse},
};
use eliteforge_agent_core as fw;
use std::sync::Arc;

pub(crate) struct CompactionModelInput {
    client: Arc<dyn ModelClient>,
    model: AgentModelConfig,
    request: ModelRequest,
}
impl CompactionModelInput {
    pub(crate) async fn generate(&self) -> Result<fw::ChatResponse, fw::ModelError> {
        let response = self
            .client
            .generate(&self.model, &self.request)
            .await
            .map_err(fw::ModelError::Fatal)?;
        let usage = response.token_usage.unwrap_or_default();
        Ok(fw::ChatResponse {
            content: response.output_text().into(),
            structured: Some(
                serde_json::to_value(&response)
                    .map_err(|e| fw::ModelError::Fatal(e.to_string()))?,
            ),
            usage: fw::Usage {
                input_tokens: usage.input_tokens as u64,
                output_tokens: usage.output_tokens as u64,
            },
            ..Default::default()
        })
    }
}
struct CompactionCompletion {
    memory: ShortTermMemoryService,
    input: PreparedCompactionInput,
    model: AgentModelConfig,
}
#[async_trait::async_trait]
impl fw::ContextReductionCompletion for CompactionCompletion {
    async fn complete(
        &self,
        result: Result<fw::ChatResponse, fw::AgentError>,
    ) -> Result<(), fw::AgentError> {
        let response = result.and_then(|response| {
            serde_json::from_value::<ModelResponse>(response.structured.unwrap_or_default())
                .map_err(|error| fw::AgentError::Run(error.to_string()))
        });
        let output = match response {
            Ok(response) => self.memory.model_compaction_output(&response),
            Err(error) => Err(ModelCompactionFailure {
                error_code: "COMPACTION_MODEL_REQUEST_FAILED".into(),
                message: error.to_string(),
            }),
        };
        let map = |error: crate::domain::short_term_memory::ShortTermMemoryError| {
            fw::AgentError::Run(error.to_string())
        };
        match output {
            Ok(output) => self
                .memory
                .commit_model_compaction(self.input.clone(), output, &self.model)
                .map(|_| ())
                .map_err(map),
            Err(failure) => {
                self.memory
                    .record_compaction_failed(&self.input, &failure)
                    .map_err(map)?;
                Err(fw::AgentError::Run(failure.message))
            }
        }
    }
}
impl ReactAgent {
    pub(crate) fn framework_compaction_job(
        &self,
        context: &AgentRunContext,
        prepared: &PreparedModelContext,
        overflow: Option<&str>,
        limits: &fw::RunLimits,
    ) -> Result<Option<fw::ContextReductionJob>, AgentError> {
        let (Some(service), Some(memory)) = (
            self.short_term_memory.as_ref(),
            context.short_term_memory.as_ref(),
        ) else {
            return Ok(None);
        };
        let model = service.compaction_model_config(&context.model);
        let policy = CompactionContextPolicy {
            todo_enabled: memory.todo_enabled,
        };
        let input = match overflow {
            Some(message) => service.prepare_overflow_model_compaction_input(
                &memory.session_code,
                message,
                policy,
                model.context_window_tokens,
                model.max_tokens,
                Some(&context.run_code),
                prepared.compaction_tail_start_seq,
                prepared.effective_retained_history_tokens,
            ),
            None => service.prepare_automatic_model_compaction_input(
                &memory.session_code,
                "auto_compaction_trigger",
                policy,
                model.context_window_tokens,
                model.max_tokens,
                Some(&context.run_code),
                prepared.compaction_tail_start_seq,
                prepared.effective_retained_history_tokens,
            ),
        }
        .map_err(short_term_memory_error)?;
        let native = service.compaction_model_request(&input, &context.locale);
        let messages = native
            .messages
            .iter()
            .map(|message| {
                if message.role == "system" {
                    fw::Message::system(message.text_content())
                } else {
                    fw::Message::user(message.text_content())
                }
            })
            .collect();
        let mut extensions = fw::Extensions::default();
        extensions.insert(CompactionModelInput {
            client: self.model_client.clone(),
            model: model.clone(),
            request: native,
        });
        let mut limits = limits.clone();
        limits.max_context_tokens = model.context_window_tokens.unwrap_or(128_000) as usize;
        limits.max_model_output_tokens = model.max_tokens.unwrap_or(4096);
        Ok(Some(fw::ContextReductionJob {
            source_revision: format!(
                "{}:{:?}:{}:{}",
                input.session_code,
                input.previous_checkpoint_message_code,
                input.source_end_seq,
                input.tail_start_seq
            ),
            model: fw::ModelRef::new("tron", "compaction"),
            stage: "tron.context.compaction".into(),
            limits,
            request: fw::ChatRequest {
                messages,
                tools: vec![],
                extensions,
                options: fw::GenerationOptions {
                    max_output_tokens: model.max_tokens,
                    ..Default::default()
                },
            },
            completion: Arc::new(CompactionCompletion {
                memory: service.clone(),
                input,
                model,
            }),
        }))
    }
}
