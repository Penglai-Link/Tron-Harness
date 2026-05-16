use serde::Deserialize;

use crate::application::workspace::{
    CreateWorkspaceCommand, MoveWorkspaceCommand, ReplaceWorkspaceRoleAccessCommand,
    ReplaceWorkspaceUserAccessCommand, UpdateWorkspaceCommand, WorkspaceRoleAccessDraft,
    WorkspaceUserAccessDraft,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceRequest {
    pub workspace_code: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_workspace_code: Option<String>,
}

impl From<CreateWorkspaceRequest> for CreateWorkspaceCommand {
    fn from(request: CreateWorkspaceRequest) -> Self {
        Self {
            workspace_code: request.workspace_code,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceWorkspaceUsersRequest {
    #[serde(default)]
    pub users: Vec<WorkspaceUserGrantRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceUserGrantRequest {
    pub user_code: String,
    pub is_owner: bool,
}

impl From<ReplaceWorkspaceUsersRequest> for ReplaceWorkspaceUserAccessCommand {
    fn from(request: ReplaceWorkspaceUsersRequest) -> Self {
        Self {
            users: request
                .users
                .into_iter()
                .map(|entry| WorkspaceUserAccessDraft {
                    user_code: entry.user_code,
                    is_owner: entry.is_owner,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceWorkspaceRolesRequest {
    #[serde(default)]
    pub roles: Vec<WorkspaceRoleGrantRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRoleGrantRequest {
    pub role_code: String,
}

impl From<ReplaceWorkspaceRolesRequest> for ReplaceWorkspaceRoleAccessCommand {
    fn from(request: ReplaceWorkspaceRolesRequest) -> Self {
        Self {
            roles: request
                .roles
                .into_iter()
                .map(|entry| WorkspaceRoleAccessDraft {
                    role_code: entry.role_code,
                })
                .collect(),
        }
    }
}
