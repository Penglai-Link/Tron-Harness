use serde::Serialize;

use crate::domain::open_app::{OpenApiRouteGrant, OpenApp, OpenAppStatus};

#[derive(Debug, Clone)]
pub struct CreateOpenAppCommand {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateOpenAppCommand {
    pub name: String,
    pub description: Option<String>,
    pub status: OpenAppStatus,
}

#[derive(Debug, Clone)]
pub struct ReplaceUserGrantsCommand {
    pub user_codes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CapabilityGrantInput {
    pub capability_code: String,
    pub operation_code: String,
}

#[derive(Debug, Clone)]
pub struct ReplaceCapabilityGrantsCommand {
    pub grants: Vec<CapabilityGrantInput>,
}

#[derive(Debug, Clone)]
pub struct CreatedOpenApp {
    pub open_app: OpenApp,
    pub app_key: String,
    pub app_key_fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOpenAppSummary {
    pub open_app_code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: OpenAppStatus,
    pub app_key_fingerprint: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_rotated_at: Option<String>,
    pub user_grant_count: usize,
    pub capability_grant_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOpenAppUserGrant {
    pub user_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOpenAppCapabilityGrant {
    pub capability_code: String,
    pub operation_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOpenAppDetail {
    pub open_app_code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: OpenAppStatus,
    pub app_key_fingerprint: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_rotated_at: Option<String>,
    pub user_grants: Vec<ManagedOpenAppUserGrant>,
    pub capability_grants: Vec<ManagedOpenAppCapabilityGrant>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOpenAppSecretReveal {
    pub open_app_code: String,
    pub app_key: String,
    pub app_key_fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedOpenAppSecretRevealResult {
    pub open_app: ManagedOpenAppDetail,
    pub secret_reveal: ManagedOpenAppSecretReveal,
}

#[derive(Debug, Clone)]
pub struct SignedOpenApiRequest {
    pub method: String,
    pub path: String,
    pub raw_query: Option<String>,
    pub body: Vec<u8>,
    pub open_app_code: String,
    pub user_code: String,
    pub timestamp: i64,
    pub nonce: String,
    pub signature_version: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedOpenAppRequest {
    pub open_app: OpenApp,
    pub authenticated: crate::domain::auth::AuthenticatedAccess,
    pub route_grant: OpenApiRouteGrant,
}
