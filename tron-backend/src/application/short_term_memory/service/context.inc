impl ShortTermMemoryService {
    pub fn context_detail(
        &self,
        session_code: &str,
        block_code: &str,
    ) -> Result<Option<serde_json::Value>, ShortTermMemoryError> {
        let snapshot = self.repository().latest_context_epoch_snapshot(session_code)?;
        let detail = match (block_code, snapshot.as_ref()) {
            ("systemPrompt", Some(snapshot)) => snapshot.get("promptPrefix").cloned(),
            ("toolSchema", Some(snapshot)) => snapshot
                .get("exposedTools")
                .or_else(|| snapshot.get("tools"))
                .cloned(),
            ("familyContext", Some(snapshot)) => snapshot.get("familyContext").cloned(),
            (_, None) => None,
            _ => None,
        };
        Ok(detail.map(enrich_builtin_prompt_sources))
    }

    pub fn prepare_model_context(
        &self,
        context: &AgentRunContext,
    ) -> Result<PreparedModelContext, ShortTermMemoryError> {
        let schema = crate::application::agent::ToolSchemaCache::default()
            .prepare(context.model.protocol, &context.tools, u32::MAX)
            .map_err(|_| ShortTermMemoryError::ContextBudgetExceeded)?;
        self.prepare_model_context_with_tool_schema_estimate(context, schema.estimated_tokens())
    }

    pub(crate) fn model_context_budget_plan(
        &self,
        model: &crate::domain::agent::AgentModelConfig,
    ) -> super::ModelContextBudgetPlan {
        self.config
            .model_context_budget_plan(model.context_window_tokens, model.max_tokens)
    }

    pub(crate) fn prepare_model_context_with_tool_schema_estimate(
        &self,
        context: &AgentRunContext,
        tool_schema_token_estimate: u32,
    ) -> Result<PreparedModelContext, ShortTermMemoryError> {
        let budget_plan = self.model_context_budget_plan(&context.model);
        let Some(memory) = context.short_term_memory.as_ref() else {
            let mut prepared = PreparedModelContext {
                messages: context.messages.clone(),
                rebased: false,
                context_hash: None,
                checkpoint_code: None,
                estimated_tokens: estimate_messages_tokens(&context.messages),
                message_token_estimate: estimate_messages_tokens(&context.messages),
                tool_schema_token_estimate,
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
            };
            apply_request_budget_fields(
                &mut prepared,
                &budget_plan,
                &self.config,
                tool_schema_token_estimate,
            );
            return Ok(prepared);
        };
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
        if tool_schema_token_estimate > budget_plan.block_budgets.tool_schema.effective_limit_tokens
        {
            return Ok(full_prepared);
        }
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
            &prepared,
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
}

fn enrich_builtin_prompt_sources(mut detail: serde_json::Value) -> serde_json::Value {
    let Some(messages) = detail
        .get_mut("messages")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return detail;
    };

    for message in messages {
        let Some(message_object) = message.as_object_mut() else {
            continue;
        };
        if message_object.contains_key("promptCode") {
            continue;
        }
        let Some(content) = message_object.get("content").and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some((definition, locale)) = crate::domain::prompt::builtin_prompts()
            .iter()
            .find_map(|definition| {
                definition.translations.iter().find_map(|(locale, translation)| {
                    (translation.prompt_body == content).then_some((definition, locale))
                })
            })
        else {
            continue;
        };
        message_object.insert(
            "catalogKey".to_string(),
            serde_json::Value::String(definition.catalog_key.clone()),
        );
        message_object.insert(
            "promptCode".to_string(),
            serde_json::Value::String(definition.prompt_code.clone()),
        );
        message_object.insert(
            "locale".to_string(),
            serde_json::Value::String(locale.clone()),
        );
    }

    detail
}
