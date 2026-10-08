//! Knowledge-base ingestion lifecycle. This is a product task, not a model job.
use serde::{Deserialize, Serialize};
mod wait_reason;
pub use wait_reason::{IngestionWaitReason, WaitReasonCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum IngestionStage {
    Parsing,
    Chunking,
    Embedding,
    Publishing,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionTask {
    pub id: String,
    pub knowledge_base_id: String,
    pub document_id: String,
    pub user_code: String,
    pub workspace_code: String,
    pub document_revision: u64,
    pub resource_revision: u64,
    /// Hash of the original bytes and immutable parser/chunk/embedding identities.
    pub input_config_hash: String,
    /// Immutable configuration snapshot; never resolve new defaults on a retry.
    pub configuration: serde_json::Value,
    pub state: String,
    pub stage: IngestionStage,
    pub attempt: u32,
    #[serde(default)]
    pub failures: u32,
    pub lease_owner: Option<String>,
    pub lease_until_ms: i64,
    pub retry_after_ms: i64,
    pub endpoint_id: Option<String>,
    pub remote_file_id: Option<String>,
    pub remote_job_id: Option<String>,
    /// A POST may have been accepted even when its response was lost.
    pub reconciling: bool,
    /// Durable upstream cancellation; retried locally even across a restart.
    #[serde(default)]
    pub cancellation_pending: bool,
    pub parsed_resource_code: Option<String>,
    pub chunks_resource_code: Option<String>,
    #[serde(default)]
    pub embedding_resource_code: Option<String>,
    pub error: Option<String>,
    #[serde(default)]
    pub wait_reason: Option<IngestionWaitReason>,
}

impl IngestionTask {
    /// Attempt-specific staging prevents a fenced-out writer from replacing a
    /// generation published by the replacement owner of the same task.
    pub fn generation_source_code(&self) -> String {
        format!("{}_{}_{}", self.document_id, self.id, self.attempt)
    }
    pub fn queued(
        id: String,
        knowledge_base_id: String,
        document_id: String,
        user_code: String,
        workspace_code: String,
        document_revision: u64,
        resource_revision: u64,
        input_config_hash: String,
        configuration: serde_json::Value,
    ) -> Self {
        Self {
            id,
            knowledge_base_id,
            document_id,
            user_code,
            workspace_code,
            document_revision,
            resource_revision,
            input_config_hash,
            configuration,
            state: "QUEUED".into(),
            stage: IngestionStage::Parsing,
            attempt: 0,
            failures: 0,
            lease_owner: None,
            lease_until_ms: 0,
            retry_after_ms: 0,
            endpoint_id: None,
            remote_file_id: None,
            remote_job_id: None,
            reconciling: false,
            cancellation_pending: false,
            parsed_resource_code: None,
            chunks_resource_code: None,
            embedding_resource_code: None,
            error: None,
            wait_reason: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IngestionLease {
    pub task_id: String,
    pub attempt: u32,
    pub owner: String,
}

#[derive(Clone, Debug)]
pub struct ParseEndpointCapacity {
    pub supported_tiers: Vec<String>,
    pub endpoint_id: String,
    pub capacity: usize,
    /// A shared CPU/GPU group may contain several independently routed endpoints.
    pub resource_group: String,
    pub resource_group_capacity: usize,
    pub weight: u32,
}

#[derive(Clone, Debug)]
pub struct IngestionClaim {
    pub task_id: Option<String>,
    pub stage: IngestionStage,
    pub owner: String,
    pub now_ms: i64,
    pub lease_ms: i64,
    pub global_capacity: usize,
    /// Hardware group of the existing Framework embedding connection, if shared.
    pub embedding_resource_group: Option<(String, usize)>,
    /// Already-authorized, enabled, healthy nodes matching the immutable profile.
    /// An official MinerU router can be one endpoint. Remote nodes are opt-in.
    pub parse_endpoints: Vec<ParseEndpointCapacity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IngestionQueueError {
    Conflict,
    NotFound,
    Invalid(String),
    Storage(String),
}

/// Database detachment is atomic; physical object removal happens after commit.
#[derive(Clone, Debug, PartialEq)]
pub struct PublicationCleanup {
    pub objects: Vec<String>,
    pub cancellations: Vec<IngestionTask>,
}

/// Storage operations fence every mutation by lease owner and attempt.
/// Clocks are supplied by the application so expiry/backoff can be tested.
pub trait KnowledgeIngestionQueue: Send + Sync {
    fn for_document(&self, document: &str) -> Result<Vec<IngestionTask>, IngestionQueueError>;
    fn pending(&self, stage: IngestionStage, now_ms: i64) -> Result<bool, IngestionQueueError>;
    fn pending_cancellations(&self) -> Result<Vec<IngestionTask>, IngestionQueueError>;
    fn complete_cancellation(
        &self,
        task: &IngestionTask,
        job_id: Option<String>,
    ) -> Result<(), IngestionQueueError>;
    fn enqueue(&self, task: IngestionTask) -> Result<IngestionTask, IngestionQueueError>;
    /// Persist the proposed configuration and enqueue its immutable snapshot atomically.
    fn enqueue_configured(
        &self,
        task: IngestionTask,
        document: super::KnowledgeBaseDocument,
    ) -> Result<IngestionTask, IngestionQueueError>;
    fn get(&self, id: &str) -> Result<IngestionTask, IngestionQueueError>;
    fn claim(&self, claim: IngestionClaim) -> Result<Option<IngestionTask>, IngestionQueueError>;
    fn report_parser_availability(
        &self,
        now_ms: i64,
        endpoints: &[ParseEndpointCapacity],
        unavailable: &[(Option<String>, IngestionWaitReason)],
    ) -> Result<(), IngestionQueueError> {
        let _ = (now_ms, endpoints, unavailable);
        Ok(())
    }
    fn checkpoint(
        &self,
        task: &IngestionTask,
        lease: &IngestionLease,
        now_ms: i64,
    ) -> Result<(), IngestionQueueError>;
    fn heartbeat(
        &self,
        lease: &IngestionLease,
        now_ms: i64,
        lease_ms: i64,
    ) -> Result<(), IngestionQueueError>;
    fn wait_for_embedding(
        &self,
        task: &IngestionTask,
        lease: &IngestionLease,
        now_ms: i64,
    ) -> Result<(), IngestionQueueError>;
    /// Return same-node IDs to the caller, which then requests remote cancellation.
    fn cancel(&self, id: &str) -> Result<IngestionTask, IngestionQueueError>;
    fn fail(
        &self,
        lease: &IngestionLease,
        now_ms: i64,
        error: &str,
        retry_after_ms: Option<i64>,
    ) -> Result<(), IngestionQueueError>;
    fn retry(&self, id: &str, now_ms: i64) -> Result<IngestionTask, IngestionQueueError>;
    /// Caller has completed one immutable RAG generation before this visibility switch.
    fn publish(
        &self,
        lease: &IngestionLease,
        now_ms: i64,
        ready_document: &serde_json::Value,
    ) -> Result<PublicationCleanup, IngestionQueueError>;
}
