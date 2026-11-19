//! Durable queue for capability package operations.
//!
//! The queue only holds intent. Execution stays in the Framework host so the
//! same idle gate and rollback used by direct requests apply to queued work.
//! Storage is expressed as a repository port; SQLite is one implementation.

use std::sync::{Arc, Mutex};

/// Upper bound for waiting operations. The UI never needs a deeper backlog and
/// the bound keeps one profile from growing an unbounded work list.
pub const CAPABILITY_QUEUE_LIMIT: usize = 100;
/// Completed operations retained for display and diagnostics.
pub const CAPABILITY_OPERATION_LOG_LIMIT: usize = 30;
/// Display name copied from the enqueue request; diagnostics only.
pub const CAPABILITY_QUEUE_NAME_LIMIT: usize = 120;
/// Raw runtime error retained in the log entry.
pub const CAPABILITY_QUEUE_ERROR_LIMIT: usize = 500;

/// Application error codes shared with the direct uninstall endpoint. These are
/// persisted with a failed uninstall so the UI can offer the force flow after an
/// immediate attempt, and remain visible in the operation log.
pub const CAPABILITY_UNINSTALL_BUNDLED: &str = "101031001";
pub const CAPABILITY_UNINSTALL_REFERENCED: &str = "101031002";
pub const CAPABILITY_UNINSTALL_INCOMPLETE: &str = "101031003";
pub const CAPABILITY_UNINSTALL_BUSY: &str = "101031004";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityQueueOperation {
    Enable,
    Disable,
    Uninstall,
}

impl CapabilityQueueOperation {
    pub fn storage_value(self) -> &'static str {
        match self {
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Uninstall => "uninstall",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "enable" => Some(Self::Enable),
            "disable" => Some(Self::Disable),
            "uninstall" => Some(Self::Uninstall),
            _ => None,
        }
    }
}

/// Execution state shown by the capability center. `Running` is in-memory only:
/// a restart reloads every pending row as `Queued` and retries it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CapabilityQueueStatus {
    #[default]
    Queued,
    Running,
}

#[derive(Debug, Clone)]
pub struct CapabilityQueueItem {
    pub id: String,
    pub pack_id: String,
    pub pack_name: String,
    pub operation: CapabilityQueueOperation,
    /// Explicit force confirmation; only meaningful for uninstall.
    pub force: bool,
    pub locale: String,
    pub created_at: i64,
    pub status: CapabilityQueueStatus,
}

#[derive(Debug, Clone)]
pub struct CapabilityOperationRecord {
    pub item_id: String,
    pub pack_id: String,
    pub pack_name: String,
    pub operation: CapabilityQueueOperation,
    pub force: bool,
    pub succeeded: bool,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub finished_at: i64,
}

#[derive(Debug, Clone)]
pub enum CapabilityQueueOutcome {
    Pending,
    Succeeded,
    Failed { error: String, code: Option<String> },
}

#[derive(Debug, Clone)]
pub enum CapabilityEnqueueRecord {
    Queued {
        item: CapabilityQueueItem,
        was_empty: bool,
    },
    Existing(CapabilityQueueItem),
    Full,
}

/// Storage port. Every mutation that must stay consistent is one call so the
/// adapter can keep it inside a single SQLite transaction.
pub trait CapabilityQueueRepository: Send {
    /// Pending items in submission order.
    fn items(&mut self) -> Result<Vec<CapabilityQueueItem>, String>;
    /// Completed operations, newest first, bounded by the retention limit.
    fn log(&mut self) -> Result<Vec<CapabilityOperationRecord>, String>;
    fn has_items(&mut self) -> Result<bool, String>;
    /// Atomically de-duplicate `(pack_id, operation)`, enforce the queue limit
    /// and append the item. A stronger uninstall request upgrades a waiting
    /// non-forced uninstall in place. The returned `was_empty` reports the queue
    /// state before insertion so the caller can attempt an immediate drain.
    fn enqueue(&mut self, item: CapabilityQueueItem) -> Result<CapabilityEnqueueRecord, String>;
    /// Atomically remove the item, append its result and trim the log.
    fn complete(&mut self, item_id: &str, record: &CapabilityOperationRecord)
    -> Result<(), String>;
    fn outcome(&mut self, item_id: &str) -> Result<CapabilityQueueOutcome, String>;
}

/// Serializes access to one repository, following the platform service shape
/// used by capability resources and PersonAI storage.
#[derive(Clone)]
pub struct CapabilityQueueService(Arc<Mutex<Box<dyn CapabilityQueueRepository>>>);

impl CapabilityQueueService {
    pub fn new(repository: impl CapabilityQueueRepository + 'static) -> Self {
        Self(Arc::new(Mutex::new(Box::new(repository))))
    }

    fn repo(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Box<dyn CapabilityQueueRepository>>, String> {
        self.0
            .lock()
            .map_err(|_| "capability queue storage is unavailable".to_string())
    }

    pub fn items(&self) -> Result<Vec<CapabilityQueueItem>, String> {
        self.repo()?.items()
    }

    pub fn log(&self) -> Result<Vec<CapabilityOperationRecord>, String> {
        self.repo()?.log()
    }

    pub fn has_items(&self) -> Result<bool, String> {
        self.repo()?.has_items()
    }

    pub fn enqueue(&self, item: CapabilityQueueItem) -> Result<CapabilityEnqueueRecord, String> {
        self.repo()?.enqueue(item)
    }

    pub fn complete(
        &self,
        item_id: &str,
        record: &CapabilityOperationRecord,
    ) -> Result<(), String> {
        self.repo()?.complete(item_id, record)
    }

    pub fn outcome(&self, item_id: &str) -> Result<CapabilityQueueOutcome, String> {
        self.repo()?.outcome(item_id)
    }
}
