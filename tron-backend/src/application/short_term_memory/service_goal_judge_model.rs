use serde_json::{Value, json};

use crate::{
    domain::{
        agent::{AgentMessage, AgentModelConfig, ModelFinish, ModelRequest, ModelResponse},
        short_term_memory::{ShortTermMemoryError, ShortTermMemoryGoal},
    },
    shared::time::current_timestamp,
};

use super::{
    GoalJudgeFailure, GoalJudgeOutput, GoalLoopDecision, PreparedGoalJudgeInput,
    ShortTermMemoryService,
    service_goal_judge_payload::{
        goal_judge_payload, hash_text, no_progress_candidate, no_progress_detected,
        no_progress_repeated_continuation_prompt, truncate_chars,
    },
    service_parts::estimate_text_tokens,
};

const RECENT_GOAL_EVENT_LIMIT: usize = 5;
const JUDGE_INPUT_PREVIEW_CHARS: usize = 2_000;

impl ShortTermMemoryService {
    pub fn prepare_goal_judge_input(
        &self,
        session_code: &str,
        run_code: &str,
        final_output: &str,
        step_count: u32,
    ) -> Result<Option<PreparedGoalJudgeInput>, ShortTermMemoryError> {
        let view = match self.repository().get_active_goal(session_code)? {
            Some(view) if view.goal.status == "ACTIVE" => view,
            _ => return Ok(None),
        };
        let expected_revision = view.goal.revision;
        let runs_used_after_judge = view.goal.runs_used.saturating_add(1);
        let todos = self.repository().list_todos(session_code)?;
        let inputs = self
            .repository()
            .list_inputs_for_run(session_code, run_code)?;
        let input_code = inputs.first().map(|input| input.input_code.clone());
        let messages = self
            .repository()
            .list_messages_for_run(session_code, run_code)?;
        let recent_events = self
            .repository()
            .list_goal_events(&view.goal.goal_code, RECENT_GOAL_EVENT_LIMIT)?;
        let final_output_hash = hash_text(final_output);
        let no_progress_candidate = no_progress_candidate(&todos, &messages);
        let no_progress_detected =
            no_progress_detected(no_progress_candidate, &recent_events, &final_output_hash);

        let judge_payload = goal_judge_payload(
            &view,
            &todos,
            &inputs,
            &messages,
            &recent_events,
            run_code,
            final_output,
            &final_output_hash,
            step_count,
        );
        let payload_text = serde_json::to_string_pretty(&judge_payload)
            .map_err(|_| ShortTermMemoryError::StorageFailure)?;
        let prompt = truncate_chars(&payload_text, self.config.goal_loop.judge_input_max_chars);
        let prompt_hash = hash_text(&prompt);
        let input_preview = json!({
            "promptPreview": truncate_chars(&prompt, JUDGE_INPUT_PREVIEW_CHARS),
            "payloadHash": prompt_hash,
            "finalOutputHash": final_output_hash,
            "runCode": run_code,
            "goalCode": view.goal.goal_code,
        });
        Ok(Some(PreparedGoalJudgeInput {
            session_code: session_code.to_string(),
            workspace_code: view.goal.workspace_code.clone(),
            goal_code: view.goal.goal_code,
            run_code: run_code.to_string(),
            input_code,
            expected_revision,
            runs_used_after_judge,
            max_runs: view.goal.max_runs,
            prompt,
            prompt_hash,
            final_output_hash,
            input_token_estimate: estimate_text_tokens(&payload_text),
            input_preview,
            no_progress_candidate,
            no_progress_detected,
        }))
    }

    pub fn goal_judge_model_config(&self, base: &AgentModelConfig) -> AgentModelConfig {
        let mut config = base.clone();
        config.max_tokens = Some(self.config.goal_loop.judge_output_max_tokens);
        config.temperature = Some(0.0);
        config.top_p = Some(1.0);
        config.thinking_enabled = false;
        config.responses_stateful_enabled = false;
        config
    }

    pub fn goal_judge_max_attempts_per_run(&self) -> u32 {
        self.config.goal_loop.judge_max_attempts_per_run.max(1)
    }

    pub fn goal_judge_model_request(&self, input: &PreparedGoalJudgeInput) -> ModelRequest {
        ModelRequest {
            messages: vec![
                AgentMessage::system(goal_judge_system_prompt()),
                AgentMessage::user(input.prompt.clone()),
            ],
            tools: Vec::new(),
            system_agent_session_id: None,
            prepared_tool_schema: None,
            response_input_items: None,
            previous_response_id: None,
        }
    }

    pub fn goal_judge_model_retry_request(
        &self,
        input: &PreparedGoalJudgeInput,
        failure: &GoalJudgeFailure,
    ) -> ModelRequest {
        ModelRequest {
            messages: vec![
                AgentMessage::system(goal_judge_retry_system_prompt()),
                AgentMessage::user(format!(
                    "Previous judge attempt failed with {error_code}: {message}\n\nRe-evaluate the same judge input below and return only a valid JSON object.\n\n{prompt}",
                    error_code = failure.error_code,
                    message = failure.message,
                    prompt = input.prompt,
                )),
            ],
            tools: Vec::new(),
            system_agent_session_id: None,
            prepared_tool_schema: None,
            response_input_items: None,
            previous_response_id: None,
        }
    }

    pub fn model_goal_judge_output(
        &self,
        response: &ModelResponse,
    ) -> Result<GoalJudgeOutput, GoalJudgeFailure> {
        if !response.tool_calls().is_empty() {
            return Err(GoalJudgeFailure {
                error_code: "GOAL_JUDGE_MODEL_TOOL_CALL".into(),
                message: "goal judge model returned tool calls".into(),
            });
        }
        if response.finish != ModelFinish::Final {
            return Err(GoalJudgeFailure {
                error_code: "GOAL_JUDGE_MODEL_NON_FINAL".into(),
                message: "goal judge model returned a non-final response".into(),
            });
        }
        let output_text = response.output_text();
        let json_text =
            extract_json_object(output_text.trim()).ok_or_else(|| GoalJudgeFailure {
                error_code: "GOAL_JUDGE_MODEL_INVALID_JSON".into(),
                message: "goal judge model did not return a JSON object".into(),
            })?;
        let mut output = serde_json::from_str::<GoalJudgeOutput>(json_text).map_err(|error| {
            GoalJudgeFailure {
                error_code: "GOAL_JUDGE_MODEL_INVALID_JSON".into(),
                message: format!("goal judge model returned invalid JSON: {error}"),
            }
        })?;
        output.verdict = output.verdict.trim().to_ascii_uppercase();
        output.reason = output.reason.trim().to_string();
        output.required_user_input = normalize_optional(output.required_user_input.take());
        output.next_step_hint = normalize_optional(output.next_step_hint.take());
        output.continuation_prompt = normalize_optional(output.continuation_prompt.take());
        validate_goal_judge_output(&output)?;
        Ok(output)
    }

    pub fn record_goal_judge_started(
        &self,
        input: &PreparedGoalJudgeInput,
        model: &AgentModelConfig,
    ) -> Result<(), ShortTermMemoryError> {
        let view = match self.repository().get_goal(&input.goal_code)? {
            Some(view) if view.goal.status == "ACTIVE" => view,
            _ => return Ok(()),
        };
        let event = self.new_goal_judge_event(
            &view.goal,
            input,
            "GOAL_JUDGE_STARTED",
            Some("Goal judge started"),
            Some(json!({
                "runCode": input.run_code,
                "judgeInput": input.input_preview,
                "judgeInputHash": input.prompt_hash,
                "inputTokenEstimate": input.input_token_estimate,
                "model": model_audit(model),
            })),
        )?;
        self.repository().append_goal_event(&event)
    }

    pub fn record_goal_judge_attempt_failed(
        &self,
        input: &PreparedGoalJudgeInput,
        attempt: u32,
        max_attempts: u32,
        failure: &GoalJudgeFailure,
        model: &AgentModelConfig,
    ) -> Result<(), ShortTermMemoryError> {
        let view = match self.repository().get_goal(&input.goal_code)? {
            Some(view) if view.goal.status == "ACTIVE" => view,
            _ => return Ok(()),
        };
        let event = self.new_goal_judge_event(
            &view.goal,
            input,
            "GOAL_JUDGE_ATTEMPT_FAILED",
            Some(failure.message.as_str()),
            Some(json!({
                "attempt": attempt,
                "maxAttempts": max_attempts,
                "failure": {
                    "errorCode": failure.error_code,
                    "message": failure.message,
                },
                "runCode": input.run_code,
                "judgeInputHash": input.prompt_hash,
                "model": model_audit(model),
            })),
        )?;
        self.repository().append_goal_event(&event)
    }

    pub fn record_goal_judge_failure(
        &self,
        input: PreparedGoalJudgeInput,
        failure: GoalJudgeFailure,
        model: &AgentModelConfig,
    ) -> Result<GoalLoopDecision, ShortTermMemoryError> {
        let mut view = self
            .repository()
            .get_goal(&input.goal_code)?
            .ok_or(ShortTermMemoryError::GoalNotFound)?;
        if view.goal.status != "ACTIVE" {
            return Ok(GoalLoopDecision {
                goal_code: view.goal.goal_code,
                verdict: "SKIPPED".into(),
                reason: "Goal is no longer active".into(),
                continuation_prompt: None,
            });
        }
        let now = current_timestamp();
        view.goal.runs_used = input.runs_used_after_judge;
        view.goal.revision = input.expected_revision.saturating_add(1);
        view.goal.last_run_code = Some(input.run_code.clone());
        view.goal.last_input_code = input.input_code.clone();
        view.goal.updated_at = now.clone();
        view.goal.consecutive_judge_failures =
            view.goal.consecutive_judge_failures.saturating_add(1);
        view.goal.last_verdict = Some("JUDGE_FAILED".into());
        view.goal.last_reason = Some(failure.message.clone());

        let threshold_reached = view.goal.consecutive_judge_failures
            >= self.config.goal_loop.max_consecutive_judge_failures;
        let budget_exhausted = view.goal.runs_used >= view.goal.max_runs;
        if threshold_reached {
            view.goal.status = "PAUSED".into();
            view.goal.paused_at = Some(now);
            view.goal.last_reason = Some("judge_unavailable".into());
        } else if budget_exhausted {
            view.goal.status = "PAUSED".into();
            view.goal.paused_at = Some(now);
            view.goal.last_reason = Some("Goal run budget was exhausted".into());
        }
        let event = self.new_goal_event(
            &view.goal,
            "JUDGE_FAILED",
            view.goal.last_reason.as_deref(),
            Some(json!({
                "failure": {
                    "errorCode": failure.error_code,
                    "message": failure.message,
                },
                "judgeFailureFallback": false,
                "thresholdReached": threshold_reached,
                "budgetExhausted": budget_exhausted,
                "runCode": input.run_code,
                "runsUsed": view.goal.runs_used,
                "maxRuns": view.goal.max_runs,
                "judgeInput": input.input_preview,
                "judgeInputHash": input.prompt_hash,
                "model": model_audit(model),
            })),
        )?;
        let updated = self.repository().update_goal(
            &view.goal,
            &view.criteria,
            &event,
            input.expected_revision,
        )?;
        Ok(GoalLoopDecision {
            goal_code: updated.goal.goal_code,
            verdict: updated
                .goal
                .last_verdict
                .unwrap_or_else(|| "JUDGE_FAILED".into()),
            reason: updated
                .goal
                .last_reason
                .unwrap_or_else(|| "goal judge failed".into()),
            continuation_prompt: None,
        })
    }

    pub fn commit_goal_judge_decision(
        &self,
        input: PreparedGoalJudgeInput,
        output: GoalJudgeOutput,
        model: &AgentModelConfig,
        response: &ModelResponse,
    ) -> Result<GoalLoopDecision, ShortTermMemoryError> {
        let mut view = self
            .repository()
            .get_goal(&input.goal_code)?
            .ok_or(ShortTermMemoryError::GoalNotFound)?;
        if view.goal.status != "ACTIVE" {
            return Ok(GoalLoopDecision {
                goal_code: view.goal.goal_code,
                verdict: "SKIPPED".into(),
                reason: "Goal is no longer active".into(),
                continuation_prompt: None,
            });
        }
        let now = current_timestamp();
        view.goal.runs_used = input.runs_used_after_judge;
        view.goal.revision = input.expected_revision.saturating_add(1);
        view.goal.last_run_code = Some(input.run_code.clone());
        view.goal.last_input_code = input.input_code.clone();
        view.goal.updated_at = now.clone();
        view.goal.last_verdict = Some(output.verdict.clone());
        view.goal.last_reason = Some(output.reason.clone());
        view.goal.consecutive_judge_failures = 0;

        let mut event_kind = "JUDGE_VERDICT";
        let mut continuation_prompt = None;
        let recent_events = if output.verdict == "CONTINUE" {
            self.repository()
                .list_goal_events(&input.goal_code, RECENT_GOAL_EVENT_LIMIT)?
        } else {
            Vec::new()
        };
        match output.verdict.as_str() {
            "COMPLETED" => {
                view.goal.status = "COMPLETED".into();
                view.goal.completed_at = Some(now);
                event_kind = "GOAL_COMPLETED";
            }
            "BLOCKED" | "NEEDS_USER_INPUT" => {
                view.goal.status = "BLOCKED".into();
                view.goal.blocked_at = Some(now);
                view.goal.blocked_reason = Some(output.reason.clone());
                if output.verdict == "NEEDS_USER_INPUT" {
                    view.goal.required_user_input = output
                        .required_user_input
                        .clone()
                        .or_else(|| Some(output.reason.clone()));
                }
                event_kind = "GOAL_BLOCKED";
            }
            "CONTINUE" => {
                let repeated_prompt = no_progress_repeated_continuation_prompt(
                    input.no_progress_candidate,
                    output.continuation_prompt.as_deref(),
                    &recent_events,
                );
                if input.no_progress_detected || repeated_prompt {
                    view.goal.status = "PAUSED".into();
                    view.goal.paused_at = Some(now);
                    view.goal.last_verdict = Some("SKIPPED".into());
                    view.goal.last_reason = Some("no_progress_detected".into());
                    event_kind = "NO_PROGRESS_DETECTED";
                } else if view.goal.runs_used >= view.goal.max_runs {
                    view.goal.status = "PAUSED".into();
                    view.goal.paused_at = Some(now);
                    view.goal.last_verdict = Some("SKIPPED".into());
                    view.goal.last_reason = Some("Goal run budget was exhausted".into());
                    event_kind = "BUDGET_EXHAUSTED";
                } else {
                    continuation_prompt = output.continuation_prompt.clone();
                }
            }
            _ => {}
        }
        let event = self.new_goal_event(
            &view.goal,
            event_kind,
            view.goal.last_reason.as_deref(),
            Some(json!({
                "verdict": view.goal.last_verdict,
                "runCode": input.run_code,
                "runsUsed": view.goal.runs_used,
                "maxRuns": view.goal.max_runs,
                "judgeInput": input.input_preview,
                "judgeInputHash": input.prompt_hash,
                "finalOutputHash": input.final_output_hash,
                "judgeOutput": output,
                "judgeOutputHash": hash_text(&response.output_text()),
                "continuationPromptHash": output.continuation_prompt.as_deref().map(hash_text),
                "noProgressDetected": input.no_progress_detected,
                "model": model_audit(model),
                "tokenUsage": response.token_usage(),
                "responseId": response.response_id(),
            })),
        )?;
        let updated = self.repository().update_goal(
            &view.goal,
            &view.criteria,
            &event,
            input.expected_revision,
        )?;
        Ok(GoalLoopDecision {
            goal_code: updated.goal.goal_code,
            verdict: updated
                .goal
                .last_verdict
                .unwrap_or_else(|| output.verdict.clone()),
            reason: updated.goal.last_reason.unwrap_or(output.reason),
            continuation_prompt,
        })
    }

    fn new_goal_judge_event(
        &self,
        goal: &ShortTermMemoryGoal,
        input: &PreparedGoalJudgeInput,
        event_kind: &str,
        summary: Option<&str>,
        payload_json: Option<Value>,
    ) -> Result<crate::domain::short_term_memory::ShortTermMemoryGoalEvent, ShortTermMemoryError>
    {
        let mut goal = goal.clone();
        goal.last_run_code = Some(input.run_code.clone());
        goal.last_input_code = input.input_code.clone();
        self.new_goal_event(&goal, event_kind, summary, payload_json)
    }
}

fn goal_judge_system_prompt() -> &'static str {
    crate::domain::prompt::builtin_prompt_text("memory.goal-judge-system", "en-US")
        .expect("goal judge prompt must be bundled")
}

fn goal_judge_retry_system_prompt() -> &'static str {
    crate::domain::prompt::builtin_prompt_text("memory.goal-judge-retry-system", "en-US")
        .expect("goal judge retry prompt must be bundled")
}

fn validate_goal_judge_output(output: &GoalJudgeOutput) -> Result<(), GoalJudgeFailure> {
    match output.verdict.as_str() {
        "CONTINUE" => {
            if output
                .continuation_prompt
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err(GoalJudgeFailure {
                    error_code: "GOAL_JUDGE_MODEL_SCHEMA_INVALID".into(),
                    message: "CONTINUE verdict requires continuationPrompt".into(),
                });
            }
        }
        "COMPLETED" | "BLOCKED" | "NEEDS_USER_INPUT" => {
            if output.continuation_prompt.is_some() {
                return Err(GoalJudgeFailure {
                    error_code: "GOAL_JUDGE_MODEL_SCHEMA_INVALID".into(),
                    message: "non-CONTINUE verdict requires null continuationPrompt".into(),
                });
            }
        }
        _ => {
            return Err(GoalJudgeFailure {
                error_code: "GOAL_JUDGE_MODEL_SCHEMA_INVALID".into(),
                message: "goal judge verdict is not supported".into(),
            });
        }
    }
    if output.reason.is_empty() {
        return Err(GoalJudgeFailure {
            error_code: "GOAL_JUDGE_MODEL_SCHEMA_INVALID".into(),
            message: "goal judge reason is required".into(),
        });
    }
    Ok(())
}

fn extract_json_object(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }
    if let Some(start) = trimmed.find('{')
        && let Some(end) = trimmed.rfind('}')
        && start < end
    {
        return Some(&trimmed[start..=end]);
    }
    None
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn model_audit(model: &AgentModelConfig) -> Value {
    json!({
        "modelCode": model.model_code,
        "modelName": model.model_name,
        "protocol": format!("{:?}", model.protocol),
    })
}
