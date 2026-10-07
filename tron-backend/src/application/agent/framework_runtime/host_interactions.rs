//! Native interaction validation remains authoritative; the host only maps product answers.
use super::{FrameworkHost, map_error};
use crate::{
    application::chat::ChatResumeEntry,
    domain::agent::{AgentError, AgentMessagePart},
};
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::Agent;
use serde_json::Value;
impl FrameworkHost {
    pub async fn pending_native_interactions(
        &self,
        session: &str,
    ) -> Result<Vec<fw::InteractionRecord>, AgentError> {
        let _admission = self.0.config_lock.lock().await;
        self.ensure_open()
            .map_err(AgentError::InvalidConfiguration)?;
        self.agent()
            .await
            .map_err(map_error)?
            .pending_interactions(session)
            .await
            .map_err(map_error)
    }
    pub async fn validate_native_resume(
        &self,
        session: &str,
        resume: &[ChatResumeEntry],
    ) -> Result<(), AgentError> {
        let pending = self.pending_native_interactions(session).await?;
        if resume.is_empty() {
            if pending.is_empty() {
                return Ok(());
            }
            return Err(AgentError::AwaitingUserInput);
        }
        let native = resume.iter().collect::<Vec<_>>();
        let _admission = self.0.config_lock.lock().await;
        self.ensure_open()
            .map_err(AgentError::InvalidConfiguration)?;
        let agent = self.agent().await.map_err(map_error)?;
        let mut logical_run = None;
        let mut ids = std::collections::BTreeSet::new();
        for entry in native {
            let id = native_id(&entry.interrupt_id)?;
            if !ids.insert(id) {
                return Err(invalid("duplicate interaction response"));
            }
            let record = agent.interaction(session, id).await.map_err(map_error)?;
            if logical_run
                .replace(record.run_id)
                .is_some_and(|previous| previous != record.run_id)
            {
                return Err(invalid("responses belong to different logical runs"));
            }
            let value = answer_value(&record.request, &entry.payload)?;
            validate_record_response(&record, value)?;
        }
        if let Some(run) = logical_run {
            ensure_waiting(agent, session, run).await?;
        }
        Ok(())
    }
}
pub(super) fn native_id(value: &str) -> Result<fw::RunId, AgentError> {
    fw::RunId::parse_str(value).map_err(|_| invalid("invalid native interaction identity"))
}
pub(super) fn answer_value(
    request: &fw::InteractionRequest,
    payload: &Value,
) -> Result<Value, AgentError> {
    match request.kind {
        fw::InteractionKind::Approval { .. } if !payload.is_boolean() => {
            Err(invalid("approval response must be a boolean"))
        }
        fw::InteractionKind::Approval { .. } => Ok(payload.clone()),
        fw::InteractionKind::Input
            if request
                .presentation
                .as_ref()
                .is_some_and(|value| value["kind"] == "ask_user")
                && payload.get("answers").is_some_and(Value::is_array) =>
        {
            super::super::question::framework_answer(payload)
        }
        fw::InteractionKind::Input => Ok(payload.clone()),
    }
}
pub(super) fn answers_from_parts(
    parts: &[AgentMessagePart],
) -> Result<Vec<ChatResumeEntry>, AgentError> {
    let mut entries = Vec::new();
    for part in parts {
        if let AgentMessagePart::Context {
            kind,
            metadata: Some(metadata),
            ..
        } = part
        {
            if kind == "tron_interaction_response" {
                if let Some(values) = metadata.get("resume") {
                    entries.extend(
                        serde_json::from_value::<Vec<ChatResumeEntry>>(values.clone())
                            .map_err(|_| invalid("invalid native resume entries"))?,
                    );
                }
            }
        }
    }
    Ok(entries)
}
pub(super) async fn apply_answers(
    agent: &Agent,
    session: &str,
    entries: &[ChatResumeEntry],
    request: &mut fw::RunRequest,
) -> Result<(), AgentError> {
    let mut records = Vec::new();
    let mut logical_run = None;
    let mut ids = std::collections::BTreeSet::new();
    for entry in entries {
        let id = native_id(&entry.interrupt_id)?;
        if !ids.insert(id) {
            return Err(invalid("duplicate interaction response"));
        }
        let record = agent.interaction(session, id).await.map_err(map_error)?;
        if logical_run
            .replace(record.run_id)
            .is_some_and(|previous| previous != record.run_id)
        {
            return Err(invalid("responses belong to different logical runs"));
        }
        let value = answer_value(&record.request, &entry.payload)?;
        validate_record_response(&record, value.clone())?;
        records.push((id, value));
    }
    if let Some(run) = logical_run {
        ensure_waiting(agent, session, run).await?;
    }
    // Validate the whole batch before saving any answer. Native stores enforce answer CAS.
    for (id, value) in records {
        request.resume = agent
            .answer(session, id, value)
            .await
            .map_err(map_error)?
            .resume;
    }
    Ok(())
}
fn invalid(message: &str) -> AgentError {
    AgentError::InvalidConfiguration(message.into())
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

/// Saved answers have already passed native validation at their original timestamp.
/// Keep the SDK store's idempotent retry rule without extending a pending expiry.
pub(super) fn validate_record_response(
    record: &fw::InteractionRecord,
    value: Value,
) -> Result<(), AgentError> {
    let candidate = fw::InteractionResponse {
        interaction_id: record.request.id,
        value,
    };
    if let Some(previous) = &record.response {
        return if previous == &candidate {
            Ok(())
        } else {
            Err(invalid("interaction already has a different response"))
        };
    }
    record
        .request
        .validate_response(&candidate, now_ms())
        .map_err(map_error)
}

async fn ensure_waiting(agent: &Agent, session: &str, run: fw::RunId) -> Result<(), AgentError> {
    let checkpoint = agent.checkpoint(run).await.map_err(map_error)?;
    if checkpoint.session_id != session || checkpoint.status != fw::ExecutionStatus::Waiting {
        return Err(invalid("native interaction execution is no longer waiting"));
    }
    Ok(())
}
