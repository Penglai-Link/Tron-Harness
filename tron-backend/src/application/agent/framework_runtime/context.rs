//! Tron maps authorized product prompt sources and attachments; Framework memory
//! owns assembly and compaction before the final controlled model admission.
use super::model::HostExecution;
use crate::domain::agent::{AgentModelProtocol, EffectiveTool};
use async_trait::async_trait;
use eliteforge_agent_core as fw;
use eliteforge_agent_core::ContextAssembler;

pub(super) struct TronContext;
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
        super::model::synchronize_selected_model(
            &mut state,
            input.model,
            &input.run.extensions,
            input.options,
        )?;
        if state
            .context_auxiliary_calls
            .as_ref()
            .is_none_or(|(step, _)| *step != input.step)
        {
            state.context_auxiliary_calls = Some((
                input.step,
                std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            ));
        }
        if reductions == 0 && input.attempt > 1 {
            if let Some(message) = state.last_model_failure.take() {
                state.error = None;
                match host.harness.framework_model_error(&state.context, &message) {
                    fw::ModelError::ContextOverflow(_) => {
                        state.context_force_compression = true;
                        // Recovery goes through the same configured global coordinator.
                        // Keep strategies and the additional auxiliary ceiling remain binding.
                    }
                    // ModelInvoker already admitted this recovery. Structured provider
                    // authentication/rate-limit errors must not veto its next dispatch.
                    _ => {}
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
            .map(|descriptor| super::tools::map_authorized_descriptor(&state.context, descriptor))
            .collect::<Result<Vec<_>, fw::AgentError>>()?;
        let super::model::HostRunState { context, turn, .. } = &mut *state;
        let projection = match host.harness.prepare_framework_context(context, turn).await {
            Ok(projection) => projection,
            Err(error) => {
                let message = error.to_string();
                state.error = Some(error);
                return Err(fw::AgentError::Run(message));
            }
        };
        let context = context.clone();
        if state
            .context_auxiliary_calls
            .as_ref()
            .is_none_or(|(step, _)| *step != input.step)
        {
            state.context_auxiliary_calls = Some((
                input.step,
                std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            ));
        }
        let auxiliary_calls = state.context_auxiliary_calls.as_ref().unwrap().1.clone();
        let force = state.context_force_compression;
        let previous_messages = state
            .prepared
            .as_ref()
            .map(|prepared| prepared.messages.clone());
        // No host mutex may be held while a reducer invokes an auxiliary model.
        drop(state);
        let window = match super::context_providers::assemble(
            input,
            host.clone(),
            &context,
            &projection,
            force,
            auxiliary_calls,
        )
        .await
        {
            Ok(window) => window,
            Err(error) => match error.primary() {
                fw::AgentError::Context(fw::ContextError::RefreshRequired { source_revision }) => {
                    return Ok(fw::ContextPreparation::Refresh {
                        source_revision: source_revision.clone(),
                    });
                }
                _ => return Err(error),
            },
        };
        let mut projection = projection;
        projection.messages = window
            .messages
            .iter()
            .map(super::messages::decode)
            .collect::<Result<Vec<_>, _>>()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        if force && previous_messages.as_ref() == Some(&projection.messages) {
            return Err(fw::ModelError::ContextOverflow(
                "configured context strategies reproduced the failed model input".into(),
            )
            .into());
        }
        projection.message_token_estimate = window
            .report
            .estimated_input_tokens
            .saturating_sub(window.report.protocol_tokens)
            .min(u32::MAX as usize) as u32;
        projection.estimated_tokens =
            window.report.estimated_input_tokens.min(u32::MAX as usize) as u32;
        projection.over_budget = false;
        projection.over_budget_reason = None;
        projection.compaction_required = false;
        super::context_projection::update(
            &mut projection,
            &window,
            input,
            &host.harness.framework_context_config(),
        )?;
        host.harness
            .framework_record_context(&context, &projection, &window.report)
            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        let mut state = host.state.lock().await;
        state.context.messages = projection.messages.clone();
        state.context_force_compression = false;
        state.prepared = Some(projection);
        Ok(fw::ContextPreparation::Ready(window))
    }
}

pub(super) struct TronTokenEstimator;
#[async_trait]
impl fw::TokenEstimator for TronTokenEstimator {
    async fn message(
        &self,
        model: &fw::ModelRef,
        message: &fw::Message,
    ) -> Result<usize, fw::ModelError> {
        let native = super::messages::decode(message).map_err(fw::ModelError::Fatal)?;
        let native_tokens =
            crate::application::short_term_memory::estimate_messages_tokens(&[native]) as usize;
        let metadata = message
            .model_context
            .as_ref()
            .map_or(0, |value| {
                serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
            })
            .saturating_add(message.structured.as_ref().map_or(0, |value| {
                serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
            }))
            .saturating_add(message.tool_result.as_ref().map_or(0, |value| {
                serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
            }));
        // Native typed media/resource estimates remain binding even when the
        // product history projection exposes a compact display representation.
        let typed_tokens =
            fw::TokenEstimator::message(&fw::ConservativeTokenEstimator, model, message).await?;
        Ok(native_tokens.saturating_add(metadata).max(typed_tokens))
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
