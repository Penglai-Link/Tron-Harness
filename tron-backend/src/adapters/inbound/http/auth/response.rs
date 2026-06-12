use serde::Serialize;

use crate::domain::auth::{AuthProviderDescriptor, CurrentSession};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProvidersDto {
    pub providers: Vec<AuthProviderDescriptor>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthGrantDto {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub session: CurrentSession,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutDto {
    pub logged_out: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepUpTokenDto {
    pub step_up_token: String,
    pub expires_at: String,
}
