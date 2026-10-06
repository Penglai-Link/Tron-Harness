use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WaitReasonCode {
    MineruUnavailable,
    MineruDisabled,
    NoEnabledNodes,
    NodeUnreachable,
    HealthTimeout,
    Unhealthy,
    VersionMismatch,
    NoTiers,
    UnsupportedTier,
    OriginalNodeUnavailable,
    NodeSetUnavailable,
    ParserMissing,
    InvalidNodeConfig,
    ParseGlobalCapacity,
    EmbeddingGlobalCapacity,
    EmbeddingGroupCapacity,
    NodeCapacity,
    RetryBackoff,
    CancellationPending,
    IntakeStorage,
    IntakeInvalid,
    IntakeForbidden,
    IntakeConflict,
    IntakeMissing,
}

/// Durable diagnostics contain codes and parameters, never a user's UI locale.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionWaitReason {
    pub code: WaitReasonCode,
    #[serde(default)]
    pub params: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<IngestionWaitReason>,
}
impl IngestionWaitReason {
    pub fn new(code: WaitReasonCode) -> Self {
        Self {
            code,
            params: Value::Null,
            details: vec![],
        }
    }
    pub fn with_params(code: WaitReasonCode, params: Value) -> Self {
        Self {
            code,
            params,
            details: vec![],
        }
    }
}
