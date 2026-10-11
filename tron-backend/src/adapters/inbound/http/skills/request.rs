use serde::Deserialize;

use crate::{application::skill::InstallSkillCommand, domain::skill::SkillSearchMode};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchSkillsRequest {
    pub query: String,
    pub mode: Option<SkillSearchMode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallRegistrySkillRequest {
    pub package_ref: Option<String>,
    pub source_ref: Option<String>,
    pub skill_name: Option<String>,
}

impl From<InstallRegistrySkillRequest> for InstallSkillCommand {
    fn from(request: InstallRegistrySkillRequest) -> Self {
        Self {
            package_ref: request.package_ref,
            source_ref: request.source_ref,
            skill_name: request.skill_name,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentSkillsRequest {
    pub skill_names: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillAgentsRequest {
    pub agent_codes: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistrySkillsQuery {
    pub search: Option<String>,
}
