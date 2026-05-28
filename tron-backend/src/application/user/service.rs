use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    application::user::{CreateUserCommand, PasswordHasher, UpdateUserCommand, UserRepository},
    config::{AuthConfig, GitHubOAuthConfig},
    domain::{
        auth::{
            AuthError, AuthMethod, AuthProviderDescriptor, AuthProviderKind, AuthenticatedUser,
            CurrentSession, OAuth2IdentityLink, OAuth2Profile, RefreshSession,
        },
        user::{
            ListUsersQuery, PagedUsers, PasswordResetResult, User, UserError, UserSourceType,
            UserStatistics, UserStatus, normalize_code_segment, optional_trimmed,
            parse_user_status, required_profile_field, unique_preserving_order,
            validate_initial_password, validate_role_codes, validate_space_codes,
            validate_user_code,
        },
    },
    shared::{
        event_bus::{BusEventDraft, BusPriority, EventBusHandle},
        id::SnowflakeId,
        time::current_timestamp,
    },
};

mod auth_oauth;
mod auth_session;
mod auth_use_cases;
mod events;
mod user_use_cases;

use auth_session::{
    auth_method_name, decode_refresh_token, ensure_active_user, map_user_error_to_auth, now_nanos,
    now_seconds, to_usize, token_validation,
};

const DEV_OAUTH2_CODE: &str = "dev-oauth2-code";
const DEV_OAUTH2_SUBJECT: &str = "dev-auth-operator";
const DEV_OAUTH2_LOGIN: &str = "auth.operator";
const DEV_OAUTH2_EMAIL: &str = "auth.operator@tron.ai";
const DEV_OAUTH2_DISPLAY_NAME: &str = "Auth Operator";
const GITHUB_PROVIDER_CODE: &str = "github";
const GITHUB_USER_AGENT: &str = "TronAuth/0.1";
const GITHUB_AUTO_PROVISION_DEFAULT_ROLE_CODE: &str = "VIEWER";
const GITHUB_AUTO_PROVISION_DEFAULT_SPACE_CODE: &str = "PLATFORM";
const GITHUB_AUTO_PROVISION_DEFAULT_LOCALE: &str = "en-US";
const GITHUB_AUTO_PROVISION_DEFAULT_TIMEZONE: &str = "UTC";
const OAUTH2_AUTO_PROVISION_FALLBACK_NAME: &str = "User";
const OAUTH_STATE_TTL_SECONDS: i64 = 300;

#[derive(Clone)]
pub struct UserService {
    repository: Arc<Mutex<Box<dyn UserRepository>>>,
    password_hasher: Arc<dyn PasswordHasher>,
    auth_config: AuthConfig,
    oauth_client: Client,
    event_bus: EventBusHandle,
}

#[derive(Debug, Clone)]
pub struct OAuth2AuthorizeResult {
    pub redirect_location: String,
    pub state_token: String,
}

#[derive(Debug, Clone)]
pub struct AuthorizedSession {
    pub session: CurrentSession,
    pub access_token: String,
    pub refresh_token: String,
    pub redirect_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AccessTokenClaims {
    sub: String,
    sid: String,
    auth_method: String,
    provider_code: Option<String>,
    exp: usize,
    iat: usize,
    kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OAuthStateClaims {
    provider: String,
    redirect_path: String,
    nonce: String,
    exp: usize,
    iat: usize,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUserResponse {
    id: i64,
    login: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubEmailResponse {
    email: String,
    primary: bool,
    verified: bool,
}

impl UserService {
    pub fn new(
        repository: impl UserRepository + 'static,
        password_hasher: impl PasswordHasher + 'static,
        auth_config: AuthConfig,
    ) -> Self {
        Self::with_event_bus(
            repository,
            password_hasher,
            auth_config,
            EventBusHandle::disabled(),
        )
    }

    pub fn with_event_bus(
        repository: impl UserRepository + 'static,
        password_hasher: impl PasswordHasher + 'static,
        auth_config: AuthConfig,
        event_bus: EventBusHandle,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            password_hasher: Arc::new(password_hasher),
            auth_config,
            oauth_client: Client::new(),
            event_bus,
        }
    }
}
