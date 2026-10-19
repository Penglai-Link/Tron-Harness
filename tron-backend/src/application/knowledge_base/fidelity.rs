//! Legacy fidelity evidence and publication guards for persisted documents.
use super::KnowledgeBaseDocument;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FidelityIssue {
    #[serde(default)]
    pub source_region: Option<[f64; 4]>,
    #[serde(default)]
    pub candidate_region: Option<[f64; 4]>,
    #[serde(default)]
    pub candidate_excerpt: Option<String>,
    pub kind: String,
    pub page: Option<u32>,
    pub blocks: Vec<String>,
    pub source_excerpt: String,
    pub image_resource_codes: Vec<String>,
}
/// Bounded agreement evidence, never a manual pass or a guarantee of clinical truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FidelityResolution {
    pub method: String,
    pub original_sha256: String,
    pub native_text_sha256: String,
    pub candidate_text_sha256: String,
    pub candidate_resource_code: String,
    pub original_parse_resource_code: Option<String>,
    pub discrepancies: Vec<FidelityIssue>,
    pub coverage: String,
}
pub(crate) fn blocked(doc: &KnowledgeBaseDocument) -> bool {
    doc.status == "REVIEW_REQUIRED"
        || doc
            .parsing_artifact
            .as_ref()
            .is_some_and(|p| !p.fidelity_issues.is_empty())
}

pub(crate) fn failure_reason(doc: &KnowledgeBaseDocument) -> String {
    doc.parsing_artifact
        .as_ref()
        .map(|artifact| {
            artifact
                .fidelity_issues
                .iter()
                .map(|issue| issue.source_excerpt.as_str())
                .collect::<Vec<_>>()
                .join("；")
        })
        .filter(|reason| !reason.is_empty())
        .unwrap_or_else(|| "现有解析产物已记录失败".into())
}
