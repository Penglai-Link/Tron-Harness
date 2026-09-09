use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    application::{
        prompt::PromptService,
        short_term_memory::{
            PreparedSessionSummaryJob, SessionSummaryFailure, SessionSummaryJobContext,
            SessionSummaryRunOutcome, ShortTermMemoryService,
        },
    },
    domain::prompt::builtin_prompt,
};

use super::super::ModelClient;
use super::{AgentInvocationError, helpers::safe_preview};

const SESSION_SUMMARY_PROMPT_KEY: &str = "memory.session-summary-system";

pub(super) fn completed_session_summary_outcome(
    awaiting_user_input: bool,
) -> SessionSummaryRunOutcome {
    SessionSummaryRunOutcome {
        status: if awaiting_user_input {
            "AWAITING_USER_INPUT".into()
        } else {
            "COMPLETED".into()
        },
        error_code: None,
        error_message: None,
    }
}

pub(super) fn failed_session_summary_outcome(
    error: &AgentInvocationError,
) -> SessionSummaryRunOutcome {
    SessionSummaryRunOutcome {
        status: if matches!(error, AgentInvocationError::RunStopped) {
            "CANCELLED".into()
        } else {
            "FAILED".into()
        },
        error_code: Some(error.code().into()),
        error_message: Some(safe_preview("diagnostic", &error.message())),
    }
}

pub(super) fn schedule_session_summary(
    short_term_memory: Option<ShortTermMemoryService>,
    prompt_service: Option<PromptService>,
    model_client: Arc<dyn ModelClient>,
    context: Option<SessionSummaryJobContext>,
    outcome: SessionSummaryRunOutcome,
) {
    let (Some(context), Some(short_term_memory)) = (context, short_term_memory) else {
        return;
    };
    let Ok(Some(job)) = short_term_memory.prepare_session_summary_job(context, outcome) else {
        return;
    };
    tokio::spawn(async move {
        run_session_summary_job(short_term_memory, prompt_service, model_client, job).await;
    });
}

async fn run_session_summary_job(
    short_term_memory: ShortTermMemoryService,
    prompt_service: Option<PromptService>,
    model_client: Arc<dyn ModelClient>,
    job: PreparedSessionSummaryJob,
) {
    let (system_prompt, prompt_version) = match resolve_session_summary_prompt(prompt_service, &job)
    {
        Ok(prompt) => prompt,
        Err(failure) => {
            let _ = short_term_memory.fail_session_summary(&job, None, failure);
            return;
        }
    };
    let input = match short_term_memory.session_summary_model_request(
        job.clone(),
        system_prompt,
        prompt_version,
    ) {
        Ok(Some(input)) => input,
        Ok(None) => return,
        Err(failure) => {
            let _ = short_term_memory.fail_session_summary(&job, None, failure);
            return;
        }
    };
    let model = short_term_memory.session_summary_model_config(&job.context.model);
    let response = match tokio::time::timeout(
        Duration::from_secs(30),
        model_client.generate(&model, &input.request),
    )
    .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => {
            let _ = short_term_memory.fail_session_summary(
                &job,
                Some(&input),
                session_summary_failure(
                    "SESSION_SUMMARY_MODEL_FAILED",
                    "session summary model request failed",
                ),
            );
            return;
        }
        Err(_) => {
            let _ = short_term_memory.fail_session_summary(
                &job,
                Some(&input),
                session_summary_failure(
                    "SESSION_SUMMARY_MODEL_TIMEOUT",
                    "session summary model request timed out after 30 seconds",
                ),
            );
            return;
        }
    };
    let output = match short_term_memory.model_session_summary_output(&response) {
        Ok(output) => output,
        Err(failure) => {
            let _ = short_term_memory.fail_session_summary(&job, Some(&input), failure);
            return;
        }
    };
    if short_term_memory
        .commit_session_summary(&input, output, &model)
        .is_err()
    {
        let _ = short_term_memory.fail_session_summary(
            &job,
            Some(&input),
            session_summary_failure(
                "SESSION_SUMMARY_STORE_FAILED",
                "session summary could not be stored",
            ),
        );
    }
}

fn resolve_session_summary_prompt(
    prompt_service: Option<PromptService>,
    job: &PreparedSessionSummaryJob,
) -> Result<(String, u64), SessionSummaryFailure> {
    let Some(prompt_service) = prompt_service else {
        return Err(session_summary_failure(
            "SESSION_SUMMARY_PROMPT_SERVICE_UNAVAILABLE",
            "session summary prompt service is unavailable",
        ));
    };
    let prompt = match prompt_service.runtime_prompt_text(
        Some(&job.context.workspace_code),
        SESSION_SUMMARY_PROMPT_KEY,
        &job.context.locale,
        &HashMap::new(),
    ) {
        Ok(Some(prompt)) => prompt,
        Ok(None) => {
            return Err(session_summary_failure(
                "SESSION_SUMMARY_PROMPT_DISABLED",
                "session summary prompt is disabled",
            ));
        }
        Err(_) => {
            return Err(session_summary_failure(
                "SESSION_SUMMARY_PROMPT_RESOLUTION_FAILED",
                "session summary prompt could not be resolved",
            ));
        }
    };
    let definition = builtin_prompt(SESSION_SUMMARY_PROMPT_KEY).ok_or_else(|| {
        session_summary_failure(
            "SESSION_SUMMARY_PROMPT_RESOLUTION_FAILED",
            "session summary prompt is not bundled",
        )
    })?;
    let version = prompt_service
        .workspace_detail(&job.context.workspace_code, &definition.prompt_code)
        .map(|resolved| resolved.version)
        .unwrap_or(definition.version);
    Ok((prompt, version))
}

fn session_summary_failure(code: &str, message: &str) -> SessionSummaryFailure {
    SessionSummaryFailure {
        error_code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{completed_session_summary_outcome, failed_session_summary_outcome};
    use crate::application::agent::AgentInvocationError;

    #[test]
    fn maps_all_run_terminal_outcomes_for_summary_metadata() {
        assert_eq!(completed_session_summary_outcome(false).status, "COMPLETED");
        assert_eq!(
            completed_session_summary_outcome(true).status,
            "AWAITING_USER_INPUT"
        );

        let failed =
            failed_session_summary_outcome(&AgentInvocationError::RuntimeFailed("boom".into()));
        assert_eq!(failed.status, "FAILED");
        assert_eq!(failed.error_code.as_deref(), Some("AGENT_RUNTIME_FAILED"));

        let cancelled = failed_session_summary_outcome(&AgentInvocationError::RunStopped);
        assert_eq!(cancelled.status, "CANCELLED");
        assert_eq!(cancelled.error_code.as_deref(), Some("AGENT_RUN_STOPPED"));
    }
}
