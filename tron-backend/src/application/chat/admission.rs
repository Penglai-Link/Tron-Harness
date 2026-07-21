use crate::application::agent::{
    ActiveRunError, ActiveRunGuard, AgentInvocationError, generate_agent_run_code,
};

use super::{ChatSendError, ChatService};

const MAIN_RUNTIME_ADAPTER_KEY: &str = "REACT";
const CODEX_RUNTIME_ADAPTER_KEY: &str = "CODEX_EXEC";
const OPENCODE_RUNTIME_ADAPTER_KEY: &str = "OPENCODE_RUN";

pub(super) struct ChatRunReservation {
    pub run_code: String,
    pub runtime_adapter_key: &'static str,
    pub guard: ActiveRunGuard,
}

impl ChatService {
    pub(super) fn reserve_run(
        &self,
        session_code: &str,
        requested_run_code: Option<String>,
        agent_code: &str,
    ) -> Result<ChatRunReservation, ChatSendError> {
        let run_code = requested_run_code
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(generate_agent_run_code);
        let runtime_adapter_key = runtime_adapter_key_for_agent(agent_code);
        let supports_steer = runtime_adapter_key == MAIN_RUNTIME_ADAPTER_KEY;
        let guard = match self.active_runs.reserve(
            session_code,
            &run_code,
            runtime_adapter_key,
            supports_steer,
        ) {
            Ok(reservation) => reservation,
            Err(ActiveRunError::AlreadyActive) => {
                let active_run_code = self
                    .active_runs
                    .snapshot(session_code)
                    .ok()
                    .flatten()
                    .map(|active_run| active_run.run_code);
                return Err(ChatSendError::RunAlreadyActive { active_run_code });
            }
            Err(error) => {
                return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                    error.to_string(),
                )));
            }
        };
        Ok(ChatRunReservation {
            run_code,
            runtime_adapter_key,
            guard,
        })
    }
}

fn runtime_adapter_key_for_agent(agent_code: &str) -> &'static str {
    match agent_code.trim().to_ascii_lowercase().as_str() {
        "codex" => CODEX_RUNTIME_ADAPTER_KEY,
        "opencode" => OPENCODE_RUNTIME_ADAPTER_KEY,
        _ => MAIN_RUNTIME_ADAPTER_KEY,
    }
}
