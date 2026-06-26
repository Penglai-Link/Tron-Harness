use serde_json::json;

use crate::{
    application::{
        agent::{
            AgentInvocationCommand, AgentInvocationError, AgentInvocationResult,
            AgentInvocationService,
        },
        short_term_memory::{GoalJudgeFailure, ShortTermMemoryService},
    },
    domain::{
        agent::{AgentModelConfig, EffectiveTool},
        chat::{ChatMessage, ChatMessageChannel, ChatMessageRole, ChatSession},
    },
    shared::time::current_timestamp,
};

use super::service::{ChatSendError, ChatService};

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
        short_term_memory: ShortTermMemoryService,
        mut last_run: AgentInvocationResult,
        mut runtime_session_id: Option<String>,
    ) -> Result<Vec<ChatMessage>, ChatSendError> {
        let mut messages = Vec::new();
        for _ in 0..20 {
            let judge_input = short_term_memory
                .prepare_goal_judge_input(
                    session_code,
                    &last_run.run_code,
                    &last_run.final_output,
                    last_run.step_count,
                )
                .map_err(|error| {
                    ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
                })?;
            let Some(judge_input) = judge_input else {
                break;
            };
            let judge_model = short_term_memory.goal_judge_model_config(&resolved_model);
            short_term_memory
                .record_goal_judge_started(&judge_input, &judge_model)
                .map_err(|error| {
                    ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
                })?;
            let max_attempts = short_term_memory.goal_judge_max_attempts_per_run();
            let mut decision = None;
            let mut last_failure = None;
            for attempt in 1..=max_attempts {
                let judge_request = if attempt == 1 {
                    short_term_memory.goal_judge_model_request(&judge_input)
                } else {
                    let failure = last_failure.as_ref().ok_or_else(|| {
                        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                            "goal judge retry missing previous failure".into(),
                        ))
                    })?;
                    short_term_memory.goal_judge_model_retry_request(&judge_input, failure)
                };
                let attempt_failure = match agent
                    .generate_stateless_model(&judge_model, &judge_request)
                    .await
                {
                    Ok(response) => match short_term_memory.model_goal_judge_output(&response) {
                        Ok(output) => {
                            decision = Some(
                                short_term_memory
                                    .commit_goal_judge_decision(
                                        judge_input.clone(),
                                        output,
                                        &judge_model,
                                        &response,
                                    )
                                    .map_err(|error| {
                                        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                                            error.to_string(),
                                        ))
                                    })?,
                            );
                            break;
                        }
                        Err(failure) => failure,
                    },
                    Err(error) => GoalJudgeFailure {
                        error_code: "GOAL_JUDGE_MODEL_REQUEST_FAILED".into(),
                        message: error.message(),
                    },
                };
                short_term_memory
                    .record_goal_judge_attempt_failed(
                        &judge_input,
                        attempt,
                        max_attempts,
                        &attempt_failure,
                        &judge_model,
                    )
                    .map_err(|error| {
                        ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
                    })?;
                last_failure = Some(attempt_failure);
            }
            let decision = match decision {
                Some(decision) => decision,
                None => {
                    let failure = last_failure.unwrap_or_else(|| GoalJudgeFailure {
                        error_code: "GOAL_JUDGE_MODEL_UNKNOWN_FAILURE".into(),
                        message: "goal judge did not produce a decision".into(),
                    });
                    short_term_memory
                        .record_goal_judge_failure(judge_input, failure, &judge_model)
                        .map_err(|error| {
                            ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                                error.to_string(),
                            ))
                        })?
                }
            };
            let Some(prompt) = decision.continuation_prompt else {
                break;
            };
            if !short_term_memory
                .should_admit_goal_continuation(session_code, &decision.goal_code)
                .map_err(|error| {
                    ChatSendError::Agent(AgentInvocationError::RuntimeFailed(error.to_string()))
                })?
            {
                break;
            }
            let invocation = AgentInvocationCommand {
                run_code: None,
                prompt: Some(prompt),
                workspace_code: Some(session.workspace_code.clone()),
                conversation_code: Some(session.session_code.clone()),
                agent_code: Some(effective_agent_code.to_string()),
                system_agent_session_id: runtime_session_id
                    .clone()
                    .or(session.system_agent_session_id.clone()),
                app_code: app_code.clone(),
                task_code: task_code.clone(),
                model: Some(resolved_model.clone()),
                system_model_id: system_model_id.clone(),
                limits: None,
                tools: tools.clone(),
                attachments: Vec::new(),
                transcript_input_kind: Some("CONTINUATION".into()),
                transcript_input_metadata: Some(json!({
                    "goalCode": decision.goal_code,
                    "source": "GOAL_LOOP",
                    "previousVerdict": decision.verdict,
                    "judgeFailureFallback": false,
                })),
            };
            let run = agent
                .invoke(invocation, user_code.clone(), locale.clone())
                .await
                .map_err(ChatSendError::Agent)?;
            let channel = if task_code.is_some() {
                ChatMessageChannel::TaskAgent
            } else {
                ChatMessageChannel::MainAgent
            };
            let assistant_message = self.create_message(
                session_code,
                Some(run.run_code.clone()),
                ChatMessageRole::Assistant,
                channel,
                "Assistant",
                &run.final_output,
                Vec::new(),
                run.token_usage,
            )?;
            let updated_at = current_timestamp();
            if let Some(next_runtime_session_id) = run.system_agent_session_id.as_deref() {
                self.repository()
                    .upsert_runtime_session(
                        session_code,
                        run.runtime_adapter_key.as_str(),
                        effective_agent_code,
                        next_runtime_session_id,
                        Some(run.run_code.as_str()),
                        &updated_at,
                    )
                    .map_err(ChatSendError::Chat)?;
                runtime_session_id = Some(next_runtime_session_id.to_string());
            }
            self.repository()
                .update_session_run(
                    session_code,
                    &run.run_code,
                    run.status,
                    Some(effective_agent_code),
                    run.system_agent_session_id.as_deref(),
                    &updated_at,
                )
                .map_err(ChatSendError::Chat)?;
            last_run = run;
            messages.push(assistant_message);
        }
        Ok(messages)
    }
}
