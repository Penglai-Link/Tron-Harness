use serde::Deserialize;

use crate::application::workspace::{
    CreateWorkspaceCommand, MoveWorkspaceCommand, UpdateWorkspaceCommand,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_workspace_code: Option<String>,
}

impl From<CreateWorkspaceRequest> for CreateWorkspaceCommand {
    fn from(request: CreateWorkspaceRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
            parent_workspace_code: request.parent_workspace_code,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
}

impl From<UpdateWorkspaceRequest> for UpdateWorkspaceCommand {
    fn from(request: UpdateWorkspaceRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveWorkspaceRequest {
    pub parent_workspace_code: Option<String>,
}

impl From<MoveWorkspaceRequest> for MoveWorkspaceCommand {
    fn from(request: MoveWorkspaceRequest) -> Self {
        Self {
            parent_workspace_code: request.parent_workspace_code,
        }
    }
}
