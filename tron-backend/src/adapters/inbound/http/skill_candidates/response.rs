use crate::domain::dude_memory::*;
use serde::Serialize;
// Each endpoint has its own protocol DTO, independently evolvable from persisted domain records.
macro_rules! candidate_vo {
    ($name:ident) => {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            candidate_code: String,
            dude_code: String,
            batch_code: String,
            skill_name: String,
            operation: CandidateOperation,
            revision: u32,
            artifact_hash: String,
            status: SkillCandidateStatus,
            base_artifact_hash: Option<String>,
            creator: Option<CreatorReceipt>,
            files: Vec<SkillArtifactFile>,
            installed_files: Vec<SkillArtifactFile>,
            evidence: Vec<SkillEvidence>,
            validation: Option<SkillValidation>,
            reason: Option<String>,
            feedback: Option<String>,
            last_error: Option<String>,
            created_at: i64,
            updated_at: i64,
        }
        impl From<SkillCandidate> for $name {
            fn from(c: SkillCandidate) -> Self {
                Self {
                    candidate_code: c.candidate_code,
                    dude_code: c.dude_code,
                    batch_code: c.batch_code,
                    skill_name: c.skill_name,
                    operation: c.operation,
                    revision: c.revision,
                    artifact_hash: c.artifact_hash,
                    status: c.status,
                    base_artifact_hash: c.base_artifact_hash,
                    creator: c.creator,
                    files: c.files,
                    installed_files: c.installed_files,
                    evidence: c.evidence,
                    validation: c.validation,
                    reason: c.reason,
                    feedback: c.feedback,
                    last_error: c.last_error,
                    created_at: c.created_at,
                    updated_at: c.updated_at,
                }
            }
        }
    };
}
candidate_vo!(ListCandidateVo);
candidate_vo!(DetailCandidateVo);
candidate_vo!(ApproveCandidateVo);
candidate_vo!(RejectCandidateVo);
candidate_vo!(ReviseCandidateVo);
candidate_vo!(RetryCandidateInstallVo);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMemoryBatchVo {
    batch_code: String,
    dude_code: String,
    session_code: Option<String>,
    status: MemoryBatchStatus,
    summary: Option<String>,
    source_count: usize,
    memory_count: usize,
    saved_count: usize,
    duplicate_skipped_count: usize,
    unreliable_skipped_count: usize,
    created_at: i64,
    updated_at: i64,
}
impl From<MemoryBatch> for ListMemoryBatchVo {
    fn from(b: MemoryBatch) -> Self {
        Self {
            batch_code: b.batch_code,
            dude_code: b.dude_code,
            session_code: b.session_code,
            status: b.status,
            summary: b.summary,
            source_count: b.sources.len(),
            memory_count: b.saved_count,
            saved_count: b.saved_count,
            duplicate_skipped_count: b.duplicate_skipped_count,
            unreliable_skipped_count: b.unreliable_skipped_count,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }
    }
}
