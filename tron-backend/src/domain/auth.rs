use serde::Serialize;

use crate::domain::user::{User, UserStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredentials,
    UserInactive,
    Unauthorized,
    RefreshInvalid,
    ProviderNotFound,
    UserNotLinked,
    OAuth2StateInvalid,
    OAuth2CodeInvalid,
    OAuth2ProviderError,
    OAuth2ProfileInvalid,
    OAuth2UpstreamFailed,
    TokenEncodingFailed,
    TokenDecodingFailed,
    PasswordVerifyFailed,
    StorageFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    Internal,
    Oauth2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthProviderKind {
    Internal,
    Oauth2,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthProviderDescriptor {
    pub code: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: AuthProviderKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedUser {
    pub user_code: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role_codes: Vec<String>,
    pub space_codes: Vec<String>,
    pub status: UserStatus,
    pub password_reset_required: bool,
}

impl From<&User> for AuthenticatedUser {
    fn from(user: &User) -> Self {
        Self {
            user_code: user.user_code.clone(),
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            email: user.email.clone(),
            role_codes: user.role_codes.clone(),
            space_codes: user.space_codes.clone(),
            status: user.status,
            password_reset_required: user.password_reset_required,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSession {
    pub authenticated: bool,
    pub auth_method: AuthMethod,
    pub provider: Option<AuthProviderDescriptor>,
    pub user: AuthenticatedUser,
}

#[derive(Debug, Clone)]
pub struct RefreshSession {
    pub id: i64,
    pub session_code: String,
    pub user_id: i64,
    pub user_code: String,
    pub auth_method: AuthMethod,
    pub provider_code: Option<String>,
    pub refresh_token_hash: String,
    pub expires_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OAuth2IdentityLink {
    pub id: i64,
    pub user_id: i64,
    pub user_code: String,
    pub provider_code: String,
    pub provider_subject: String,
    pub email: String,
    pub display_name: String,
    pub linked_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct OAuth2Profile {
    pub provider_code: String,
    pub provider_name: String,
    pub subject: String,
    pub login: String,
    pub email: String,
    pub display_name: String,
}
