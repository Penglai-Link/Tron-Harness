use serde::{Deserialize, Serialize};

use crate::shared::id::SnowflakeId;

use super::OpenAppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenAppStatus {
    Enabled,
    Disabled,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAppCredentialStatus {
    Active,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApp {
    pub id: SnowflakeId,
    pub open_app_code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: OpenAppStatus,
    pub created_at: String,
    pub updated_at: String,
    pub last_rotated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAppCredential {
    pub id: SnowflakeId,
    pub credential_code: String,
    pub open_app_code: String,
    pub status: OpenAppCredentialStatus,
    pub secret_ciphertext: String,
    pub secret_fingerprint: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAppUserGrant {
    pub id: SnowflakeId,
    pub grant_code: String,
    pub open_app_code: String,
    pub user_code: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAppCapabilityGrant {
    pub id: SnowflakeId,
    pub grant_code: String,
    pub open_app_code: String,
    pub capability_code: String,
    pub operation_code: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiNonceRecord {
    pub id: SnowflakeId,
    pub nonce_code: String,
    pub open_app_code: String,
    pub nonce: String,
    pub signed_at: i64,
    pub expires_at: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAppAuthState {
    pub open_app: OpenApp,
    pub active_credential: Option<OpenAppCredential>,
    pub user_grants: Vec<OpenAppUserGrant>,
    pub capability_grants: Vec<OpenAppCapabilityGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApiRouteGrant {
    pub capability_code: &'static str,
    pub operation_code: &'static str,
}

#[derive(Debug, Clone)]
pub struct OpenApiCanonicalRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub raw_query: Option<&'a str>,
    pub body: &'a [u8],
    pub open_app_code: &'a str,
    pub user_code: &'a str,
    pub timestamp: i64,
    pub nonce: &'a str,
}

pub fn open_app_status_to_storage(status: OpenAppStatus) -> &'static str {
    match status {
        OpenAppStatus::Enabled => "enabled",
        OpenAppStatus::Disabled => "disabled",
        OpenAppStatus::Archived => "archived",
    }
}

pub fn open_app_status_from_storage(value: &str) -> Result<OpenAppStatus, OpenAppError> {
    match value {
        "enabled" => Ok(OpenAppStatus::Enabled),
        "disabled" => Ok(OpenAppStatus::Disabled),
        "archived" => Ok(OpenAppStatus::Archived),
        _ => Err(OpenAppError::StorageFailure),
    }
}

pub fn open_app_credential_status_to_storage(status: OpenAppCredentialStatus) -> &'static str {
    match status {
        OpenAppCredentialStatus::Active => "active",
        OpenAppCredentialStatus::Revoked => "revoked",
    }
}

pub fn open_app_credential_status_from_storage(
    value: &str,
) -> Result<OpenAppCredentialStatus, OpenAppError> {
    match value {
        "active" => Ok(OpenAppCredentialStatus::Active),
        "revoked" => Ok(OpenAppCredentialStatus::Revoked),
        _ => Err(OpenAppError::StorageFailure),
    }
}
