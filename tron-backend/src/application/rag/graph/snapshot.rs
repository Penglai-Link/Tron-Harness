//! Server-owned continuations. A cursor never contains caller-supplied scopes or vectors.
use serde_json::Value;
use std::collections::BTreeSet;

pub const GRAPH_SNAPSHOT_TTL_MS: i64 = 300_000;
pub const GRAPH_SNAPSHOTS_PER_USER: usize = 16;

#[derive(Clone, Debug)]
pub struct GraphSnapshotDraft {
    pub owner_hash: String,
    pub identity_hash: String,
    pub request: Value,
    pub traversal: Value,
    pub generation_ids: BTreeSet<String>,
    /// Continuations retain the original expiry instead of extending retention.
    pub expires_at: i64,
}

#[derive(Clone, Debug)]
pub struct GraphQuerySnapshot {
    pub cursor: String,
    pub owner_hash: String,
    pub identity_hash: String,
    pub request: Value,
    pub traversal: Value,
    pub generation_ids: BTreeSet<String>,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphSnapshotError {
    Invalid,
    Unavailable,
    PublicationChanged,
    Storage(String),
}

impl std::fmt::Display for GraphSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.write_str("invalid graph snapshot"),
            Self::Unavailable => f.write_str("graph snapshot is unavailable; start a new search"),
            Self::PublicationChanged => {
                f.write_str("graph publication changed; start a new search")
            }
            Self::Storage(error) => write!(f, "graph snapshot storage: {error}"),
        }
    }
}
impl std::error::Error for GraphSnapshotError {}
