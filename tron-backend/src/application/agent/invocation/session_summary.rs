use std::sync::Arc;

use crate::application::{
    prompt::PromptService,
    short_term_memory::{
        SessionSummaryJobContext, SessionSummaryRunOutcome, ShortTermMemoryService,
    },
};

use super::super::ModelClient;
use super::{AgentInvocationError, helpers::safe_preview};

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
    _prompt_service: Option<PromptService>,
    model_client: Arc<dyn ModelClient>,
    context: Option<SessionSummaryJobContext>,
    outcome: SessionSummaryRunOutcome,
) {
    let (Some(context), Some(short_term_memory)) = (context, short_term_memory) else {
        return;
    };
    if outcome.status != "COMPLETED" {
        return;
    }
    crate::app::telemetry::spawn_traced(async move {
        if let Err(error) = short_term_memory
            .summarize(
                &context.session_code,
                &context.model,
                model_client,
                Some("RunCompleted".into()),
                None,
            )
            .await
        {
            tracing::error!(
                "session summary failed for {}: {error:?}",
                context.session_code
            );
        }
    });
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
