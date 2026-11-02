use crate::application::agent::{
    ActiveRunError, ActiveRunGuard, ActiveRunRegistry, AgentInvocationError,
    generate_agent_run_code,
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
        runtime_type: crate::domain::agent::AgentRuntimeType,
        compacting: bool,
    ) -> Result<ChatRunReservation, ChatSendError> {
        let run_code = requested_run_code
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(generate_agent_run_code);
        eliteforge_agent_core::RunId::parse_str(&run_code).map_err(|_| {
            ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                "runCode must be a UUID".into(),
            ))
        })?;
        let runtime_adapter_key = runtime_adapter_key_for_agent(runtime_type);
        if compacting && runtime_adapter_key != MAIN_RUNTIME_ADAPTER_KEY {
            return Err(ChatSendError::Agent(AgentInvocationError::RuntimeFailed(
                "Context compaction requires a native agent".into(),
            )));
        }
        let supports_steer = runtime_adapter_key == MAIN_RUNTIME_ADAPTER_KEY;
        let reserve = if compacting {
            ActiveRunRegistry::reserve_context_compaction
        } else {
            ActiveRunRegistry::reserve
        };
        let guard = match reserve(
            &self.active_runs,
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
            Err(ActiveRunError::SessionArchived) => {
                return Err(ChatSendError::Chat(
                    crate::domain::chat::ChatError::SessionArchived,
                ));
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

fn runtime_adapter_key_for_agent(runtime: crate::domain::agent::AgentRuntimeType) -> &'static str {
    use crate::domain::agent::AgentRuntimeType;
    match runtime {
        AgentRuntimeType::React => MAIN_RUNTIME_ADAPTER_KEY,
        AgentRuntimeType::CodexExec => CODEX_RUNTIME_ADAPTER_KEY,
        AgentRuntimeType::OpenCodeRun => OPENCODE_RUNTIME_ADAPTER_KEY,
        AgentRuntimeType::DshHeadless => "DSH_HEADLESS",
        AgentRuntimeType::CodexAppServer => "CODEX_APP_SERVER",
        AgentRuntimeType::OpenCodeServe => "OPENCODE_SERVE",
        AgentRuntimeType::OpenCodeAcp => "OPENCODE_ACP",
    }
}
