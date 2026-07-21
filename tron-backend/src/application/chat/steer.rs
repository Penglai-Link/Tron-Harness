use crate::{application::agent::ActiveRunError, domain::chat::ChatError};

#[derive(Debug, Clone, PartialEq)]
pub enum ChatSteerError {
    Chat(ChatError),
    SessionCodeRequired,
    ExpectedRunCodeRequired,
    NoActiveRun,
    RunMismatch { active_run_code: String },
    RuntimeUnsupported { runtime_adapter_key: String },
    RunStarting,
    RunFinishing,
    RegistryUnavailable,
}

impl From<ActiveRunError> for ChatSteerError {
    fn from(error: ActiveRunError) -> Self {
        match error {
            ActiveRunError::NoActiveRun => Self::NoActiveRun,
            ActiveRunError::RunMismatch { active_run_code } => {
                Self::RunMismatch { active_run_code }
            }
            ActiveRunError::RuntimeUnsupported {
                runtime_adapter_key,
            } => Self::RuntimeUnsupported {
                runtime_adapter_key,
            },
            ActiveRunError::RunStarting => Self::RunStarting,
            ActiveRunError::RunFinishing => Self::RunFinishing,
            ActiveRunError::AlreadyActive | ActiveRunError::RegistryUnavailable => {
                Self::RegistryUnavailable
            }
        }
    }
}
