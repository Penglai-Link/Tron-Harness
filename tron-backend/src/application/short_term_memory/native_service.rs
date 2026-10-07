use super::*;
use crate::adapters::outbound::persistence::sqlite::framework_storage::blocking;
use crate::{
    application::agent::FrameworkHost,
    domain::{
        agent::{AgentMessage, AgentModelConfig, AgentRunContext},
        short_term_memory::*,
    },
    shared::time::{current_timestamp, timestamp_from_millis},
};
use eliteforge_agent_core as fw;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

#[derive(Clone)]
pub struct ShortTermMemoryService {
    pub(crate) host: FrameworkHost,
    config: ShortTermMemoryConfig,
}
fn error(e: fw::StateError) -> ShortTermMemoryError {
    match e {
        fw::StateError::NotFound => ShortTermMemoryError::SessionNotFound,
        fw::StateError::Conflict => ShortTermMemoryError::GoalStateConflict,
        fw::StateError::Busy => ShortTermMemoryError::RunConflict,
        fw::StateError::Invalid(_) => ShortTermMemoryError::GoalInvalid,
        _ => ShortTermMemoryError::StorageFailure,
    }
}
impl ShortTermMemoryService {
    pub fn with_host(host: FrameworkHost, config: ShortTermMemoryConfig) -> Self {
        Self { host, config }
    }
    pub(crate) fn framework_context_config(&self) -> &ShortTermMemoryConfig {
        &self.config
    }
    pub(crate) async fn framework(
        &self,
    ) -> Result<eliteforge_agent_framework::Framework, ShortTermMemoryError> {
        self.host
            .storage_framework()
            .await
            .map_err(|_| ShortTermMemoryError::StorageFailure)
    }
    pub(crate) fn ensure_session(&self, id: &str) -> Result<String, ShortTermMemoryError> {
        blocking(self.ensure_session_async(id))
    }
    async fn ensure_session_async(&self, id: &str) -> Result<String, ShortTermMemoryError> {
        self.framework()
            .await?
            .sessions()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?
            .get(id)
            .await
            .map_err(error)?;
        let id = id.to_owned();
        self.host
            .shared_database()
            .await
            .map_err(error)?
            .call(move |c| {
                c.query_row(
                    "SELECT workspace_code FROM workspace_session_links WHERE session_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
            })
            .await
            .map_err(|_| ShortTermMemoryError::StorageFailure)?
            .ok_or(ShortTermMemoryError::SessionNotFound)
    }
    /// Keep the native lifecycle check and synchronous business SQL on one task,
    /// so both operations use the host's connection and transaction binding.
    pub(crate) fn with_active_session_write<T: Send>(
        &self,
        id: &str,
        operation: impl FnOnce() -> Result<T, crate::domain::chat::ChatError> + Send,
    ) -> Result<T, crate::domain::chat::ChatError> {
        use crate::adapters::outbound::persistence::sqlite::framework_storage::chat_error;
        use crate::domain::chat::ChatError;
        let mut failure = None;
        let result = blocking(async {
            let framework = self
                .host
                .storage_framework()
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            let sessions = framework
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            let db = self.host.shared_database().await.map_err(chat_error)?;
            db.transaction(async {
                let record = sessions.get(id).await?;
                if record.status != fw::SessionStatus::Active {
                    failure = Some(ChatError::SessionArchived);
                    return Err(fw::StateError::Conflict);
                }
                operation().map_err(|error| {
                    failure = Some(error);
                    fw::StateError::Backend("business session write failed".into())
                })
            })
            .await
            .map_err(chat_error)
        });
        match failure {
            Some(error) => Err(error),
            None => result,
        }
    }
    pub fn record_run_failed(
        &self,
        context: &AgentRunContext,
        reason: &str,
    ) -> Result<(), ShortTermMemoryError> {
        if let Some(session) = &context.conversation_code {
            self.append_display_event(
                session,
                Some(&context.run_code),
                "tron.execution-diagnostic",
                json!({"reason":super::redact_sensitive_text(reason)}),
            )
        } else {
            Ok(())
        }
    }
    pub(crate) fn append_display_event(
        &self,
        session: &str,
        run: Option<&str>,
        kind: &str,
        payload: Value,
    ) -> Result<(), ShortTermMemoryError> {
        blocking(async {
            let mut event = fw::SessionEvent::new(session, kind, payload);
            event.run_id = run.and_then(|s| fw::RunId::parse_str(s).ok());
            self.framework()
                .await?
                .sessions()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .store
                .append_event(event)
                .await
                .map_err(error)?;
            Ok(())
        })
    }
    fn read_context(&self, id: &str, key: &str) -> Result<Option<Value>, ShortTermMemoryError> {
        blocking(async {
            Ok(self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .read_context(id, key)
                .await
                .map_err(error)?
                .map(|(_, v)| v))
        })
    }
    pub fn list_todos(&self, id: &str) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
        self.ensure_session(id)?;
        self.read_context(id, "tron.todos")?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| ShortTermMemoryError::StorageFailure)
            .map(Option::unwrap_or_default)
    }
    pub fn replace_todos(
        &self,
        context: &AgentRunContext,
        items: Vec<TodoUpdateItem>,
    ) -> Result<Vec<ShortTermMemoryTodo>, ShortTermMemoryError> {
        let memory = context
            .short_term_memory
            .as_ref()
            .filter(|m| m.todo_enabled)
            .ok_or(ShortTermMemoryError::TodoInvalid)?;
        let workspace = self.ensure_session(&memory.session_code)?;
        if workspace != context.workspace_code || items.len() > 50 {
            return Err(ShortTermMemoryError::TodoInvalid);
        }
        let mut positions = std::collections::BTreeSet::new();
        let mut codes = std::collections::BTreeSet::new();
        let mut active = 0;
        let now = current_timestamp();
        let previous = blocking(async {
            self.framework()
                .await?
                .sessions()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .read_context(&memory.session_code, "tron.todos")
                .await
                .map_err(error)
        })?;
        let expected = previous.as_ref().map(|(r, _)| *r);
        let old: Vec<ShortTermMemoryTodo> = previous
            .map(|(_, value)| serde_json::from_value(value))
            .transpose()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?
            .unwrap_or_default();
        let todos = items
            .into_iter()
            .map(|item| {
                let content = item.content.trim().to_owned();
                if content.is_empty()
                    || content.chars().count() > 500
                    || item.position < 0
                    || !positions.insert(item.position)
                {
                    return Err(ShortTermMemoryError::TodoInvalid);
                }
                let status = TodoStatus::from_code(&item.status)?;
                active += usize::from(status == TodoStatus::InProgress);
                if active > 1 {
                    return Err(ShortTermMemoryError::TodoInvalid);
                }
                let code = item
                    .todo_code
                    .unwrap_or_else(|| fw::RunId::new_v4().to_string());
                if !codes.insert(code.clone()) {
                    return Err(ShortTermMemoryError::TodoInvalid);
                }
                let prior = old.iter().find(|t| t.todo_code == code);
                Ok(ShortTermMemoryTodo {
                    id: 0,
                    todo_code: code,
                    session_code: memory.session_code.clone(),
                    workspace_code: workspace.clone(),
                    position: item.position,
                    content,
                    status,
                    priority: TodoPriority::from_code(&item.priority)?,
                    created_by_run_code: prior
                        .and_then(|t| t.created_by_run_code.clone())
                        .or_else(|| Some(context.run_code.clone())),
                    updated_by_run_code: Some(context.run_code.clone()),
                    created_at: prior.map(|t| t.created_at.clone()).unwrap_or(now.clone()),
                    updated_at: now.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        blocking(async {
            let framework = self.framework().await?;
            let sessions = framework
                .sessions()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?;
            let db = self.host.shared_database().await.map_err(error)?;
            let value =
                serde_json::to_value(&todos).map_err(|_| ShortTermMemoryError::StorageFailure)?;
            db.transaction(async {
                sessions
                    .write_context(&memory.session_code, "tron.todos", expected, value.clone())
                    .await?;
                let mut event = fw::SessionEvent::new(
                    &memory.session_code,
                    "tron.todo.updated",
                    json!({"todos":value}),
                );
                event.run_id = fw::RunId::parse_str(&context.run_code).ok();
                sessions.store.append_event(event).await?;
                Ok(())
            })
            .await
            .map_err(error)?;
            Ok::<_, ShortTermMemoryError>(())
        })?;
        Ok(todos)
    }
    async fn view(
        &self,
        goal: fw::GoalRecord,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        let workspace = self.ensure_session_async(&goal.session_id).await?;
        let created = timestamp_from_millis(goal.created_at_ms as i64);
        let updated = timestamp_from_millis(goal.updated_at_ms as i64);
        let status = match goal.status {
            fw::GoalStatus::Active => "ACTIVE",
            fw::GoalStatus::Paused => "PAUSED",
            fw::GoalStatus::Blocked => "BLOCKED",
            fw::GoalStatus::Completed => "COMPLETED",
            fw::GoalStatus::Cancelled => "CANCELLED",
            fw::GoalStatus::Cleared => "CLEARED",
        };
        let criteria = goal
            .criteria
            .iter()
            .enumerate()
            .map(|(position, c)| ShortTermMemoryGoalCriterion {
                id: 0,
                criterion_code: c.id.clone(),
                goal_code: goal.id.clone(),
                session_code: goal.session_id.clone(),
                position: position as i64,
                text: c.text.clone(),
                status: if c.completed { "COMPLETED" } else { "PENDING" }.into(),
                created_at: created.clone(),
                updated_at: updated.clone(),
                removed_at: None,
            })
            .collect();
        Ok(ShortTermMemoryGoalView {
            goal: ShortTermMemoryGoal {
                id: 0,
                goal_code: goal.id,
                session_code: goal.session_id,
                workspace_code: workspace,
                user_code: None,
                task_code: None,
                active_family_code: None,
                objective_markdown: goal.objective,
                status: status.into(),
                revision: goal.revision as i64,
                runs_used: goal.runs_used,
                max_runs: goal.max_runs,
                last_run_code: goal.last_run_id.map(|r| r.to_string()),
                last_input_code: None,
                last_verdict: goal.last_decision.as_ref().map(|d| {
                    match d.verdict {
                        fw::GoalVerdict::Continue => "CONTINUE",
                        fw::GoalVerdict::Completed => "COMPLETED",
                        fw::GoalVerdict::Blocked => "BLOCKED",
                        fw::GoalVerdict::NeedsUserInput => "NEEDS_USER_INPUT",
                    }
                    .into()
                }),
                last_reason: goal.reason.clone(),
                blocked_reason: (goal.status == fw::GoalStatus::Blocked)
                    .then(|| goal.reason.clone())
                    .flatten(),
                required_user_input: goal.required_input,
                consecutive_judge_failures: goal.consecutive_failures,
                created_by_run_code: None,
                created_at: created,
                updated_at: updated.clone(),
                paused_at: (goal.status == fw::GoalStatus::Paused).then(|| updated.clone()),
                blocked_at: (goal.status == fw::GoalStatus::Blocked).then(|| updated.clone()),
                completed_at: (goal.status == fw::GoalStatus::Completed).then(|| updated.clone()),
                cancelled_at: (goal.status == fw::GoalStatus::Cancelled).then(|| updated.clone()),
                cleared_at: (goal.status == fw::GoalStatus::Cleared).then(|| updated.clone()),
                metadata_json: Some(json!({"progress":goal.progress})),
            },
            criteria,
        })
    }
    pub fn get_active_goal(
        &self,
        id: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        self.ensure_session(id)?;
        blocking(async {
            let goal = self
                .framework()
                .await?
                .goals()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .active(id)
                .await
                .map_err(error)?;
            match goal {
                Some(g) => Ok(Some(self.view(g).await?)),
                None => Ok(None),
            }
        })
    }
    pub fn get_latest_goal(
        &self,
        id: &str,
    ) -> Result<Option<ShortTermMemoryGoalView>, ShortTermMemoryError> {
        self.ensure_session(id)?;
        blocking(async {
            let goal = self
                .framework()
                .await?
                .goals()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .list(id)
                .await
                .map_err(error)?
                .into_iter()
                .max_by_key(|g| (g.created_at_ms, g.id.clone()));
            match goal {
                Some(g) => Ok(Some(self.view(g).await?)),
                None => Ok(None),
            }
        })
    }
    pub fn set_goal(
        &self,
        command: SetGoalCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.set_goal_with_operator(command, None)
    }
    pub fn set_goal_with_operator(
        &self,
        command: SetGoalCommand,
        user: Option<String>,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.ensure_session(&command.session_code)?;
        blocking(async {
            let framework = self.framework().await?;
            let goals = framework
                .goals()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?;
            let db = self.host.shared_database().await.map_err(error)?;
            let mut goal = fw::GoalRecord::new(&command.session_code, command.objective.trim());
            goal.max_runs = command.max_runs.unwrap_or(self.config.goal_loop.max_runs);
            let goal = db
                .transaction(async {
                    let goal = goals.set(goal).await?;
                    let mut event = fw::SessionEvent::new(
                        &command.session_code,
                        "tron.goal.operator",
                        json!({"userCode":user}),
                    );
                    event.goal_id = Some(goal.id.clone());
                    framework
                        .sessions()
                        .map_err(|e| fw::StateError::Backend(e.to_string()))?
                        .store
                        .append_event(event)
                        .await?;
                    Ok(goal)
                })
                .await
                .map_err(error)?;
            self.view(goal).await
        })
    }
    async fn resolve_goal(
        &self,
        command: &GoalSessionCommand,
    ) -> Result<fw::GoalRecord, ShortTermMemoryError> {
        self.ensure_session_async(&command.session_code).await?;
        let goals = self
            .framework()
            .await?
            .goals()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
        let goal = if let Some(id) = &command.goal_code {
            goals.get(id).await.map_err(error)?
        } else {
            goals
                .active(&command.session_code)
                .await
                .map_err(error)?
                .ok_or(ShortTermMemoryError::GoalNotFound)?
        };
        if goal.session_id != command.session_code {
            return Err(ShortTermMemoryError::GoalNotFound);
        }
        Ok(goal)
    }
    fn transition(
        &self,
        command: GoalSessionCommand,
        status: fw::GoalStatus,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        blocking(async {
            let goal = self.resolve_goal(&command).await?;
            let result = self
                .framework()
                .await?
                .goals()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .transition(&goal.id, goal.revision, status, None)
                .await
                .map_err(error)?;
            self.view(result).await
        })
    }
    pub fn pause_goal(
        &self,
        c: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition(c, fw::GoalStatus::Paused)
    }
    pub fn resume_goal(
        &self,
        c: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition(c, fw::GoalStatus::Active)
    }
    pub fn cancel_goal(
        &self,
        c: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition(c, fw::GoalStatus::Cancelled)
    }
    pub fn clear_goal(
        &self,
        c: GoalSessionCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        self.transition(c, fw::GoalStatus::Cleared)
    }
    pub fn update_goal(
        &self,
        c: UpdateGoalCommand,
    ) -> Result<ShortTermMemoryGoalView, ShortTermMemoryError> {
        blocking(async {
            let mut goal = self
                .resolve_goal(&GoalSessionCommand {
                    session_code: c.session_code,
                    goal_code: Some(c.goal_code),
                })
                .await?;
            let revision = goal.revision;
            goal.objective = c.objective.trim().into();
            if let Some(max) = c.max_runs {
                goal.max_runs = max;
            }
            let goal = self
                .framework()
                .await?
                .goals()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .update(goal, revision)
                .await
                .map_err(error)?;
            self.view(goal).await
        })
    }
    pub fn should_admit_goal_continuation(
        &self,
        id: &str,
        goal: &str,
    ) -> Result<bool, ShortTermMemoryError> {
        Ok(self
            .get_active_goal(id)?
            .is_some_and(|g| g.goal.goal_code == goal && g.goal.status == "ACTIVE"))
    }
    pub fn compaction_model_config(&self, model: &AgentModelConfig) -> AgentModelConfig {
        let mut model = model.clone();
        model.temperature = Some(0.0);
        model
    }
    pub(crate) fn model_context_budget_plan(
        &self,
        model: &AgentModelConfig,
    ) -> ModelContextBudgetPlan {
        self.config
            .model_context_budget_plan(model.context_window_tokens, model.max_tokens)
    }
    pub fn overflow_retry_limit(&self) -> u32 {
        1
    }
    pub(crate) fn framework_data_source(
        &self,
        context: &AgentRunContext,
        id: &str,
    ) -> Result<(Vec<AgentMessage>, Option<String>), ShortTermMemoryError> {
        let memory = context
            .short_term_memory
            .as_ref()
            .ok_or(ShortTermMemoryError::SessionNotFound)?;
        if self.ensure_session(&memory.session_code)? != context.workspace_code {
            return Err(ShortTermMemoryError::SessionNotFound);
        }
        let entries = match id {
            "current_session_working_state" => {
                let todos = if memory.todo_enabled {
                    self.list_todos(&memory.session_code)?
                } else {
                    vec![]
                };
                let goal = self.get_active_goal(&memory.session_code)?;
                super::assembler::working_state_message(
                    goal.as_ref(),
                    &todos,
                    memory.todo_enabled,
                    u32::MAX,
                )
                .into_iter()
                .collect()
            }
            "current_session_checkpoint" => vec![],
            "session_family_context" => self.family_context(context)?,
            _ => return Err(ShortTermMemoryError::StorageFailure),
        };
        Ok((entries, None))
    }
    pub(crate) fn prepare_model_context_with_tool_schema_estimate(
        &self,
        context: &AgentRunContext,
        tools: u32,
    ) -> Result<PreparedModelContext, ShortTermMemoryError> {
        let plan = self.config.model_context_budget_plan(
            context.model.context_window_tokens,
            context.model.max_tokens,
        );
        let tokens = estimate_messages_tokens(&context.messages);
        let mut prepared = PreparedModelContext {
            sources: None,
            messages: context.messages.clone(),
            rebased: false,
            context_hash: None,
            checkpoint_code: None,
            estimated_tokens: tokens.saturating_add(tools),
            message_token_estimate: tokens,
            tool_schema_token_estimate: tools,
            budget_breakdown: None,
            over_budget_reason: None,
            context_window_limit: Some(plan.model_context_window),
            effective_input_budget: Some(plan.effective_input_budget),
            output_token_reserve: Some(plan.output_token_reserve),
            over_budget: false,
            compaction_trigger_reached: false,
            compaction_required: false,
            compaction_tail_start_seq: None,
            compaction_trigger_tokens: None,
            effective_retained_history_tokens: None,
        };
        if let Some(memory) = &context.short_term_memory {
            let (working, _) =
                self.framework_data_source(context, "current_session_working_state")?;
            let (family, _) = self.framework_data_source(context, "session_family_context")?;
            prepared.sources = Some(PreparedContextSources {
                revision: eliteforge_agent_memory::MemoryRevision {
                    scope: eliteforge_agent_memory::MemoryScope::new(
                        &context.workspace_code,
                        &memory.session_code,
                    )
                    .map_err(|_| ShortTermMemoryError::StorageFailure)?,
                    generation: "native-history".into(),
                    revision: 0,
                },
                prefix: memory.prompt_prefix.clone(),
                checkpoint: vec![],
                family,
                working,
                history: vec![],
            });
        }
        Ok(prepared)
    }
    pub(crate) fn record_framework_context(
        &self,
        context: &AgentRunContext,
        prepared: &PreparedModelContext,
        report: &fw::ContextAssemblyReport,
    ) -> Result<(), ShortTermMemoryError> {
        let Some(session) = &context.conversation_code else {
            return Ok(());
        };
        self.append_display_event(session,Some(&context.run_code),"tron.context-view",json!({"messages":prepared.messages,"budgetBreakdown":prepared.budget_breakdown,"contextReport":report,"tools":context.tools,"executionScope":context.execution_scope}))
    }
    pub(crate) fn record_framework_memory_reduction(
        &self,
        context: &AgentRunContext,
        block: &str,
        audit: &crate::domain::agent::TokenUsageAuditSnapshot,
    ) -> Result<(), ShortTermMemoryError> {
        if let Some(session) = &context.conversation_code {
            self.append_display_event(
                session,
                Some(&context.run_code),
                "tron.context-reduction-audit",
                json!({"blockId":block,"usageAudit":audit}),
            )
        } else {
            Ok(())
        }
    }
    pub fn context_detail(
        &self,
        id: &str,
        block: &str,
    ) -> Result<Option<Value>, ShortTermMemoryError> {
        self.ensure_session(id)?;
        blocking(async {
            let events = self
                .framework()
                .await?
                .events()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?;
            let mut after = 0;
            let mut last = None;
            loop {
                let page = events.list(id, after, 1000).await.map_err(error)?;
                if page.is_empty() {
                    break;
                }
                after = page.last().unwrap().sequence;
                for event in page {
                    if event.kind == "tron.context-view" {
                        last = Some(event.payload);
                    }
                }
            }
            let Some(view) = last else { return Ok(None) };
            if block == "toolSchema" {
                return Ok(Some(
                    json!({"blockCode":block,"tools":view["tools"],"budgetBreakdown":view["budgetBreakdown"]}),
                ));
            }
            let ids: &[&str] = match block {
                "systemPrompt" => &["seed_prompt_prefix", fw::INSTRUCTIONS_BLOCK],
                "applicationContext" => &["application_context"],
                "familyContext" => &["session_family_context"],
                _ => return Err(ShortTermMemoryError::GoalInvalid),
            };
            let messages = view["messages"].as_array().cloned().unwrap_or_default();
            let mut selected = Vec::new();
            let mut offset = 0;
            let mut blocks = Vec::new();
            if let Some(reports) = view["contextReport"]["blocks"].as_array() {
                let mut reports = reports.iter().collect::<Vec<_>>();
                reports.sort_by_key(|report| report["position"].as_u64().unwrap_or(0));
                for report in reports {
                    let count = report["final_message_count"].as_u64().unwrap_or(0) as usize;
                    if report["id"].as_str().is_some_and(|id| ids.contains(&id)) {
                        selected.extend(messages.iter().skip(offset).take(count).cloned());
                        blocks.push(report.clone());
                    }
                    offset = offset.saturating_add(count);
                }
            }
            Ok(Some(
                json!({"blockCode":block,"messages":selected,"blocks":blocks,"budgetBreakdown":view["budgetBreakdown"],"executionScope":view["executionScope"]}),
            ))
        })
    }
    pub fn latest_compaction(&self, session: &str) -> Result<Option<Value>, ShortTermMemoryError> {
        self.ensure_session(session)?;
        self.read_context(session, "tron.session-summary-view")
    }
    pub async fn summarize(
        &self,
        session: &str,
        model: &AgentModelConfig,
        client: std::sync::Arc<dyn crate::application::agent::ModelClient>,
        reason: Option<String>,
        note: Option<String>,
    ) -> Result<Value, ShortTermMemoryError> {
        self.ensure_session_async(session).await?;
        let framework = self.framework().await?;
        let services = framework
            .services()
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
        let conversations = services
            .get::<fw::ConversationStoreService>()
            .ok_or(ShortTermMemoryError::StorageFailure)?;
        let snapshot = conversations
            .0
            .history(session)
            .await
            .map_err(|_| ShortTermMemoryError::StorageFailure)?
            .into_iter()
            .find(|s| s.key.branch == "main")
            .ok_or(ShortTermMemoryError::CompactionConflict)?;
        if snapshot.messages.is_empty() {
            return Err(ShortTermMemoryError::CompactionConflict);
        }
        let source = crate::application::agent::framework_runtime::summary_loop::SummarySource {
            snapshot,
            store: self
                .host
                .summary_store()
                .await
                .map_err(|_| ShortTermMemoryError::StorageFailure)?,
            lifecycle: framework
                .sessions()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?
                .store,
            database: self.host.shared_database().await.map_err(error)?,
            options: eliteforge_agent_memory::CompressionOptions {
                max_input_tokens: self.model_context_budget_plan(model).effective_input_budget
                    as usize,
                max_summary_bytes: 16 * 1024,
                max_model_calls: 16,
            },
            reason,
            note,
        };
        let selection = eliteforge_agent_framework::AgentSelection {
            agent_loop: Some("tron.summary-loop".into()),
            external_agent: Some("framework.external-agent".into()),
            context_assembler: Some("tron.driver.context".into()),
            tool_policies: Some(vec![]),
            tool_providers: Some(vec![]),
            ..Default::default()
        };
        let agent = framework.agent(selection).build().await.map_err(|e| {
            eprintln!("summary agent binding failed: {e}");
            ShortTermMemoryError::StorageFailure
        })?;
        let mut request = fw::RunRequest::new(session, "");
        request.extensions.insert(source);
        crate::application::agent::framework_runtime::attach_auxiliary_model(
            &mut request,
            model,
            client,
        )
        .map_err(|e| {
            eprintln!("summary model binding failed: {e}");
            ShortTermMemoryError::StorageFailure
        })?;
        let result = agent.run(request).await;
        let cleanup = agent.close().await;
        if !cleanup.errors.is_empty() {
            eprintln!("summary Agent cleanup failed: {:?}", cleanup.errors);
        }
        let result = result.map_err(|e| {
            eprintln!("native summary failed: {e}");
            match e.primary() {
                fw::AgentError::SessionBusy(_) => ShortTermMemoryError::RunConflict,
                _ => ShortTermMemoryError::StorageFailure,
            }
        })?;
        result
            .structured
            .ok_or(ShortTermMemoryError::StorageFailure)
    }
    fn family_context(
        &self,
        context: &AgentRunContext,
    ) -> Result<Vec<AgentMessage>, ShortTermMemoryError> {
        // Only related sessions in the same workspace are authorized as family data.
        blocking(async {
            let db = self.host.shared_database().await.map_err(error)?;
            let workspace = context.workspace_code.clone();
            let current = context.conversation_code.clone();
            let app = context.execution_scope.app_code.clone();
            let task = context.execution_scope.task_code.clone();
            let links=db.call(move|c|{let mut q=c.prepare("SELECT session_id,business_json FROM workspace_session_links WHERE workspace_code=?1")?;q.query_map([workspace],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<Result<Vec<_>,_>>()}).await.map_err(|_|ShortTermMemoryError::StorageFailure)?;
            let sessions = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ShortTermMemoryError::StorageFailure)?;
            let mut result = vec![];
            for (id, link) in links {
                if Some(&id) == current.as_ref() {
                    continue;
                }
                let link: Value = serde_json::from_str(&link)
                    .map_err(|_| ShortTermMemoryError::StorageFailure)?;
                if !(task
                    .as_deref()
                    .is_some_and(|v| link["task_code"].as_str() == Some(v))
                    || app
                        .as_deref()
                        .is_some_and(|v| link["app_code"].as_str() == Some(v)))
                {
                    continue;
                }
                if let Some((_, summary)) = sessions
                    .read_context(&id, "tron.session-summary-view")
                    .await
                    .map_err(error)?
                {
                    result.push(AgentMessage::context_block(
                        "session_family_context",
                        summary.to_string(),
                    ));
                }
                if result.len() >= self.config.family_load.default_max_sessions as usize {
                    break;
                }
            }
            Ok(result)
        })
    }
}
