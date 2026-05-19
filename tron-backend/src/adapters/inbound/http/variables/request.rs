use serde::Deserialize;

use crate::application::variable::{CreateVariableCommand, UpdateVariableCommand};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateVariableRequest {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

impl From<CreateVariableRequest> for CreateVariableCommand {
    fn from(request: CreateVariableRequest) -> Self {
        Self {
            key: request.key,
            value: request.value,
            description: request.description,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateVariableRequest {
    pub value: String,
    pub description: Option<String>,
}

impl From<UpdateVariableRequest> for UpdateVariableCommand {
    fn from(request: UpdateVariableRequest) -> Self {
        Self {
            value: request.value,
            description: request.description,
        }
    }
}
