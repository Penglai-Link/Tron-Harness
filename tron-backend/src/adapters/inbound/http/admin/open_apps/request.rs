use serde::Deserialize;

use crate::{
    application::open_app::{
        CapabilityGrantInput, CreateOpenAppCommand, ReplaceCapabilityGrantsCommand,
        ReplaceUserGrantsCommand, UpdateOpenAppCommand,
    },
    domain::open_app::OpenAppStatus,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOpenAppRequest {
    pub name: String,
    pub description: Option<String>,
}

impl From<CreateOpenAppRequest> for CreateOpenAppCommand {
    fn from(request: CreateOpenAppRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOpenAppRequest {
    pub name: String,
    pub description: Option<String>,
    pub status: OpenAppStatus,
}

impl From<UpdateOpenAppRequest> for UpdateOpenAppCommand {
    fn from(request: UpdateOpenAppRequest) -> Self {
        Self {
            name: request.name,
            description: request.description,
            status: request.status,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceUserGrantsRequest {
    pub user_codes: Vec<String>,
}

impl From<ReplaceUserGrantsRequest> for ReplaceUserGrantsCommand {
    fn from(request: ReplaceUserGrantsRequest) -> Self {
        Self {
            user_codes: request.user_codes,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityGrantRequest {
    pub capability_code: String,
    pub operation_code: String,
}

impl From<CapabilityGrantRequest> for CapabilityGrantInput {
    fn from(request: CapabilityGrantRequest) -> Self {
        Self {
            capability_code: request.capability_code,
            operation_code: request.operation_code,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceCapabilityGrantsRequest {
    pub grants: Vec<CapabilityGrantRequest>,
}

impl From<ReplaceCapabilityGrantsRequest> for ReplaceCapabilityGrantsCommand {
    fn from(request: ReplaceCapabilityGrantsRequest) -> Self {
        Self {
            grants: request
                .grants
                .into_iter()
                .map(CapabilityGrantInput::from)
                .collect(),
        }
    }
}
