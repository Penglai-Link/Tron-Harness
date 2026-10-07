use crate::{
    application::agent::{ActiveRunError, ActiveRunState},
    domain::chat::ChatError,
};

use super::ChatService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatActiveRunState {
    Running,
    Finishing,
}

impl ChatActiveRunState {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Running => "RUNNING",
            Self::Finishing => "FINISHING",
        }
    }
}

impl From<ActiveRunState> for ChatActiveRunState {
    fn from(value: ActiveRunState) -> Self {
        match value {
            ActiveRunState::Running => Self::Running,
            ActiveRunState::Finishing => Self::Finishing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveChatRunSnapshot {
    pub run_code: String,
    pub runtime_adapter_key: String,
    pub supports_steer: bool,
    pub state: ChatActiveRunState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetActiveChatRunResult {
    pub session_code: String,
    pub active_run: Option<ActiveChatRunSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopChatRunCommand {
    pub session_code: Option<String>,
    pub expected_run_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopChatRunResult {
    pub session_code: String,
    pub run_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatRunControlError {
    SessionCodeRequired,
    ExpectedRunCodeRequired,
    SessionNotFound,
    NoActiveRun,
    RunMismatch { active_run_code: String },
    RunFinishing,
    RegistryUnavailable,
}

impl ChatService {
    pub fn get_active_run(
        &self,
        session_code: &str,
    ) -> Result<GetActiveChatRunResult, ChatRunControlError> {
        let session_code = normalized_required(Some(session_code.to_string()))
            .ok_or(ChatRunControlError::SessionCodeRequired)?;
        self.ensure_run_control_session_exists(&session_code)?;
        let active_run = self
            .active_runs
            .snapshot(&session_code)
            .map_err(ChatRunControlError::from)?
            .map(|snapshot| ActiveChatRunSnapshot {
                run_code: snapshot.run_code,
                runtime_adapter_key: snapshot.runtime_adapter_key,
                supports_steer: snapshot.supports_steer,
                state: snapshot.state.into(),
            });
        Ok(GetActiveChatRunResult {
            session_code,
            active_run,
        })
    }

    pub fn stop_run(
        &self,
        command: StopChatRunCommand,
    ) -> Result<StopChatRunResult, ChatRunControlError> {
        let session_code = normalized_required(command.session_code)
            .ok_or(ChatRunControlError::SessionCodeRequired)?;
        let expected_run_code = normalized_required(command.expected_run_code)
            .ok_or(ChatRunControlError::ExpectedRunCodeRequired)?;
        self.ensure_run_control_session_exists(&session_code)?;
        let stopped = self
            .active_runs
            .request_stop(&session_code, &expected_run_code)
            .map_err(ChatRunControlError::from)?;
        Ok(StopChatRunResult {
            session_code,
            run_code: stopped.run_code,
        })
    }

    fn ensure_run_control_session_exists(
        &self,
        session_code: &str,
    ) -> Result<(), ChatRunControlError> {
        match self.get_session(session_code) {
            Ok(_) => Ok(()),
            Err(ChatError::SessionNotFound) => Err(ChatRunControlError::SessionNotFound),
            Err(_) => Err(ChatRunControlError::RegistryUnavailable),
        }
    }
}

impl From<ActiveRunError> for ChatRunControlError {
    fn from(error: ActiveRunError) -> Self {
        match error {
            ActiveRunError::NoActiveRun => Self::NoActiveRun,
            ActiveRunError::RunMismatch { active_run_code } => {
                Self::RunMismatch { active_run_code }
            }
            ActiveRunError::RunFinishing => Self::RunFinishing,
            ActiveRunError::AlreadyActive
            | ActiveRunError::SessionArchived
            | ActiveRunError::RunStarting
            | ActiveRunError::RuntimeUnsupported { .. }
            | ActiveRunError::RegistryUnavailable => Self::RegistryUnavailable,
        }
    }
}

fn normalized_required(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
