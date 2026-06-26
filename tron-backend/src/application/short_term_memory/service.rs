use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::{
    application::short_term_memory::assembler::{
        assemble_context, checkpoint_message_from_parts, estimate_messages_tokens,
    },
    domain::{
        agent::{AgentRunContext, ModelResponse, ModelTokenUsage},
        short_term_memory::{ShortTermMemoryError, ShortTermMemoryMessage, ShortTermMemoryPart},
    },
    shared::{
        id::{SnowflakeIdProvider, SnowflakedSnowflakeIdProvider},
        time::current_timestamp,
    },
};

use super::{
    ModelContextBudgetPlan, ModelResponseTranscriptFacts, PreparedModelContext,
    ShortTermMemoryConfig, ShortTermMemoryRepository, history_retention::select_retained_history,
    service_parts::transcript_part_from_agent_part,
};

#[derive(Clone)]
pub struct ShortTermMemoryService {
    pub(super) repository: Arc<Mutex<Box<dyn ShortTermMemoryRepository>>>,
    pub(super) id_provider: Arc<dyn SnowflakeIdProvider>,
    pub(super) config: ShortTermMemoryConfig,
}

impl ShortTermMemoryService {
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
        }
    }

    pub fn clone_with_id_provider(&self, id_provider: Arc<dyn SnowflakeIdProvider>) -> Self {
        Self {
            repository: self.repository.clone(),
            id_provider,
            config: self.config.clone(),
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
            json!({"runCode": context.run_code, "modelCode": context.model.model_code}),
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
        )?;
        self.create_run_family_snapshots(context)
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

    pub fn prepare_model_context(
        &self,
        context: &AgentRunContext,
    ) -> Result<PreparedModelContext, ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(PreparedModelContext {
                messages: context.messages.clone(),
                rebased: false,
                context_hash: None,
                checkpoint_code: None,
                estimated_tokens: estimate_messages_tokens(&context.messages),
                message_token_estimate: estimate_messages_tokens(&context.messages),
                tool_schema_token_estimate: 0,
                budget_breakdown: None,
                over_budget_reason: None,
                context_window_limit: context.model.context_window_tokens,
                effective_input_budget: None,
                output_token_reserve: context.model.max_tokens,
                over_budget: false,
                compaction_trigger_reached: false,
                compaction_required: false,
                compaction_tail_start_seq: None,
                compaction_trigger_tokens: None,
                effective_retained_history_tokens: None,
            });
        };
        let budget_plan = self.config.model_context_budget_plan(
            context.model.context_window_tokens,
            context.model.max_tokens,
        );
        let mut repository = self.repository();
        let compaction = repository.latest_compaction(&memory.session_code)?;
        let min_seq = Self::checkpoint_replay_min_seq(compaction.as_ref());
        let recent_source = repository.list_messages_after_seq(&memory.session_code, min_seq)?;
        let checkpoint = compaction.as_ref().and_then(|item| {
            repository
                .message_parts(&item.checkpoint_message_code)
                .ok()
                .and_then(|parts| {
                    checkpoint_message_from_parts(
                        &parts,
                        self.config.context.reasoning_summary_replay_enabled,
                    )
                })
        });
        let checkpoint_code = checkpoint.as_ref().and_then(|_| {
            compaction
                .as_ref()
                .map(|item| item.checkpoint_message_code.clone())
        });
        let todos = if memory.todo_enabled {
            repository.list_todos(&memory.session_code)?
        } else {
            Vec::new()
        };
        drop(repository);
        let family_context = self.load_session_family_context(
            &memory.session_code,
            &budget_plan,
            memory.todo_enabled,
        )?;
        let active_goal = self.load_active_goal_for_context(&memory.session_code)?;
        let tool_schema_token_estimate = estimate_tool_schema_tokens(&context.tools);
        let mut full_prepared = assemble_context(
            &memory.prompt_prefix,
            memory.current_input_message.as_ref(),
            Some(&context.run_code),
            checkpoint.clone(),
            family_context.clone(),
            &todos,
            active_goal.as_ref(),
            recent_source.clone(),
            memory.todo_enabled,
            &budget_plan,
            self.config.context.reasoning_summary_replay_enabled,
        );
        apply_request_budget_fields(
            &mut full_prepared,
            &budget_plan,
            &self.config,
            tool_schema_token_estimate,
        );
        full_prepared.checkpoint_code = checkpoint_code.clone();
        let full_history_tokens = full_prepared
            .budget_breakdown
            .as_ref()
            .map(|breakdown| breakdown.recent_transcript_tokens);
        annotate_compaction_budget(
            &mut full_prepared,
            &budget_plan,
            &self.config,
            recent_source.len(),
            recent_source.len(),
            0,
            None,
            full_history_tokens,
        );

        if self.config.compaction.auto_enabled
            && full_prepared.compaction_trigger_reached
            && !prompt_prefix_budget_exceeded(&full_prepared)
        {
            let history_budget_tokens = current_session_history_budget_tokens(&full_prepared);
            let retained_history_tokens = history_budget_tokens.saturating_mul(
                self.config
                    .compaction
                    .retained_history_ratio_bps
                    .min(10_000),
            ) / 10_000;
            let selection = select_retained_history(
                recent_source.clone(),
                retained_history_tokens,
                Some(&context.run_code),
                self.config.context.reasoning_summary_replay_enabled,
            );
            if selection.head_messages.is_empty() {
                if let Some(breakdown) = full_prepared.budget_breakdown.as_mut() {
                    breakdown.current_session_history_budget_tokens = Some(history_budget_tokens);
                    breakdown.effective_retained_history_tokens = Some(retained_history_tokens);
                    breakdown.raw_tail_transcript_tokens = selection.tail_token_estimate;
                    breakdown.compressed_history_tokens = 0;
                    breakdown.history_source_message_count = Some(selection.source_message_count);
                    breakdown.raw_tail_message_count = Some(selection.tail_messages.len());
                    breakdown.compressed_history_message_count = Some(0);
                    breakdown.raw_tail_start_seq = Some(selection.tail_start_seq);
                }
                refresh_over_budget_reason(&mut full_prepared);
            } else {
                let selected_tail_message_count = selection.tail_messages.len();
                let compressed_history_message_count = selection.head_messages.len();
                let tail_start_seq = selection.tail_start_seq;
                let full_estimated_tokens = full_prepared.estimated_tokens;
                let mut tail_prepared = assemble_context(
                    &memory.prompt_prefix,
                    memory.current_input_message.as_ref(),
                    Some(&context.run_code),
                    checkpoint,
                    family_context,
                    &todos,
                    active_goal.as_ref(),
                    selection.tail_messages,
                    memory.todo_enabled,
                    &budget_plan,
                    self.config.context.reasoning_summary_replay_enabled,
                );
                apply_request_budget_fields(
                    &mut tail_prepared,
                    &budget_plan,
                    &self.config,
                    tool_schema_token_estimate,
                );
                tail_prepared.checkpoint_code = checkpoint_code;
                tail_prepared.compaction_required = true;
                tail_prepared.compaction_tail_start_seq = Some(tail_start_seq);
                annotate_compaction_budget(
                    &mut tail_prepared,
                    &budget_plan,
                    &self.config,
                    selection.source_message_count,
                    selected_tail_message_count,
                    compressed_history_message_count,
                    Some(tail_start_seq),
                    full_history_tokens,
                );
                tail_prepared.compaction_trigger_reached = true;
                if let Some(breakdown) = tail_prepared.budget_breakdown.as_mut() {
                    breakdown.compaction_trigger_reached = true;
                }
                self.append_event(
                    &memory.session_code,
                    &context.workspace_code,
                    Some(&context.run_code),
                    Some(&memory.input_code),
                    None,
                    None,
                    "agent.context.compaction_requested",
                    "RUNTIME",
                    json!({
                        "estimatedTokens": tail_prepared.estimated_tokens,
                        "fullEstimatedTokens": full_estimated_tokens,
                        "toolSchemaTokenEstimate": tool_schema_token_estimate,
                        "maxOutputTokens": context.model.max_tokens,
                        "triggerEstimateMode": "input_threshold",
                        "triggerRatioBps": self.config.compaction.trigger_ratio_bps,
                        "triggerTokens": tail_prepared.compaction_trigger_tokens,
                        "compactionTriggerReached": tail_prepared.compaction_trigger_reached,
                        "retainedHistoryRatioBps": self.config.compaction.retained_history_ratio_bps,
                        "effectiveRetainedHistoryTokens": tail_prepared.effective_retained_history_tokens,
                        "tailStartSeq": tail_start_seq,
                        "rawTailStartSeq": tail_start_seq,
                        "pinnedStartSeq": selection.pinned_start_seq,
                        "compressedHistoryMessageCount": compressed_history_message_count,
                        "rawTailMessageCount": selected_tail_message_count,
                        "historySourceMessageCount": selection.source_message_count,
                        "historyRetention": {
                            "rawTailStartSeq": tail_start_seq,
                            "pinnedStartSeq": selection.pinned_start_seq,
                            "rawTailMessageCount": selected_tail_message_count,
                            "compressedHistoryMessageCount": compressed_history_message_count,
                            "historySourceMessageCount": selection.source_message_count,
                        },
                        "budgetBreakdown": tail_prepared.budget_breakdown.as_ref(),
                        "overBudgetReason": tail_prepared.over_budget_reason.as_deref(),
                        "compactionMode": "model",
                    }),
                    None,
                )?;
                return Ok(tail_prepared);
            }
        }
        let prepared = full_prepared;
        self.record_context_epoch(
            context,
            &budget_plan,
            prepared.messages.len(),
            prepared.estimated_tokens,
            prepared.rebased,
            prepared.over_budget,
        )?;
        if prepared.rebased {
            self.append_event(
                &memory.session_code,
                &context.workspace_code,
                Some(&context.run_code),
                Some(&memory.input_code),
                None,
                None,
                "agent.context.rebased",
                "RUNTIME",
                json!({"reason": "latest_checkpoint_loaded"}),
                None,
            )?;
        }
        self.append_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            None,
            None,
            "agent.context.assembled",
            "RUNTIME",
            json!({
                "messageCount": prepared.messages.len(),
                "estimatedTokens": prepared.estimated_tokens,
                "toolSchemaTokenEstimate": tool_schema_token_estimate,
                "rebased": prepared.rebased,
                "overBudget": prepared.over_budget,
                "budgetBreakdown": prepared.budget_breakdown.as_ref(),
                "overBudgetReason": prepared.over_budget_reason.as_deref(),
            }),
            None,
        )?;
        Ok(prepared)
    }

    pub fn record_model_response(
        &self,
        context: &AgentRunContext,
        response: &ModelResponse,
    ) -> Result<(), ShortTermMemoryError> {
        self.record_model_response_with_facts(
            context,
            response,
            ModelResponseTranscriptFacts { usage_audit: None },
        )
    }

    pub fn record_model_response_with_facts(
        &self,
        context: &AgentRunContext,
        response: &ModelResponse,
        facts: ModelResponseTranscriptFacts,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(memory) = context.short_term_memory.as_ref() else {
            return Ok(());
        };
        let parts = response
            .assistant
            .parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| {
                transcript_part_from_agent_part(
                    &memory.session_code,
                    &context.workspace_code,
                    (index + 1) as i64,
                    part,
                    false,
                )
            })
            .collect::<Vec<_>>();
        if parts.is_empty() {
            return self.append_event(
                &memory.session_code,
                &context.workspace_code,
                Some(&context.run_code),
                Some(&memory.input_code),
                None,
                None,
                "agent.model.responded",
                "MODEL",
                json!({
                    "responseKind": match response.finish {
                        crate::domain::agent::ModelFinish::Final => "final_output",
                        crate::domain::agent::ModelFinish::ToolCalls => "tool_calls",
                    },
                    "tokenUsage": response.token_usage(),
                    "usageAudit": facts.usage_audit,
                    "messageCode": null,
                }),
                None,
            );
        }
        let (message_code, mut message, parts) = self.build_transcript_message(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            "assistant",
            "normal",
            Some(context),
            parts,
            response.token_usage(),
        )?;
        if let Some(usage_audit) = facts.usage_audit.clone() {
            message.metadata_json = Some(json!({
                "usageAudit": usage_audit,
            }));
        }
        let responded_event = self.new_event(
            &memory.session_code,
            &context.workspace_code,
            Some(&context.run_code),
            Some(&memory.input_code),
            Some(&message_code),
            None,
            "agent.model.responded",
            "MODEL",
            json!({
                "responseKind": match response.finish {
                    crate::domain::agent::ModelFinish::Final => "final_output",
                    crate::domain::agent::ModelFinish::ToolCalls => "tool_calls",
                },
                "tokenUsage": response.token_usage(),
                "usageAudit": facts.usage_audit,
                "messageCode": message_code.clone(),
            }),
            None,
        )?;
        let event = match response.finish {
            crate::domain::agent::ModelFinish::Final => self.new_event(
                &memory.session_code,
                &context.workspace_code,
                Some(&context.run_code),
                Some(&memory.input_code),
                Some(&message_code),
                None,
                "agent.message.assistant_completed",
                "MODEL",
                json!({"runCode": context.run_code, "messageCode": message_code.clone()}),
                None,
            )?,
            crate::domain::agent::ModelFinish::ToolCalls => self.new_event(
                &memory.session_code,
                &context.workspace_code,
                Some(&context.run_code),
                Some(&memory.input_code),
                Some(&message_code),
                None,
                "agent.message.assistant_tool_calls_created",
                "MODEL",
                json!({"count": response.tool_calls().len(), "messageCode": message_code.clone()}),
                None,
            )?,
        };
        let events = [responded_event, event]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        self.repository()
            .create_message_with_events(&message, &parts, &events)?;
        Ok(())
    }

    pub(super) fn build_transcript_message(
        &self,
        session_code: &str,
        workspace_code: &str,
        run_code: Option<&str>,
        role: &str,
        message_type: &str,
        context: Option<&AgentRunContext>,
        mut parts: Vec<ShortTermMemoryPart>,
        token_usage: Option<ModelTokenUsage>,
    ) -> Result<(String, ShortTermMemoryMessage, Vec<ShortTermMemoryPart>), ShortTermMemoryError>
    {
        let message_code = self.next_code("ATM")?;
        let now = current_timestamp();
        for part in &mut parts {
            part.id = 0;
            if part.part_code.is_empty() {
                part.part_code = self.next_code("ATP")?;
            }
            part.message_code = message_code.clone();
        }
        let message = ShortTermMemoryMessage {
            id: 0,
            message_code: message_code.clone(),
            session_code: session_code.to_string(),
            workspace_code: workspace_code.to_string(),
            run_code: run_code.map(str::to_string),
            seq: 0,
            role: role.into(),
            message_type: message_type.into(),
            status: "completed".into(),
            agent_code: context
                .and_then(|ctx| ctx.short_term_memory.as_ref())
                .and_then(|memory| memory.agent_code.clone())
                .or_else(|| context.and_then(|ctx| ctx.execution_scope.app_code.clone())),
            model_code: context.map(|ctx| ctx.model.model_code.clone()),
            model_provider: None,
            model_name: context.map(|ctx| ctx.model.model_name.clone()),
            model_protocol: context.map(|ctx| format!("{:?}", ctx.model.protocol)),
            finish_reason: None,
            error_json: None,
            input_tokens: token_usage.map(|usage| usage.input_tokens),
            output_tokens: token_usage.map(|usage| usage.output_tokens),
            reasoning_tokens: None,
            total_tokens: token_usage.map(|usage| usage.total_tokens),
            metadata_json: None,
            created_at: now.clone(),
            completed_at: Some(now),
        };
        Ok((message_code, message, parts))
    }

    pub(super) fn ensure_session(
        &self,
        session_code: &str,
    ) -> Result<String, ShortTermMemoryError> {
        self.repository()
            .session_workspace(session_code.trim())?
            .ok_or(ShortTermMemoryError::SessionNotFound)
    }
}

fn estimate_tool_schema_tokens(tools: &[crate::domain::agent::EffectiveTool]) -> u32 {
    serde_json::to_string(tools)
        .ok()
        .map(|text| ((text.len() as u32).saturating_add(3)) / 4)
        .unwrap_or(0)
}

fn apply_request_budget_fields(
    prepared: &mut PreparedModelContext,
    budget_plan: &ModelContextBudgetPlan,
    config: &ShortTermMemoryConfig,
    tool_schema_token_estimate: u32,
) {
    prepared.estimated_tokens = prepared
        .message_token_estimate
        .saturating_add(tool_schema_token_estimate);
    prepared.tool_schema_token_estimate = tool_schema_token_estimate;
    prepared.context_window_limit = Some(budget_plan.model_context_window);
    prepared.effective_input_budget = Some(budget_plan.effective_input_budget);
    prepared.output_token_reserve = Some(budget_plan.output_token_reserve);
    let trigger_tokens = compaction_trigger_tokens(budget_plan, Some(config));
    prepared.compaction_trigger_tokens = Some(trigger_tokens);
    prepared.compaction_trigger_reached = prepared.estimated_tokens >= trigger_tokens;
    let input_budget_exceeded = prepared.estimated_tokens > budget_plan.effective_input_budget;
    prepared.over_budget = input_budget_exceeded || prompt_prefix_budget_exceeded(prepared);
    if let Some(breakdown) = prepared.budget_breakdown.as_mut() {
        breakdown.tool_schema_tokens = tool_schema_token_estimate;
        breakdown.estimated_input_tokens = prepared.estimated_tokens;
        breakdown.effective_input_budget = Some(budget_plan.effective_input_budget);
        breakdown.context_window_limit = Some(budget_plan.model_context_window);
        breakdown.output_token_reserve = Some(budget_plan.output_token_reserve);
        breakdown.prompt_prefix_budget_tokens = Some(
            budget_plan
                .block_budgets
                .seed_prompt_prefix
                .effective_limit_tokens,
        );
        breakdown.compaction_trigger_ratio_bps = Some(config.compaction.trigger_ratio_bps);
        breakdown.compaction_trigger_tokens = Some(trigger_tokens);
        breakdown.compaction_trigger_reached = prepared.compaction_trigger_reached;
        breakdown.input_budget_exceeded = input_budget_exceeded;
    }
    refresh_over_budget_reason(prepared);
}

fn annotate_compaction_budget(
    prepared: &mut PreparedModelContext,
    budget_plan: &ModelContextBudgetPlan,
    config: &ShortTermMemoryConfig,
    selection_source_message_count: usize,
    raw_tail_message_count: usize,
    compressed_history_message_count: usize,
    tail_start_seq: Option<i64>,
    source_history_token_estimate: Option<u32>,
) {
    let raw_tail_tokens = prepared
        .budget_breakdown
        .as_ref()
        .map(|breakdown| breakdown.recent_transcript_tokens)
        .unwrap_or(0);
    let history_budget_tokens = current_session_history_budget_tokens(prepared);
    let retained_history_tokens = history_budget_tokens
        .saturating_mul(config.compaction.retained_history_ratio_bps.min(10_000))
        / 10_000;
    let trigger_tokens = compaction_trigger_tokens(budget_plan, Some(config));
    prepared.compaction_trigger_tokens = Some(trigger_tokens);
    prepared.compaction_trigger_reached = prepared.estimated_tokens >= trigger_tokens;
    prepared.effective_retained_history_tokens = Some(retained_history_tokens);
    let input_budget_exceeded = prepared.estimated_tokens > budget_plan.effective_input_budget;
    prepared.over_budget = input_budget_exceeded || prompt_prefix_budget_exceeded(prepared);
    if let Some(breakdown) = prepared.budget_breakdown.as_mut() {
        breakdown.raw_tail_transcript_tokens = raw_tail_tokens;
        breakdown.compressed_history_tokens = if compressed_history_message_count > 0 {
            source_history_token_estimate
                .unwrap_or(breakdown.recent_transcript_tokens)
                .saturating_sub(raw_tail_tokens)
        } else {
            0
        };
        breakdown.compaction_trigger_ratio_bps = Some(config.compaction.trigger_ratio_bps);
        breakdown.compaction_trigger_tokens = Some(trigger_tokens);
        breakdown.compaction_trigger_reached = prepared.compaction_trigger_reached;
        breakdown.input_budget_exceeded = input_budget_exceeded;
        breakdown.retained_history_ratio_bps = Some(config.compaction.retained_history_ratio_bps);
        breakdown.effective_retained_history_tokens = Some(retained_history_tokens);
        breakdown.current_session_history_budget_tokens = Some(history_budget_tokens);
        breakdown.history_source_message_count = Some(selection_source_message_count);
        breakdown.raw_tail_message_count = Some(raw_tail_message_count);
        breakdown.compressed_history_message_count = Some(compressed_history_message_count);
        breakdown.raw_tail_start_seq = tail_start_seq;
    }
    refresh_over_budget_reason(prepared);
}

fn refresh_over_budget_reason(prepared: &mut PreparedModelContext) {
    if !prepared.over_budget {
        prepared.over_budget_reason = None;
        return;
    }
    let Some(breakdown) = prepared.budget_breakdown.as_ref() else {
        prepared.over_budget_reason = Some("NON_COMPRESSIBLE_CONTEXT_OVER_EFFECTIVE_BUDGET".into());
        return;
    };
    if prompt_prefix_budget_exceeded(prepared) {
        prepared.over_budget_reason = Some("SEED_PROMPT_PREFIX_OVER_BUDGET".into());
        return;
    }
    let compressed_history_message_count = breakdown.compressed_history_message_count.unwrap_or(0);
    prepared.over_budget_reason = Some(
        if breakdown.input_budget_exceeded {
            if compressed_history_message_count > 0 {
                "COMPRESSED_HISTORY_OVER_EFFECTIVE_BUDGET"
            } else {
                "NON_COMPRESSIBLE_CONTEXT_OVER_EFFECTIVE_BUDGET"
            }
        } else {
            "NON_COMPRESSIBLE_CONTEXT_OVER_EFFECTIVE_BUDGET"
        }
        .to_string(),
    );
}

fn compaction_trigger_tokens(
    budget_plan: &ModelContextBudgetPlan,
    config: Option<&ShortTermMemoryConfig>,
) -> u32 {
    let ratio = config
        .map(|config| config.compaction.trigger_ratio_bps)
        .unwrap_or(10_000)
        .min(10_000);
    budget_plan.effective_input_budget.saturating_mul(ratio) / 10_000
}

fn prompt_prefix_budget_exceeded(prepared: &PreparedModelContext) -> bool {
    prepared
        .budget_breakdown
        .as_ref()
        .and_then(|breakdown| {
            breakdown
                .prompt_prefix_budget_tokens
                .map(|limit| breakdown.prompt_prefix_tokens > limit)
        })
        .unwrap_or(false)
}

fn current_session_history_budget_tokens(prepared: &PreparedModelContext) -> u32 {
    let Some(breakdown) = prepared.budget_breakdown.as_ref() else {
        return prepared.effective_input_budget.unwrap_or(0);
    };
    let effective_input_budget = breakdown.effective_input_budget.unwrap_or(0);
    let fixed_tokens = breakdown
        .prompt_prefix_tokens
        .saturating_add(breakdown.checkpoint_tokens)
        .saturating_add(breakdown.session_family_context_tokens)
        .saturating_add(breakdown.working_state_tokens)
        .saturating_add(breakdown.tool_schema_tokens);
    effective_input_budget.saturating_sub(fixed_tokens)
}
