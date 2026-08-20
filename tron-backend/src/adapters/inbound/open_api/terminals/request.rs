use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenApiGenerateTerminalCommandRequest {
    pub workspace_code: String,
    pub model_code: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenApiIssueTerminalSocketTicketRequest {
    pub workspace_code: String,
    pub app_code: Option<String>,
}
