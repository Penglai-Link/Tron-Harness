use std::{error::Error, fmt};
#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    ReasoningEffortUnsupported,
    InvalidProtocolModelType,
    RuntimeResolutionFailed(String),
    StorageFailure,
}
impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReasoningEffortUnsupported => {
                formatter.write_str("reasoningEffort is not supported by the selected model")
            }
            Self::InvalidProtocolModelType => {
                formatter.write_str("provider protocol does not support the model type")
            }
            Self::RuntimeResolutionFailed(message) => formatter.write_str(message),
            Self::StorageFailure => formatter.write_str("provider model storage failure"),
        }
    }
}
impl Error for ModelError {}
