//! Tron owns prompt sources, attachments and compaction; Framework calls this
//! before admitting a model request, using the same estimator for final admission.
use super::model::HostExecution;
use crate::domain::agent::{AgentModelProtocol, EffectiveTool};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_core::ContextAssembler;

pub(super) struct TronContext;
#[async_trait]
impl fw::ContextProvider for TronContext {
    async fn context(&self, _run: &fw::RunContext) -> Result<Vec<fw::Message>, fw::AgentError> {
        // Ordered product history is seeded into the conversation, not promoted
        // into trusted instructions by the ContextProvider extension point.
        Ok(Vec::new())
    }
}

#[async_trait]
impl fw::ContextPreparationPolicy for TronContext {
    async fn prepare(
        &self,
        input: &fw::ContextAssemblyInput<'_>,
        reductions: usize,
    ) -> Result<fw::ContextPreparation, fw::AgentError> {
        if input
            .run
            .extensions
            .get::<super::external::ExternalExecution>()
            .is_some()
        {
            return fw::DefaultContextAssembler::new(fw::ContextLayout::default(), vec![])?
                .assemble(input)
                .await
                .map(fw::ContextPreparation::Ready);
        }
        let host = HostExecution::from_extensions(&input.run.extensions)?;
        let mut state = host.state.lock().await;
        if reductions == 0 && input.attempt > 1 {
            if let Some(message) = state.last_model_failure.take() {
                state.error = None;
                match host.harness.framework_model_error(&state.context, &message) {
                    fw::ModelError::ContextOverflow(_) => {
                        if let Some(prepared) = state.prepared.as_ref() {
                            let job = host
                                .harness
                                .framework_compaction_job(
                                    &state.context,
                                    prepared,
                                    Some(&message),
                                    input.limits,
                                )
                                .map_err(|error| fw::AgentError::Run(error.to_string()))?;
                            if let Some(job) = job {
                                return Ok(fw::ContextPreparation::Reduce(job));
                            }
                        }
                        return Err(fw::ModelError::ContextOverflow(message).into());
                    }
                    fw::ModelError::Transient(_) => {
                        let super::model::HostRunState { context, turn, .. } = &mut *state;
                        host.harness
                            .framework_fallback(context, turn, &message)
                            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
                    }
                    error => return Err(error.into()),
                }
            }
        }
        state.context.messages = input
            .history
            .messages
            .iter()
            .map(super::messages::decode)
            .collect::<Result<Vec<_>, _>>()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        // Instructions belong to the current authorized host scope. Persisted
        // conversation history cannot freeze an earlier workspace/agent prefix.
        let old_prefix_len = state
            .context
            .messages
            .iter()
            .take_while(|message| matches!(message.role.as_str(), "system" | "developer"))
            .count();
        let prefix = state.trusted_prefix.clone();
        state.context.messages.splice(..old_prefix_len, prefix);
        state.context.tools = input
            .tools
            .iter()
            .map(|descriptor| {
                let mut tool = state
                    .context
                    .authorized_tools()
                    .iter()
                    .find(|tool| tool.tool_name == descriptor.call_name)
                    .cloned()
                    .ok_or_else(|| {
                        fw::AgentError::ToolSetup(format!(
                            "unauthorized tool: {}",
                            descriptor.call_name
                        ))
                    })?;
                tool.description = descriptor.description.clone();
                tool.input_schema = descriptor.input_schema.clone();
                Ok(tool)
            })
            .collect::<Result<Vec<_>, fw::AgentError>>()?;
        let super::model::HostRunState {
            context,
            turn,
            prepared,
            ..
        } = &mut *state;
        let projection = match host.harness.prepare_framework_context(context, turn).await {
            Ok(projection) => projection,
            Err(error) => {
                let message = error.to_string();
                state.error = Some(error);
                return Err(fw::AgentError::Run(message));
            }
        };
        if projection.compaction_required && projection.compaction_trigger_reached {
            if let Some(job) = host
                .harness
                .framework_compaction_job(context, &projection, None, input.limits)
                .map_err(|error| fw::AgentError::Run(error.to_string()))?
            {
                return Ok(fw::ContextPreparation::Reduce(job));
            }
        }
        host.harness
            .validate_framework_context(context, &projection)
            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        let mut messages = projection
            .messages
            .iter()
            .map(super::messages::encode)
            .collect::<Result<Vec<_>, _>>()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        // Carry opaque continuation only for an unchanged assistant message.
        for message in &mut messages {
            if message.role != fw::Role::Assistant {
                continue;
            }
            if let Some(original) = input.history.messages.iter().rev().find(|candidate| {
                candidate.role == message.role
                    && candidate.content == message.content
                    && candidate
                        .tool_calls
                        .iter()
                        .map(|call| (&call.id, &call.call_name, &call.arguments))
                        .eq(message
                            .tool_calls
                            .iter()
                            .map(|call| (&call.id, &call.call_name, &call.arguments)))
            }) {
                message.model_context = original.model_context.clone();
                message.tool_calls = original.tool_calls.clone();
            }
        }
        let tokens = fw::context_tokens(&messages, input.model, input.estimator).await?;
        let protocol = input
            .estimator
            .tools(input.model, input.tools)
            .await?
            .saturating_add(input.estimator.options(input.model, input.options).await?);
        let report = fw::ContextAssemblyReport {
            model: input.model.clone(),
            stage: input.stage.into(),
            step: input.step,
            attempt: input.attempt,
            node_execution: input.run.node_execution().cloned(),
            input_capacity: input.budget.max_tokens,
            protocol_tokens: protocol,
            estimated_input_tokens: tokens.saturating_add(protocol),
            blocks: vec![fw::ContextBlockReport {
                id: "tron.harness.context".into(),
                position: 0,
                source_version: projection.context_hash.clone(),
                original_tokens: Some(
                    fw::context_tokens(&input.history.messages, input.model, input.estimator)
                        .await?,
                ),
                allocated_tokens: input.budget.max_tokens.saturating_sub(protocol),
                final_tokens: tokens,
                outcome: if projection.rebased {
                    fw::ContextBlockOutcome::Reduced
                } else {
                    fw::ContextBlockOutcome::Kept
                },
            }],
        };
        *prepared = Some(projection);
        Ok(fw::ContextPreparation::Ready(fw::ContextWindow {
            messages,
            report,
        }))
    }
}

pub(super) struct TronTokenEstimator;
#[async_trait]
impl fw::TokenEstimator for TronTokenEstimator {
    async fn message(
        &self,
        _: &fw::ModelRef,
        message: &fw::Message,
    ) -> Result<usize, fw::ModelError> {
        let native = super::messages::decode(message).map_err(fw::ModelError::Fatal)?;
        Ok(crate::application::short_term_memory::estimate_messages_tokens(&[native]) as usize)
    }
    async fn tools(
        &self,
        _: &fw::ModelRef,
        tools: &[fw::ToolDescriptor],
    ) -> Result<usize, fw::ModelError> {
        let native: Vec<_> = tools
            .iter()
            .map(|tool| EffectiveTool {
                tool_name: tool.call_name.clone(),
                domain_code: None,
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                skill_name: None,
                required_variable_keys: Vec::new(),
            })
            .collect();
        // ChatCompletions has the larger wrapper. Final provider dispatch also
        // validates its exact schema budget through Tron's protocol cache.
        let snapshot = crate::application::agent::ToolSchemaCache::default()
            .prepare(AgentModelProtocol::ChatCompletions, &native, u32::MAX)
            .map_err(fw::ModelError::Fatal)?;
        Ok(snapshot.estimated_tokens() as usize)
    }
}
