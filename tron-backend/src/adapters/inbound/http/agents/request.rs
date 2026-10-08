use serde::Deserialize;

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
    pub max_parallel_tool_calls: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceInternalAgentToolAssignmentsRequest {
    pub tool_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInternalAgentMcpPolicyRequest {
    pub mcp_access_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceInternalAgentMcpServerAssignmentsRequest {
    pub server_codes: Vec<String>,
}
