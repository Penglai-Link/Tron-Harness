use serde::Deserialize;

use crate::{
    application::skill::InstallSkillCommand,
    domain::skill::{SkillSearchMode, WorkspaceSkillListQuery},
};

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
    pub enable_for_workspace_code: Option<String>,
}

impl From<InstallRegistrySkillRequest> for InstallSkillCommand {
    fn from(request: InstallRegistrySkillRequest) -> Self {
        Self {
            package_ref: request.package_ref,
            source_ref: request.source_ref,
            skill_name: request.skill_name,
            enable_for_workspace_code: request.enable_for_workspace_code,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSkillBindingRequest {
    pub skill_name: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillDetailQuery {
    pub workspace_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkillUninstallQuery {
    pub remove_bindings: Option<bool>,
}

pub type WorkspaceSkillsQuery = WorkspaceSkillListQuery;
