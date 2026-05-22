use serde::Deserialize;

use crate::{
    application::app::{CreateAppCommand, UpdateAppCommand},
    domain::app::AppRuntimeRole,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAppRequest {
    pub name: String,
    pub description: Option<String>,
    pub local_directory: String,
    pub runtime_role: AppRuntimeRole,
    pub repository_url: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl From<CreateAppRequest> for CreateAppCommand {
    fn from(request: CreateAppRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
            local_directory: request.local_directory,
            runtime_role: request.runtime_role,
            repository_url: request.repository_url,
            default_branch: request.default_branch,
            tags: request.tags,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAppRequest {
    pub name: String,
    pub description: Option<String>,
    pub local_directory: String,
    pub runtime_role: AppRuntimeRole,
    pub repository_url: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl From<UpdateAppRequest> for UpdateAppCommand {
    fn from(request: UpdateAppRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
            local_directory: request.local_directory,
            runtime_role: request.runtime_role,
            repository_url: request.repository_url,
            default_branch: request.default_branch,
            tags: request.tags,
        }
    }
}
