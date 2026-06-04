use serde::Deserialize;

use crate::application::agent::UpdatePlatformMainAgentCommand;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateMainAgentConfigRequest {
    pub personality: Option<String>,
    pub instructions: Option<String>,
}

impl From<UpdateMainAgentConfigRequest> for UpdatePlatformMainAgentCommand {
    fn from(request: UpdateMainAgentConfigRequest) -> Self {
        Self {
            personality: request.personality,
            instructions: request.instructions,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListToolsQuery {
    pub domain_code: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInternalAgentPolicyRequest {
    pub tool_assignment_mode: Option<String>,
    pub tool_exposure_mode: Option<String>,
    pub expanded_domain_persistence_mode: Option<String>,
    pub max_steps: Option<u32>,
    pub max_tool_calls: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceInternalAgentToolAssignmentsRequest {
    pub tool_codes: Vec<String>,
}
