use serde::Serialize;

use crate::domain::auth::AuthProviderDescriptor;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProvidersDto {
    pub providers: Vec<AuthProviderDescriptor>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutDto {
    pub logged_out: bool,
}
