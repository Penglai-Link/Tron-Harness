use serde::Deserialize;

use crate::application::user::{StepUpVariableRevealScope, StepUpVariableRevealTarget};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalLoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyStepUpPasswordRequest {
    pub password: String,
    pub target: VerifyStepUpTargetRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyStepUpTargetRequest {
    pub scope: VerifyStepUpTargetScope,
    pub workspace_code: Option<String>,
    pub variable_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerifyStepUpTargetScope {
    Platform,
    Workspace,
}

impl From<VerifyStepUpTargetRequest> for StepUpVariableRevealTarget {
    fn from(request: VerifyStepUpTargetRequest) -> Self {
        let scope = match request.scope {
            VerifyStepUpTargetScope::Platform => StepUpVariableRevealScope::Platform,
            VerifyStepUpTargetScope::Workspace => StepUpVariableRevealScope::Workspace,
        };
        Self {
            scope,
            workspace_code: request.workspace_code,
            variable_key: request.variable_key,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OAuth2AuthorizeQuery {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuth2CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}
