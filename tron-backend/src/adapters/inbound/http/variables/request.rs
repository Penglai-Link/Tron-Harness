use serde::Deserialize;

use crate::application::variable::{
    CreateVariableCommand, UpdateVariableCommand, VariableValueMode,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVariableRequest {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
    pub value_mode: Option<RequestVariableValueMode>,
}

impl From<CreateVariableRequest> for CreateVariableCommand {
    fn from(request: CreateVariableRequest) -> Self {
        Self {
            key: request.key,
            value: request.value,
            description: request.description,
            is_sensitive: request.is_sensitive,
            value_mode: request.value_mode.map(Into::into),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateVariableRequest {
    pub value: Option<String>,
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
    pub value_mode: Option<RequestVariableValueMode>,
}

impl From<UpdateVariableRequest> for UpdateVariableCommand {
    fn from(request: UpdateVariableRequest) -> Self {
        Self {
            value: request.value,
            description: request.description,
            is_sensitive: request.is_sensitive,
            value_mode: request.value_mode.map(Into::into),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestVariableValueMode {
    Replace,
    Preserve,
}

impl From<RequestVariableValueMode> for VariableValueMode {
    fn from(value: RequestVariableValueMode) -> Self {
        match value {
            RequestVariableValueMode::Replace => Self::Replace,
            RequestVariableValueMode::Preserve => Self::Preserve,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevealWorkspaceVariableRequest {
    pub workspace_code: String,
    pub variable_key: String,
    pub step_up_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevealPlatformVariableRequest {
    pub variable_key: String,
    pub step_up_token: String,
}
