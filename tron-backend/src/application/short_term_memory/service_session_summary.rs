use serde_json::{Value, json};

use crate::{
    domain::{
        agent::{AgentMessage, AgentModelConfig, ModelFinish, ModelRequest, ModelResponse},
        short_term_memory::{ShortTermMemoryError, ShortTermMemoryFamilySnapshot},
    },
    shared::time::current_timestamp,
};

use super::{
    ModelSessionSummaryOutput, PreparedSessionSummaryInput, PreparedSessionSummaryJob,
    SessionSummaryFailure, SessionSummaryJobContext, SessionSummaryRunOutcome,
    ShortTermMemoryService,
    assembler::estimate_messages_tokens,
    service_family_support::session_summary_snapshot_code,
    service_parts::estimate_text_tokens,
    service_session_summary_support::{
        hash_text, redact_sensitive_text, session_summary_payload, strip_outer_markdown_fence,
        summary_failure, truncate_chars, truncate_markdown,
    },
};

pub const SESSION_SUMMARY_OUTPUT_MAX_TOKENS: u32 = 1_024;
pub(super) const TOOL_RESULT_PREVIEW_CHARS: usize = 480;
pub(super) const FILE_REFERENCE_PREVIEW_CHARS: usize = 512;
pub(super) const ERROR_MESSAGE_MAX_CHARS: usize = 512;

impl ShortTermMemoryService {
    pub fn prepare_session_summary_job(
        &self,
        context: SessionSummaryJobContext,
        outcome: SessionSummaryRunOutcome,
    ) -> Result<Option<PreparedSessionSummaryJob>, ShortTermMemoryError> {
        let snapshot_code = session_summary_snapshot_code(&context.session_code, &context.run_code);
        let created_at = current_timestamp();
        let mut repository = self.repository();
        let workspace_code = repository
            .session_workspace(&context.session_code)?
            .ok_or(ShortTermMemoryError::SessionNotFound)?;
        if workspace_code != context.workspace_code {
            return Err(ShortTermMemoryError::StorageFailure);
        }
        if repository
            .session_summary_for_run(&context.session_code, &context.run_code)?
            .is_some()
        {
            return Ok(None);
        }
        let snapshot = ShortTermMemoryFamilySnapshot {
            id: repository.next_id("agent_session_family_snapshots")?,
            snapshot_code: snapshot_code.clone(),
            family_code: None,
            session_code: context.session_code.clone(),
            run_code: Some(context.run_code.clone()),
            snapshot_kind: "SESSION_SUMMARY".into(),
            status: "PENDING".into(),
            source_start_seq: None,
            source_end_seq: None,
            source_checkpoint_message_code: None,
            summary_markdown: None,
            token_estimate: None,
            error_code: None,
            error_message: None,
            metadata_json: Some(summary_metadata(
                &outcome,
                json!({
                    "stage": "PENDING",
                    "source": "run_history_persisted",
                }),
            )),
            created_at: created_at.clone(),
            completed_at: None,
            expires_at: None,
        };
        repository.create_family_snapshots(&[snapshot])?;
        Ok(Some(PreparedSessionSummaryJob {
            context,
            outcome,
            snapshot_code,
            created_at,
        }))
    }

    pub fn session_summary_model_config(&self, base: &AgentModelConfig) -> AgentModelConfig {
        let mut config = base.clone();
        config.temperature = Some(0.0);
        config.thinking_enabled = false;
        config.reasoning_effort = None;
        config.responses_stateful_enabled = false;
        config.max_tokens = Some(SESSION_SUMMARY_OUTPUT_MAX_TOKENS);
        config.max_retries = Some(0);
        config
    }

    pub fn session_summary_model_request(
        &self,
        job: PreparedSessionSummaryJob,
        system_prompt: String,
        prompt_version: u64,
    ) -> Result<Option<PreparedSessionSummaryInput>, SessionSummaryFailure> {
        let mut repository = self.repository();
        let run_messages = repository
            .list_messages_for_run(&job.context.session_code, &job.context.run_code)
            .map_err(|_| {
                summary_failure(
                    "SESSION_SUMMARY_SOURCE_READ_FAILED",
                    "could not read the current run transcript",
                )
            })?;
        let previous = repository
            .latest_completed_session_summary(&job.context.session_code)
            .map_err(|_| {
                summary_failure(
                    "SESSION_SUMMARY_SOURCE_READ_FAILED",
                    "could not read the previous session summary",
                )
            })?;
        let source_end_seq = run_messages
            .last()
            .map(|(message, _)| message.seq)
            .or_else(|| {
                previous
                    .as_ref()
                    .and_then(|snapshot| snapshot.source_end_seq)
            })
            .unwrap_or(0);
        if previous
            .as_ref()
            .and_then(|snapshot| snapshot.source_end_seq)
            .is_some_and(|previous_end| previous_end > source_end_seq)
        {
            drop(repository);
            self.expire_session_summary(&job, source_end_seq)
                .map_err(|_| {
                    summary_failure(
                        "SESSION_SUMMARY_STORE_FAILED",
                        "could not mark a superseded session summary",
                    )
                })?;
            return Ok(None);
        }
        let previous_end_seq = previous
            .as_ref()
            .and_then(|snapshot| snapshot.source_end_seq);
        let source_start_seq = previous_end_seq.map_or(0, |seq| seq.saturating_add(1));
        let messages = if source_start_seq <= source_end_seq {
            repository
                .list_messages_after_seq(&job.context.session_code, source_start_seq)
                .map_err(|_| {
                    summary_failure(
                        "SESSION_SUMMARY_SOURCE_READ_FAILED",
                        "could not read the session summary range",
                    )
                })?
                .into_iter()
                .filter(|(message, _)| message.seq <= source_end_seq)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let goal = repository
            .get_latest_goal(&job.context.session_code)
            .map_err(|_| {
                summary_failure(
                    "SESSION_SUMMARY_SOURCE_READ_FAILED",
                    "could not read the current goal",
                )
            })?;
        let todos = repository
            .list_todos(&job.context.session_code)
            .map_err(|_| {
                summary_failure(
                    "SESSION_SUMMARY_SOURCE_READ_FAILED",
                    "could not read current todos",
                )
            })?;
        drop(repository);

        let previous_snapshot_code = previous
            .as_ref()
            .map(|snapshot| snapshot.snapshot_code.clone());
        let payload = session_summary_payload(
            &job,
            previous.as_ref(),
            &messages,
            goal.as_ref(),
            &todos,
            source_start_seq,
            source_end_seq,
        );
        let payload_text = serde_json::to_string_pretty(&payload).map_err(|_| {
            summary_failure(
                "SESSION_SUMMARY_INPUT_SERIALIZE_FAILED",
                "could not serialize session summary evidence",
            )
        })?;
        let prompt_hash = hash_text(&system_prompt);
        let request = ModelRequest {
            messages: vec![
                AgentMessage::system(system_prompt),
                AgentMessage::user(payload_text),
            ],
            tools: Vec::new(),
            system_agent_session_id: None,
            prepared_tool_schema: None,
            response_input_items: None,
            previous_response_id: None,
        };
        let input_token_estimate = estimate_messages_tokens(&request.messages);
        let max_input_tokens = self
            .config
            .model_context_budget_plan(
                job.context.model.context_window_tokens,
                Some(SESSION_SUMMARY_OUTPUT_MAX_TOKENS),
            )
            .effective_input_budget;
        let in_progress = session_summary_snapshot(
            &job,
            "IN_PROGRESS",
            Some(source_start_seq),
            Some(source_end_seq),
            None,
            None,
            None,
            Some(summary_metadata(
                &job.outcome,
                json!({
                    "stage": "IN_PROGRESS",
                    "source": "session_transcript",
                    "previousSnapshotCode": previous_snapshot_code,
                    "promptCatalogKey": "memory.session-summary-system",
                    "promptHash": prompt_hash,
                    "promptVersion": prompt_version,
                    "inputTokenEstimate": input_token_estimate,
                    "maxInputTokens": max_input_tokens,
                }),
            )),
            None,
        );
        self.repository()
            .create_family_snapshots(&[in_progress])
            .map_err(|_| {
                summary_failure(
                    "SESSION_SUMMARY_STORE_FAILED",
                    "could not start the session summary",
                )
            })?;
        self.append_event(
            &job.context.session_code,
            &job.context.workspace_code,
            Some(&job.context.run_code),
            None,
            None,
            None,
            "agent.session_summary.started",
            "SESSION_SUMMARY",
            json!({
                "runCode": job.context.run_code,
                "sessionCode": job.context.session_code,
                "sourceStartSeq": source_start_seq,
                "sourceEndSeq": source_end_seq,
                "inputTokenEstimate": input_token_estimate,
                "promptCatalogKey": "memory.session-summary-system",
                "promptHash": prompt_hash,
                "promptVersion": prompt_version,
            }),
            None,
        )
        .map_err(|_| {
            summary_failure(
                "SESSION_SUMMARY_EVENT_WRITE_FAILED",
                "could not record session summary start",
            )
        })?;

        if input_token_estimate > max_input_tokens {
            return Err(summary_failure(
                "SESSION_SUMMARY_INPUT_TOO_LARGE",
                "session summary input exceeds the model context budget",
            ));
        }
        Ok(Some(PreparedSessionSummaryInput {
            job,
            source_start_seq,
            source_end_seq,
            previous_snapshot_code,
            prompt_hash,
            prompt_version,
            input_token_estimate,
            max_input_tokens,
            request,
        }))
    }

    pub fn model_session_summary_output(
        &self,
        response: &ModelResponse,
    ) -> Result<ModelSessionSummaryOutput, SessionSummaryFailure> {
        if !response.tool_calls().is_empty() {
            return Err(summary_failure(
                "SESSION_SUMMARY_MODEL_TOOL_CALL",
                "session summary model returned tool calls",
            ));
        }
        if response.finish != ModelFinish::Final {
            return Err(summary_failure(
                "SESSION_SUMMARY_MODEL_NON_FINAL",
                "session summary model returned a non-final response",
            ));
        }
        let markdown = strip_outer_markdown_fence(response.output_text().trim())
            .trim()
            .to_string();
        if markdown.is_empty() {
            return Err(summary_failure(
                "SESSION_SUMMARY_MODEL_EMPTY_OUTPUT",
                "session summary model returned empty output",
            ));
        }
        let markdown = truncate_markdown(
            &markdown,
            self.config.family_load.default_snapshot_max_chars,
        );
        Ok(ModelSessionSummaryOutput {
            markdown,
            token_usage: response.token_usage(),
            provider_metadata: Some(json!({"responseId": response.response_id()})),
        })
    }

    pub fn commit_session_summary(
        &self,
        input: &PreparedSessionSummaryInput,
        output: ModelSessionSummaryOutput,
        model: &AgentModelConfig,
    ) -> Result<(), ShortTermMemoryError> {
        let completed_at = current_timestamp();
        let summary_token_estimate = estimate_text_tokens(&output.markdown);
        let snapshot = session_summary_snapshot(
            &input.job,
            "COMPLETED",
            Some(input.source_start_seq),
            Some(input.source_end_seq),
            Some(output.markdown),
            Some(summary_token_estimate),
            None,
            Some(summary_metadata(
                &input.job.outcome,
                json!({
                    "stage": "COMPLETED",
                    "source": "session_transcript",
                    "previousSnapshotCode": input.previous_snapshot_code,
                    "promptCatalogKey": "memory.session-summary-system",
                    "promptHash": input.prompt_hash,
                    "promptVersion": input.prompt_version,
                    "inputTokenEstimate": input.input_token_estimate,
                    "maxInputTokens": input.max_input_tokens,
                    "summaryTokenEstimate": summary_token_estimate,
                    "tokenUsage": output.token_usage,
                    "providerMetadata": output.provider_metadata,
                    "model": {
                        "code": model.model_code,
                        "name": model.model_name,
                        "protocol": format!("{:?}", model.protocol),
                    },
                }),
            )),
            Some(completed_at.clone()),
        );
        self.repository().create_family_snapshots(&[snapshot])?;
        let _ = self.append_event(
            &input.job.context.session_code,
            &input.job.context.workspace_code,
            Some(&input.job.context.run_code),
            None,
            None,
            None,
            "agent.session_summary.completed",
            "SESSION_SUMMARY",
            json!({
                "runCode": input.job.context.run_code,
                "sessionCode": input.job.context.session_code,
                "sourceStartSeq": input.source_start_seq,
                "sourceEndSeq": input.source_end_seq,
                "summaryTokenEstimate": summary_token_estimate,
                "promptHash": input.prompt_hash,
                "promptVersion": input.prompt_version,
                "completedAt": completed_at,
            }),
            None,
        );
        Ok(())
    }

    pub fn fail_session_summary(
        &self,
        job: &PreparedSessionSummaryJob,
        input: Option<&PreparedSessionSummaryInput>,
        failure: SessionSummaryFailure,
    ) -> Result<(), ShortTermMemoryError> {
        let completed_at = current_timestamp();
        let error_message = truncate_chars(&failure.message, ERROR_MESSAGE_MAX_CHARS);
        let snapshot = session_summary_snapshot(
            job,
            "FAILED",
            input.map(|value| value.source_start_seq),
            input.map(|value| value.source_end_seq),
            None,
            None,
            Some((failure.error_code.as_str(), error_message.as_str())),
            Some(summary_metadata(
                &job.outcome,
                json!({
                    "stage": "FAILED",
                    "promptHash": input.map(|value| value.prompt_hash.as_str()),
                    "promptVersion": input.map(|value| value.prompt_version),
                    "inputTokenEstimate": input.map(|value| value.input_token_estimate),
                    "errorCode": failure.error_code,
                }),
            )),
            Some(completed_at.clone()),
        );
        self.repository().create_family_snapshots(&[snapshot])?;
        let _ = self.append_event(
            &job.context.session_code,
            &job.context.workspace_code,
            Some(&job.context.run_code),
            None,
            None,
            None,
            "agent.session_summary.failed",
            "SESSION_SUMMARY",
            json!({
                "runCode": job.context.run_code,
                "sessionCode": job.context.session_code,
                "sourceStartSeq": input.map(|value| value.source_start_seq),
                "sourceEndSeq": input.map(|value| value.source_end_seq),
                "errorCode": failure.error_code,
                "errorMessage": error_message,
                "completedAt": completed_at,
            }),
            None,
        );
        Ok(())
    }

    pub fn recover_session_summaries_on_startup(&self) -> Result<usize, ShortTermMemoryError> {
        self.repository()
            .recover_stale_session_summaries(&current_timestamp())
    }

    fn expire_session_summary(
        &self,
        job: &PreparedSessionSummaryJob,
        source_end_seq: i64,
    ) -> Result<(), ShortTermMemoryError> {
        let snapshot = session_summary_snapshot(
            job,
            "EXPIRED",
            None,
            Some(source_end_seq),
            None,
            None,
            None,
            Some(summary_metadata(
                &job.outcome,
                json!({
                    "stage": "EXPIRED",
                    "reason": "newer_session_summary_already_completed",
                }),
            )),
            Some(current_timestamp()),
        );
        self.repository().create_family_snapshots(&[snapshot])
    }
}

#[allow(clippy::too_many_arguments)]
fn session_summary_snapshot(
    job: &PreparedSessionSummaryJob,
    status: &str,
    source_start_seq: Option<i64>,
    source_end_seq: Option<i64>,
    summary_markdown: Option<String>,
    token_estimate: Option<u32>,
    failure: Option<(&str, &str)>,
    metadata_json: Option<Value>,
    completed_at: Option<String>,
) -> ShortTermMemoryFamilySnapshot {
    ShortTermMemoryFamilySnapshot {
        id: 0,
        snapshot_code: job.snapshot_code.clone(),
        family_code: None,
        session_code: job.context.session_code.clone(),
        run_code: Some(job.context.run_code.clone()),
        snapshot_kind: "SESSION_SUMMARY".into(),
        status: status.into(),
        source_start_seq,
        source_end_seq,
        source_checkpoint_message_code: None,
        summary_markdown,
        token_estimate,
        error_code: failure.map(|(code, _)| code.to_string()),
        error_message: failure.map(|(_, message)| message.to_string()),
        metadata_json,
        created_at: job.created_at.clone(),
        completed_at,
        expires_at: None,
    }
}

fn summary_metadata(outcome: &SessionSummaryRunOutcome, stage: Value) -> Value {
    let mut value = json!({
        "strategyVersion": "session-summary-v1",
        "rawReasoningOmitted": true,
        "rawToolArgumentsOmitted": true,
        "fullToolOutputOmitted": true,
        "runOutcome": {
            "status": outcome.status,
            "errorCode": outcome.error_code,
            "errorMessage": outcome.error_message.as_deref().map(|message| {
                truncate_chars(&redact_sensitive_text(message), ERROR_MESSAGE_MAX_CHARS)
            }),
        },
    });
    if let (Some(target), Some(source)) = (value.as_object_mut(), stage.as_object()) {
        target.extend(source.clone());
    }
    value
}
