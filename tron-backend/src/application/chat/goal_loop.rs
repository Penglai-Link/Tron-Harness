use super::service::{ChatSendError, ChatService, GoalContinuationResult, paused_run_presentation};
use crate::{
    application::{
        agent::{
            AgentInvocationCommand, AgentInvocationError, AgentInvocationResult,
            AgentInvocationService,
        },
        short_term_memory::{GoalJudgeFailure, GoalSessionCommand, ShortTermMemoryService},
    },
    domain::{
        agent::{AgentModelConfig, EffectiveTool},
        chat::{
            ChatMessage, ChatMessageChannel, ChatMessageRole, ChatSession, ChatUserInputRequest,
        },
    },
    shared::time::current_timestamp,
};
use eliteforge_agent_framework::{self as fw, ContinuationPolicy};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
impl ChatService {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_goal_continuations(
        &self,
        session_code: &str,
        session: &ChatSession,
        effective_agent_code: &str,
        app_code: Option<String>,
        task_code: Option<String>,
        system_model_id: Option<String>,
        resolved_model: AgentModelConfig,
        tools: Vec<EffectiveTool>,
        user_code: Option<String>,
        locale: Option<String>,
        agent: AgentInvocationService,
        memory: ShortTermMemoryService,
        last_run: AgentInvocationResult,
        runtime_session_id: Option<String>,
        supports_user_input_requests: bool,
    ) -> Result<GoalContinuationResult, ChatSendError> {
        let initial_status = continuation_status(&last_run);
        if matches!(
            initial_status,
            fw::RunStatus::StepLimit | fw::RunStatus::ToolCallLimit
        ) {
            pause_goal(&memory, session_code)?;
            return Ok(empty_result());
        }
        if initial_status == fw::RunStatus::AwaitingInput {
            return Ok(empty_result());
        }
        let active = memory.get_active_goal(session_code).map_err(memory_error)?;
        if !active.is_some_and(|view| view.goal.status == "ACTIVE") {
            return Ok(empty_result());
        }
        // An execution without native conservative observations cannot safely
        // receive a fresh automatic allowance. A manual goal resume remains a
        // separate user admission through the existing product protocol.
        let Some(observation) = last_run.sdk_execution.clone() else {
            pause_goal(&memory, session_code)?;
            return Ok(empty_result());
        };
        let initial_execution: fw::RunExecutionSnapshot = match observation.try_into() {
            Ok(snapshot) => snapshot,
            Err(_) => {
                pause_goal(&memory, session_code)?;
                return Ok(empty_result());
            }
        };
        let previous_run = last_run.run_code.clone();
        let adapter = last_run.runtime_adapter_key.clone();
        let policy = Arc::new(GoalHarness {
            chat: self.clone(),
            session_code: session_code.into(),
            session: session.clone(),
            effective_agent_code: effective_agent_code.into(),
            app_code,
            task_code,
            system_model_id,
            resolved_model,
            tools,
            user_code,
            locale,
            agent: agent.clone(),
            memory,
            supports_user_input_requests,
            state: tokio::sync::Mutex::new(GoalState {
                last_run,
                runtime_session_id,
                messages: vec![],
                user_input_request: None,
            }),
            failure: Mutex::new(None),
        });
        let mut request = fw::RunRequest::new(format!("goal/{session_code}/{previous_run}"), "");
        let continuation = fw::ContinuationRequest {
            policy: policy.clone(),
            initial_state: json!({"runCode":previous_run}),
            initial_execution,
            max_rounds: 20,
        };
        if continuation.attach(&mut request).is_err() {
            pause_goal(&policy.memory, session_code)?;
            return Ok(empty_result());
        }
        let outcome = agent
            .run_goal_driver(request, session_code, &previous_run, &adapter)
            .await;
        if let Err(error) = outcome {
            if let Some(business) = policy
                .failure
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
            {
                return Err(business);
            }
            return Err(ChatSendError::Agent(error));
        }
        let mut state = policy.state.lock().await;
        Ok(GoalContinuationResult {
            messages: std::mem::take(&mut state.messages),
            user_input_request: state.user_input_request.take(),
            final_run: Some(state.last_run.clone()),
        })
    }
}
fn continuation_status(run: &AgentInvocationResult) -> fw::RunStatus {
    if let Some(native) = &run.native {
        use crate::domain::agent::AgentNativeRunStatus as Status;
        return match native.status {
            Status::Completed => fw::RunStatus::Completed,
            Status::AwaitingInput => fw::RunStatus::AwaitingInput,
            Status::StepLimit => fw::RunStatus::StepLimit,
            Status::ToolCallLimit => fw::RunStatus::ToolCallLimit,
        };
    }
    match run.status {
        "AWAITING_USER_INPUT" => fw::RunStatus::AwaitingInput,
        "STEP_LIMIT" => fw::RunStatus::StepLimit,
        "TOOL_CALL_LIMIT" => fw::RunStatus::ToolCallLimit,
        _ => fw::RunStatus::Completed,
    }
}
fn empty_result() -> GoalContinuationResult {
    GoalContinuationResult {
        messages: vec![],
        user_input_request: None,
        final_run: None,
    }
}
fn memory_error(error: impl std::fmt::Display) -> ChatSendError {
    ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
}
fn pause_goal(memory: &ShortTermMemoryService, session: &str) -> Result<(), ChatSendError> {
    if let Some(active) = memory.get_active_goal(session).map_err(memory_error)? {
        if active.goal.status == "ACTIVE" {
            memory
                .pause_goal(GoalSessionCommand {
                    session_code: session.into(),
                    goal_code: Some(active.goal.goal_code),
                })
                .map_err(memory_error)?;
        }
    }
    Ok(())
}
struct GoalState {
    last_run: AgentInvocationResult,
    runtime_session_id: Option<String>,
    messages: Vec<ChatMessage>,
    user_input_request: Option<ChatUserInputRequest>,
}
struct GoalHarness {
    chat: ChatService,
    session_code: String,
    session: ChatSession,
    effective_agent_code: String,
    app_code: Option<String>,
    task_code: Option<String>,
    system_model_id: Option<String>,
    resolved_model: AgentModelConfig,
    tools: Vec<EffectiveTool>,
    user_code: Option<String>,
    locale: Option<String>,
    agent: AgentInvocationService,
    memory: ShortTermMemoryService,
    supports_user_input_requests: bool,
    state: tokio::sync::Mutex<GoalState>,
    failure: Mutex<Option<ChatSendError>>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct GoalContinuationPlan {
    prompt: String,
    goal_code: String,
    verdict: String,
}
impl GoalHarness {
    fn native<T>(&self, outcome: Result<T, ChatSendError>) -> Result<T, fw::AgentError> {
        outcome.map_err(|error| {
            let stopped = matches!(
                &error,
                ChatSendError::Agent(AgentInvocationError::RunStopped)
            );
            let detail = format!("{error:?}");
            *self.failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
            if stopped {
                fw::AgentError::Cancelled
            } else {
                fw::AgentError::Run(detail)
            }
        })
    }
    async fn evaluate_goal(
        &self,
        parent: &fw::RunContext,
    ) -> Result<fw::ContinuationDecision, ChatSendError> {
        self.agent
            .ensure_continuation_active(&self.session_code, parent)
            .map_err(ChatSendError::Agent)?;
        let last_run = self.state.lock().await.last_run.clone();
        let judge_input = self
            .memory
            .prepare_goal_judge_input(
                &self.session_code,
                &last_run.run_code,
                &last_run.final_output,
                last_run.step_count,
            )
            .map_err(|error| {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
            })?;
        let Some(judge_input) = judge_input else {
            return Ok(eliteforge_agent_framework::ContinuationDecision::Complete);
        };
        let judge_model = self.memory.goal_judge_model_config(&self.resolved_model);
        self.memory
            .record_goal_judge_started(&judge_input, &judge_model)
            .map_err(|error| {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
            })?;
        let max_attempts = self.memory.goal_judge_max_attempts_per_run();
        let repair = std::sync::Arc::new(GoalJudgeRepair {
            memory: self.memory.clone(),
            input: judge_input.clone(),
            model: judge_model.clone(),
            max_attempts,
            last_failure: std::sync::Mutex::new(None),
        });
        let request = self.memory.goal_judge_model_request(&judge_input);
        let response = self
            .agent
            .generate_goal_judge_child(
                &judge_model,
                &request,
                repair.clone(),
                max_attempts as usize,
                parent,
            )
            .await;
        let decision = match response {
            Ok(response) => {
                let output = self
                    .memory
                    .model_goal_judge_output(&response)
                    .map_err(|e| {
                        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(e.message))
                    })?;
                self.memory
                    .commit_goal_judge_decision(
                        judge_input.clone(),
                        output,
                        &judge_model,
                        &response,
                    )
                    .map_err(|e| {
                        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(e.to_string()))
                    })?
            }
            Err(AgentInvocationError::RunStopped) => {
                return Err(ChatSendError::Agent(AgentInvocationError::RunStopped));
            }
            Err(error) => {
                if parent
                    .extensions
                    .get::<fw::ContinuationBudget>()
                    .is_some_and(|budget| budget.exhausted())
                    || parent
                        .usage()
                        .input_tokens
                        .saturating_add(parent.usage().output_tokens)
                        >= parent.token_ceiling
                {
                    return Err(ChatSendError::Agent(error));
                }
                let failure = repair
                    .last_failure
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| GoalJudgeFailure {
                        error_code: "GOAL_JUDGE_MODEL_REQUEST_FAILED".into(),
                        message: error.message(),
                    });
                self.memory
                    .record_goal_judge_failure(judge_input, failure, &judge_model)
                    .map_err(|e| {
                        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(e.to_string()))
                    })?
            }
        };
        let Some(prompt) = decision.continuation_prompt else {
            return Ok(eliteforge_agent_framework::ContinuationDecision::Complete);
        };
        if !self
            .memory
            .should_admit_goal_continuation(&self.session_code, &decision.goal_code)
            .map_err(|error| {
                ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
            })?
        {
            return Ok(eliteforge_agent_framework::ContinuationDecision::Complete);
        }

        Ok(fw::ContinuationDecision::Continue(
            serde_json::to_value(GoalContinuationPlan {
                prompt,
                goal_code: decision.goal_code,
                verdict: decision.verdict,
            })
            .map_err(memory_error)?,
        ))
    }
    async fn execute_goal(
        &self,
        next: Value,
        parent: &fw::RunContext,
    ) -> Result<fw::ContinuationStep, ChatSendError> {
        self.agent
            .ensure_continuation_active(&self.session_code, parent)
            .map_err(ChatSendError::Agent)?;
        let plan: GoalContinuationPlan = serde_json::from_value(next).map_err(memory_error)?;
        let mut runtime_session_id = self.state.lock().await.runtime_session_id.clone();
        let project_scope = self
            .chat
            .project_execution_scope(&self.session)
            .map_err(ChatSendError::Chat)?;
        let invocation_app_code = match project_scope.as_ref() {
            Some(scope) => scope.app_code.clone(),
            None => self.app_code.clone(),
        };
        let invocation = AgentInvocationCommand {
            run_code: None,
            prompt: Some(plan.prompt),
            workspace_code: Some(self.session.workspace_code.clone()),
            conversation_code: Some(self.session.session_code.clone()),
            agent_code: Some(self.effective_agent_code.clone()),
            system_agent_session_id: runtime_session_id
                .clone()
                .or(self.session.system_agent_session_id.clone()),
            app_code: invocation_app_code,
            execution_scope_override: project_scope,
            task_code: self.task_code.clone(),
            model: Some(self.resolved_model.clone()),
            system_model_id: self.system_model_id.clone(),
            limits: None,
            tools: self.tools.clone(),
            attachments: Vec::new(),
            page_context: None,
            supports_user_input_requests: self.supports_user_input_requests,
            transcript_input_kind: Some("CONTINUATION".into()),
            transcript_input_metadata: Some(json!({
                "goalCode": plan.goal_code,
                "source": "GOAL_LOOP",
                "previousVerdict": plan.verdict,
                "judgeFailureFallback": false,
            })),
        };
        let run = self
            .agent
            .invoke_continuation(
                invocation,
                self.user_code.clone(),
                self.locale.clone(),
                parent,
            )
            .await
            .map_err(ChatSendError::Agent)?;
        let status = continuation_status(&run);
        let (assistant_output, raw_user_input_request) = if status == fw::RunStatus::AwaitingInput {
            paused_run_presentation(&run)?
        } else {
            (run.final_output.clone(), None)
        };
        let channel = if self.task_code.is_some() {
            ChatMessageChannel::TaskAgent
        } else {
            ChatMessageChannel::MainAgent
        };
        let assistant_message = self.chat.create_message(
            &self.session_code,
            Some(run.run_code.clone()),
            ChatMessageRole::Assistant,
            channel,
            "Assistant",
            &assistant_output,
            Vec::new(),
            run.token_usage,
        )?;
        let user_input_request = if let Some(raw) = raw_user_input_request {
            Some(self.chat.persist_user_input_request(
                &self.session_code,
                &run,
                &assistant_message,
                raw,
            )?)
        } else {
            None
        };
        let updated_at = current_timestamp();
        if let Some(next_runtime_session_id) = run.system_agent_session_id.as_deref() {
            self.chat
                .repository()
                .upsert_runtime_session(
                    &self.session_code,
                    run.runtime_adapter_key.as_str(),
                    &self.effective_agent_code,
                    next_runtime_session_id,
                    Some(run.run_code.as_str()),
                    &updated_at,
                )
                .map_err(ChatSendError::Chat)?;
            runtime_session_id = Some(next_runtime_session_id.to_string());
        }
        self.chat
            .repository()
            .update_session_run(
                &self.session_code,
                &run.run_code,
                run.status,
                Some(&self.effective_agent_code),
                run.system_agent_session_id.as_deref(),
                &updated_at,
            )
            .map_err(ChatSendError::Chat)?;

        let content = run.final_output.clone().into();
        let state_value = json!({"runCode":run.run_code});
        let mut state = self.state.lock().await;
        state.last_run = run;
        state.runtime_session_id = runtime_session_id;
        state.messages.push(assistant_message);
        state.user_input_request = user_input_request;
        Ok(fw::ContinuationStep {
            state: state_value,
            content,
            interactions: vec![],
            status,
        })
    }
}
#[async_trait::async_trait]
impl ContinuationPolicy for GoalHarness {
    async fn evaluate(
        &self,
        _: &Value,
        parent: &fw::RunContext,
    ) -> Result<fw::ContinuationDecision, fw::AgentError> {
        self.native(self.evaluate_goal(parent).await)
    }
    async fn execute(
        &self,
        next: Value,
        parent: &fw::RunContext,
    ) -> Result<fw::ContinuationStep, fw::AgentError> {
        self.native(self.execute_goal(next, parent).await)
    }
    async fn budget_exhausted(&self, _: &Value, _: &fw::RunContext) -> Result<(), fw::AgentError> {
        self.native(pause_goal(&self.memory, &self.session_code))
    }
}
struct GoalJudgeRepair {
    memory: ShortTermMemoryService,
    input: crate::application::short_term_memory::PreparedGoalJudgeInput,
    model: AgentModelConfig,
    max_attempts: u32,
    last_failure: std::sync::Mutex<Option<GoalJudgeFailure>>,
}
#[async_trait::async_trait]
impl eliteforge_agent_framework::ModelJobRepair for GoalJudgeRepair {
    async fn validate(
        &self,
        response: &eliteforge_agent_core::ChatResponse,
        attempt: usize,
    ) -> Result<Option<eliteforge_agent_core::ChatRequest>, eliteforge_agent_core::AgentError> {
        let wire =
            eliteforge_agent_official_plugin_model_http::response_from_framework(response.clone())?;
        let response = crate::adapters::outbound::model::http_model_client::convert(&wire)
            .map_err(eliteforge_agent_core::AgentError::Run)?;
        match self.memory.model_goal_judge_output(&response) {
            Ok(_) => Ok(None),
            Err(failure) => {
                self.memory
                    .record_goal_judge_attempt_failed(
                        &self.input,
                        attempt as u32,
                        self.max_attempts,
                        &failure,
                        &self.model,
                    )
                    .map_err(|e| eliteforge_agent_core::AgentError::Run(e.to_string()))?;
                *self.last_failure.lock().unwrap() = Some(failure.clone());
                if attempt >= self.max_attempts as usize {
                    return Err(eliteforge_agent_core::AgentError::Run(failure.message));
                }
                let request = self
                    .memory
                    .goal_judge_model_retry_request(&self.input, &failure);
                let wire = crate::adapters::outbound::model::http_model_client::convert(&request)
                    .map_err(eliteforge_agent_core::AgentError::Run)?;
                let mut request =
                    eliteforge_agent_official_plugin_model_http::request_to_framework(wire)?;
                request.options.temperature = self.model.temperature;
                request.options.top_p = self.model.top_p;
                request.options.max_output_tokens = self.model.max_tokens;
                Ok(Some(request))
            }
        }
    }
}
