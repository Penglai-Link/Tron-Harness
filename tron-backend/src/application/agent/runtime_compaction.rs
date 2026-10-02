//! Product compaction selection and persistence. Framework executes the model
//! and controls reassembly, cancellation and bounded overflow recovery.
use super::{ModelClient, ReactAgent, runtime_helpers::short_term_memory_error};
use crate::{
    application::short_term_memory::{
        CompactionContextPolicy, ModelCompactionFailure, PreparedCompactionInput,
        PreparedModelContext, ShortTermMemoryService,
    },
    domain::agent::{AgentError, AgentModelConfig, AgentRunContext, ModelRequest},
};
use eliteforge_agent_core as fw;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CompactionModelInput {
    client: Arc<dyn ModelClient>,
    model: AgentModelConfig,
}
impl CompactionModelInput {
    pub(crate) fn with_client(&self, client: Arc<dyn ModelClient>) -> Self {
        Self {
            client,
            model: self.model.clone(),
        }
    }
    pub(crate) async fn generate(
        &self,
        request: &fw::ChatRequest,
    ) -> Result<fw::ChatResponse, fw::ModelError> {
        let mut model = self.model.clone();
        model.temperature = request.options.temperature;
        model.top_p = request.options.top_p;
        model.max_tokens = request.options.max_output_tokens;
        let native = eliteforge_agent_official_plugin_model_http::request_from_framework(request)?;
        let native: ModelRequest =
            crate::adapters::outbound::model::http_model_client::convert(&native)
                .map_err(fw::ModelError::Fatal)?;
        let response = self
            .client
            .generate_once(&model, &native)
            .await
            .map_err(|error| {
                eliteforge_agent_official_plugin_model_http::decode_model_error(&error)
                    .unwrap_or(fw::ModelError::Fatal(error))
            })?;
        if response.finish != crate::domain::agent::ModelFinish::Final {
            return Err(fw::ModelError::Fatal(
                "compaction returned tool execution".into(),
            ));
        }
        eliteforge_agent_official_plugin_model_http::response_to_framework(
            crate::adapters::outbound::model::http_model_client::convert(&response)
                .map_err(fw::ModelError::Fatal)?,
        )
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
        let output = result
            .and_then(eliteforge_agent_memory::MemoryCompactionResult::from_chat_response)
            .map(
                |result| crate::application::short_term_memory::ModelCompactionOutput {
                    summary: result.summary.clone(),
                    token_usage: crate::application::short_term_memory::memory_provider_usage(
                        &result,
                    ),
                    provider_metadata: None,
                },
            )
            .map_err(|error| ModelCompactionFailure {
                error_code: "COMPACTION_MODEL_REQUEST_FAILED".into(),
                message: error.to_string(),
            });
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
    pub(crate) fn framework_compaction_extensions(
        &self,
        model: &AgentModelConfig,
    ) -> fw::Extensions {
        let mut extensions = fw::Extensions::default();
        extensions.insert(CompactionModelInput {
            client: self.model_client.clone(),
            model: model.clone(),
        });
        extensions
    }
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
        let service = service.framework_history_service();
        if !service
            .framework_history_reducible(
                context,
                prepared.effective_retained_history_tokens.unwrap_or(0),
            )
            .map_err(short_term_memory_error)?
        {
            return Ok(None);
        }
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
        let mut extensions = fw::Extensions::default();
        extensions.insert(CompactionModelInput {
            client: self.model_client.clone(),
            model: model.clone(),
        });
        let mut limits = limits.clone();
        limits.max_context_tokens = limits.max_context_tokens.min(
            model
                .context_window_tokens
                .unwrap_or(128_000)
                .saturating_sub(model.max_tokens.unwrap_or(4096)) as usize,
        );
        limits.max_model_output_tokens = limits
            .max_model_output_tokens
            .min(model.max_tokens.unwrap_or(4096));
        Ok(Some(fw::ContextReductionJob {
            workflow: Some(Arc::new(AutomaticMemoryWorkflow {
                history: input.history.clone(),
                previous: input.previous_summary.clone(),
                extensions: extensions.clone(),
            })),
            source_revision: format!(
                "{}:{:?}:{}:{}",
                input.session_code,
                input.previous_checkpoint_message_code,
                input.source_end_seq,
                input.tail_start_seq
            ),
            model: self
                .framework_model_scope(context)?
                .summary_reference(&context.model)?,
            stage: "tron.context.compaction".into(),
            limits,
            request: fw::ChatRequest {
                messages: vec![],
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

struct AutomaticMemoryWorkflow {
    history: Vec<String>,
    previous: Option<String>,
    extensions: fw::Extensions,
}
#[async_trait::async_trait]
impl fw::ModelWorkflow for AutomaticMemoryWorkflow {
    async fn run(
        &self,
        run: &fw::RunContext,
        models: &dyn fw::ChatExecutor,
        limits: &fw::RunLimits,
        model: &fw::ModelRef,
    ) -> Result<fw::ChatResponse, fw::AgentError> {
        let mut context = run.clone();
        context.extensions = self.extensions.clone();
        let compactor = eliteforge_agent_memory::MemoryCompactor::new(
            model.clone(),
            eliteforge_agent_memory::CompressionOptions {
                max_input_tokens: limits.max_context_tokens.max(1),
                max_summary_bytes: limits.max_model_output_tokens as usize,
                max_model_calls: 16,
            },
        )?;
        let result = compactor
            .compact_history_with_usage(
                &self.history,
                self.previous.as_deref(),
                &context,
                models,
                limits,
            )
            .await?;
        Ok(result.into_chat_response())
    }
}
