use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthProviderKind {
    Internal,
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
    pub avatar_url: Option<String>,
    pub user_code: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSession {
    pub authenticated: bool,
    pub auth_method: AuthMethod,
    pub provider: Option<AuthProviderDescriptor>,
    pub user: AuthenticatedUser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessTokenIdentity {
    pub user_code: String,
    pub session_code: String,
    pub auth_method: AuthMethod,
    pub provider_code: Option<String>,
}
