use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApproveCandidateRequest {
    pub revision: u32,
    pub artifact_hash: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RejectCandidateRequest {
    pub revision: u32,
    pub artifact_hash: String,
    pub reason: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviseCandidateRequest {
    pub revision: u32,
    pub artifact_hash: String,
    pub feedback: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetryCandidateInstallRequest {
    pub revision: u32,
    pub artifact_hash: String,
}
