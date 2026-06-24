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
    ModelResponseTranscriptFacts, PreparedModelContext, ShortTermMemoryConfig,
    ShortTermMemoryRepository, selector::RecentTranscriptSelector,
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
                context_window_limit: context.model.context_window_tokens,
                effective_input_budget: None,
                output_token_reserve: context.model.max_tokens,
                protocol_overhead_reserve: None,
                safety_margin_tokens: None,
                over_budget: false,
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
        let keep_recent_tokens = self.config.compaction.keep_recent_tokens.min(
            budget_plan
                .block_budgets
                .current_session_recent_transcript
                .effective_limit_tokens,
        );
        let selector = RecentTranscriptSelector::new(
            &self.config.compaction,
            keep_recent_tokens,
            self.config.context.reasoning_summary_replay_enabled,
        );
        let selection = selector.select(recent_source);
        let family_context = self.load_session_family_context(
            &memory.session_code,
            &budget_plan,
            memory.todo_enabled,
        )?;
        let active_goal = self.load_active_goal_for_context(&memory.session_code)?;
        let mut prepared = assemble_context(
            &memory.prompt_prefix,
            checkpoint,
            family_context,
            &todos,
            active_goal.as_ref(),
            selection.tail_messages,
            memory.todo_enabled,
            &budget_plan,
            self.config.context.reasoning_summary_replay_enabled,
        );
        let tool_schema_token_estimate = estimate_tool_schema_tokens(&context.tools);
        prepared.estimated_tokens = prepared
            .estimated_tokens
            .saturating_add(tool_schema_token_estimate);
        prepared.tool_schema_token_estimate = tool_schema_token_estimate;
        prepared.context_window_limit = Some(budget_plan.model_context_window);
        prepared.effective_input_budget = Some(budget_plan.effective_input_budget);
        prepared.output_token_reserve = Some(budget_plan.output_token_reserve);
        prepared.protocol_overhead_reserve = Some(budget_plan.protocol_overhead_reserve);
        prepared.safety_margin_tokens = Some(budget_plan.safety_margin_tokens);
        prepared.over_budget = prepared.estimated_tokens > budget_plan.effective_input_budget;
        prepared.checkpoint_code = checkpoint_code;
        if prepared.over_budget && self.config.compaction.auto_enabled {
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
                    "estimatedTokens": prepared.estimated_tokens,
                    "toolSchemaTokenEstimate": tool_schema_token_estimate,
                    "maxOutputTokens": context.model.max_tokens,
                    "compactionMode": "model",
                }),
                None,
            )?;
            return Ok(prepared);
        }
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
                let include_raw_reasoning = self.config.compaction.reasoning_inclusion == "raw";
                transcript_part_from_agent_part(
                    &memory.session_code,
                    &context.workspace_code,
                    (index + 1) as i64,
                    part,
                    include_raw_reasoning,
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
