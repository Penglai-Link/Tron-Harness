//! Tron maps authorized product prompt sources and attachments; Framework memory
//! owns assembly and compaction before the final controlled model admission.
use super::model::HostExecution;
use crate::domain::agent::{AgentModelProtocol, EffectiveTool};
use async_trait::async_trait;
use eliteforge_agent_core as fw;

pub(super) struct TronContext;
#[async_trait]
impl fw::ContextPreparationPolicy for TronContext {
    async fn prepare(
        &self,
        input: &fw::ContextAssemblyInput<'_>,
        reductions: usize,
    ) -> Result<fw::ContextPreparation, fw::AgentError> {
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
            .map_err(fw::AgentError::InvalidConfiguration)?
            .into();
        // Instructions belong to the current authorized host scope. Persisted
        // conversation history cannot freeze an earlier workspace/agent prefix.
        let old_prefix_len = state
            .context
            .messages
            .iter()
            .take_while(|message| super::messages::is_prompt_prefix(message))
            .count();
        let prefix = state.trusted_prefix.clone();
        std::sync::Arc::make_mut(&mut state.context.messages).splice(..old_prefix_len, prefix);
        state.context.tools = input
            .tools
            .iter()
            .map(|descriptor| {
                super::tools::map_authorized_descriptor(
                    &state.context,
                    descriptor,
                    &input.run.extensions,
                )
            })
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
        let manual = input.run.extensions.get::<fw::ContextCompactionRequest>().is_some();
        let force = state.context_force_compression || manual;
        let onboarding = state.onboarding;
        let previous_messages = state
            .prepared
            .as_ref()
            .map(|prepared| prepared.messages.clone());
        // No host mutex may be held while a reducer invokes an auxiliary model.
        drop(state);
        let assembled = super::context_providers::assemble(
            input, host.clone(), &context, &projection, force, auxiliary_calls, onboarding,
        ).await;
        let window = match assembled {
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
            .map_err(fw::AgentError::InvalidConfiguration)?
            .into();
        if force && !manual && previous_messages.as_ref() == Some(&projection.messages) {
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
        let snapshot_sequence = host.harness
            .framework_record_context(&context, &projection, &window.report)
            .map_err(|error| fw::AgentError::Run(error.to_string()))?;
        // Inspector metadata follows the native output/replay channel. Do not
        // duplicate full history over SSE or fit a report into bus attribute caps.
        input.run.emit_output(fw::OutputEvent::Custom {
            name: "tron.short_term_memory.context.assembled".into(),
            value: serde_json::json!({
                "contextReport": window.report,
                "budgetBreakdown": projection.budget_breakdown,
                "snapshotSequence": snapshot_sequence.map(|value| value.to_string()),
            }),
        });
        let mut state = host.state.lock().await;
        state.context.messages = projection.messages.clone();
        state.context_force_compression = false;
        state.prepared = Some(projection);
        Ok(fw::ContextPreparation::Ready(window))
    }
}

// Use the vendor tokenizer only for the matching model. Other models retain
// the framework's conservative estimate until their tokenizer is configured.
fn glm_tokenizer(model: &fw::ModelRef) -> Option<&'static tokenizers::Tokenizer> {
    if !model.model.to_ascii_lowercase().ends_with("glm-5.3-flash") {
        return None;
    }
    static TOKENIZER: std::sync::OnceLock<Option<tokenizers::Tokenizer>> = std::sync::OnceLock::new();
    TOKENIZER.get_or_init(|| {
        use std::io::Read;
        let data = include_bytes!("../../../../resources/tokenizers/glm-5.3-flash.tokenizer.json.gz");
        let mut json = Vec::new();
        flate2::read::GzDecoder::new(&data[..]).read_to_end(&mut json).ok()?;
        tokenizers::Tokenizer::from_bytes(json).ok()
    }).as_ref()
}

fn model_text_tokens(model: &fw::ModelRef, text: &str) -> usize {
    glm_tokenizer(model)
        .and_then(|tokenizer| tokenizer.encode(text, false).ok())
        .map_or_else(|| fw::estimate_unicode_text_tokens(text), |encoding| encoding.len())
}

fn model_json_tokens<T: serde::Serialize>(model: &fw::ModelRef, value: &T) -> usize {
    serde_json::to_string(value).map_or(usize::MAX, |text| model_text_tokens(model, &text))
}

pub(super) struct TronTokenEstimator;
#[async_trait]
impl fw::TokenEstimator for TronTokenEstimator {
    async fn message(
        &self,
        model: &fw::ModelRef,
        message: &fw::Message,
    ) -> Result<usize, fw::ModelError> {
        // Count the host message projection that is sent to the model, not
        // its lossless history wrappers. In particular, raw reasoning and
        // encoded tool calls are retained for history but not sent as text.
        let native = super::messages::decode(message).map_err(fw::ModelError::Fatal)?;
        let text = if message.role == fw::Role::Tool {
            // Provider-backed tool IDs send only the output. Generated legacy
            // IDs fall back to the host's textual tool representation.
            if message.tool_call_id.as_deref().is_some_and(|id| !id.starts_with("generated_call_")) {
                native.first_tool_output().unwrap_or_default().to_owned()
            } else {
                native.text_content()
            }
        } else {
            native.plain_text_content()
        };
        let mut projected = message.clone();
        let mut blocks = vec![fw::ContentBlock::Text { text }];
        if message.role == fw::Role::User {
            blocks.extend(message.content.0.iter().filter(|block| matches!(block, fw::ContentBlock::Media { .. })).cloned());
        }
        projected.content = fw::Content(blocks);
        // These fields support history and UI rendering; the provider receives
        // the tool output above and top-level calls, not the sidecar copies.
        projected.structured = None;
        projected.tool_result = None;
        if projected.model_context.as_ref().is_some_and(|context| context.format == "framework.provider-metrics.v1") {
            projected.model_context = None;
        }
        let mut count = fw::TokenEstimator::message(&fw::UnicodeTokenEstimator, model, &projected).await?;
        if glm_tokenizer(model).is_some() {
            let text = match &projected.content.0[0] { fw::ContentBlock::Text { text } => text, _ => unreachable!() };
            count = count.saturating_sub(fw::estimate_unicode_text_tokens(text)).saturating_add(model_text_tokens(model, text));
            for call in &projected.tool_calls {
                let arguments = serde_json::to_string(&call.arguments).unwrap_or_default();
                for part in [&call.id, &call.call_name, &arguments] {
                    count = count.saturating_sub(fw::estimate_unicode_text_tokens(part)).saturating_add(model_text_tokens(model, part));
                }
            }
        }
        Ok(count)
    }
    async fn options(
        &self,
        model: &fw::ModelRef,
        options: &fw::GenerationOptions,
    ) -> Result<usize, fw::ModelError> {
        if glm_tokenizer(model).is_some() {
            return Ok(options.output_schema.as_ref().map_or(0, |schema| model_json_tokens(model, schema)));
        }
        fw::TokenEstimator::options(&fw::UnicodeTokenEstimator, model, options).await
    }

    async fn tools(
        &self,
        model: &fw::ModelRef,
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
        if glm_tokenizer(model).is_some() {
            return Ok(model_json_tokens(model, snapshot.payload()));
        }
        Ok(snapshot.estimated_tokens() as usize)
    }
}

#[cfg(test)]
mod token_estimator_tests {
    use super::*;
    use fw::TokenEstimator;

    #[tokio::test]
    async fn excludes_history_only_parts_but_counts_visible_content_and_calls() {
        let model = fw::ModelRef::new("test", "test");
        let mut clean = fw::Message::new(fw::Role::Assistant, fw::Content(vec![
            fw::ContentBlock::Text { text: "visible answer".into() },
        ]));
        clean.tool_calls.push(fw::ToolCall::new("call-1", "lookup", serde_json::json!({"query":"a2ui"})));
        let mut recorded = clean.clone();
        recorded.content.0.push(fw::ContentBlock::Json { value: serde_json::json!({"tronMessagePart": {"type":"reasoning", "text":"x".repeat(20000)}}) });
        recorded.content.0.push(fw::ContentBlock::Json { value: serde_json::json!({"tronMessagePart": {"type":"toolCall", "call_id":"call-1", "tool_name":"lookup", "input":{"query":"a2ui"}}}) });
        recorded.model_context = Some(fw::ModelMessageContext { model: model.clone(), protocol: "test".into(), format: "framework.provider-metrics.v1".into(), data: serde_json::json!({"padding":"x".repeat(20000)}) });
        let baseline = TronTokenEstimator.message(&model, &clean).await.unwrap();
        assert_eq!(TronTokenEstimator.message(&model, &recorded).await.unwrap(), baseline);
        clean.tool_calls.clear();
        assert!(baseline > TronTokenEstimator.message(&model, &clean).await.unwrap());
        recorded.content.0.push(fw::ContentBlock::Text { text: "more visible content".into() });
        assert!(TronTokenEstimator.message(&model, &recorded).await.unwrap() > baseline);
    }

    #[test]
    fn glm_flash_uses_vendor_tokenizer_only_for_matching_model() {
        let glm = fw::ModelRef::new("test", "PERSON_AI:FIXED:High:glm_plan/glm-5.3-flash");
        let other = fw::ModelRef::new("test", "other-model");
        assert!(glm_tokenizer(&glm).is_some());
        assert!(glm_tokenizer(&other).is_none());
        let sample = r#"{"items":[{"name":"example","count":123}]}"#;
        assert!(model_text_tokens(&glm, sample) < fw::estimate_unicode_text_tokens(sample));
        assert_eq!(model_text_tokens(&other, sample), fw::estimate_unicode_text_tokens(sample));
    }

    #[tokio::test]
    async fn counts_tool_output_without_history_metadata() {
        let model = fw::ModelRef::new("test", "test");
        let mut message = fw::Message::new(fw::Role::Tool, fw::Content(vec![fw::ContentBlock::Text { text: "small result".into() }]));
        message.tool_call_id = Some("call-1".into());
        let estimate = TronTokenEstimator.message(&model, &message).await.unwrap();
        assert_eq!(estimate, 16 + fw::estimate_unicode_text_tokens("small result"));
    }
}
