use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySource {
    pub source_code: String,
    pub workspace_code: String,
    pub version: String,
    pub excerpt: String,
    pub root_owner_assistant_code: Option<String>,
    pub lineage_assistant_codes: Vec<String>,
    pub kind: MemorySourceKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySourceKind {
    Conversation,
    UserStatement,
    TaskResult,
    MemoryReport,
    RuntimeLog,
}
impl MemorySource {
    pub fn eligible_for(&self, assistant: &str) -> bool {
        self.root_owner_assistant_code.as_deref() != Some(assistant)
            && !self
                .lineage_assistant_codes
                .iter()
                .any(|code| code == assistant)
            && matches!(
                self.kind,
                MemorySourceKind::Conversation
                    | MemorySourceKind::UserStatement
                    | MemorySourceKind::TaskResult
            )
            && !self.excerpt.trim().is_empty()
    }
    pub fn identity(&self) -> String {
        format!("{}:{}", self.workspace_code.len(), self.workspace_code) + &self.source_code
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryBatchStatus {
    Prepared,
    Running,
    Completed,
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBatch {
    pub batch_code: String,
    pub assistant_code: String,
    pub user_code: String,
    pub session_code: Option<String>,
    pub status: MemoryBatchStatus,
    pub sources: Vec<MemorySource>,
    pub memory_receipts: Vec<MemoryWriteReceipt>,
    pub saved_count: usize,
    pub duplicate_skipped_count: usize,
    pub unreliable_skipped_count: usize,
    #[serde(default)]
    pub consolidation_plan: Option<serde_json::Value>,
    pub summary: Option<String>,
    pub revision: u32,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWriteReceipt {
    pub proposal_key: String,
    pub semantic_key: String,
    pub workspace_code: String,
    pub memory_code: String,
    pub source_codes: Vec<String>,
    pub claim_kind: MemoryClaimKind,
    pub conflict_with_memory_code: Option<String>,
    pub outcome: MemoryWriteOutcome,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryWriteOutcome {
    Saved,
    DuplicateSkipped,
    UnreliableSkipped,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryClaimKind {
    Fact,
    UserDecision,
    Inference,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateOperation {
    Create,
    Update,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCandidateStatus {
    WaitingDependency,
    Authoring,
    ValidationFailed,
    PendingApproval,
    Approved,
    Installing,
    InstallFailed,
    Installed,
    Rejected,
    RevisionRequested,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactEncoding {
    Utf8,
    Base64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillArtifactFile {
    pub path: String,
    pub content: String,
    pub encoding: ArtifactEncoding,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorReceipt {
    pub skill_name: String,
    pub version: String,
    pub content_hash: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub evidence: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillValidation {
    pub static_passed: bool,
    pub behavior_passed: bool,
    pub checks: Vec<ValidationCheck>,
    pub artifact_hash: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillEvidence {
    pub source_code: String,
    pub workspace_code: String,
    pub version: String,
    pub excerpt: String,
}
impl From<&MemorySource> for SkillEvidence {
    fn from(source: &MemorySource) -> Self {
        Self {
            source_code: source.source_code.clone(),
            workspace_code: source.workspace_code.clone(),
            version: source.version.clone(),
            excerpt: source.excerpt.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCandidate {
    pub candidate_code: String,
    pub assistant_code: String,
    pub batch_code: String,
    pub user_code: String,
    pub skill_name: String,
    pub operation: CandidateOperation,
    pub revision: u32,
    pub artifact_hash: String,
    pub status: SkillCandidateStatus,
    pub base_artifact_hash: Option<String>,
    pub creator: Option<CreatorReceipt>,
    pub author_session_code: String,
    pub files: Vec<SkillArtifactFile>,
    pub installed_files: Vec<SkillArtifactFile>,
    pub evidence: Vec<SkillEvidence>,
    pub validation: Option<SkillValidation>,
    pub approved_hash: Option<String>,
    pub reason: Option<String>,
    pub feedback: Option<String>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorCheck {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistantMemoryError {
    NotFound,
    Conflict,
    ExecutionBusy,
    Invalid(String),
    Dependency(String),
    Storage(String),
    Installation(String),
}
impl std::fmt::Display for AssistantMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("PersonAI memory record not found"),
            Self::Conflict => f.write_str("PersonAI memory record or installed skill changed"),
            Self::ExecutionBusy => f.write_str("PersonAI memory session already has an active run"),
            Self::Invalid(s) | Self::Dependency(s) | Self::Storage(s) | Self::Installation(s) => {
                f.write_str(s)
            }
        }
    }
}
impl std::error::Error for AssistantMemoryError {}
