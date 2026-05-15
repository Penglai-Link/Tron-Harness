use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::user::{CreateUserCommand, PasswordHasher, UpdateUserCommand, UserRepository},
    config::AuthConfig,
    domain::{
        auth::{
            AuthError, AuthMethod, AuthProviderDescriptor, AuthProviderKind,
            AuthenticatedUser, CurrentSession, OAuth2IdentityLink, OAuth2Profile,
            RefreshSession,
        },
        user::{
            ListUsersQuery, PagedUsers, PasswordResetResult, User, UserError, UserStatistics,
            UserStatus, optional_trimmed, parse_user_status, required_profile_field,
            unique_preserving_order, validate_initial_password, validate_role_codes,
            validate_space_codes, validate_user_code,
        },
    },
    shared::id::SnowflakeId,
};

const DEFAULT_TIMESTAMP: &str = "2026-05-12T00:00:00Z";
const DEV_OAUTH2_CODE: &str = "dev-oauth2-code";
const DEV_OAUTH2_SUBJECT: &str = "dev-auth-operator";
const DEV_OAUTH2_EMAIL: &str = "auth.operator@tron.ai";
const DEV_OAUTH2_DISPLAY_NAME: &str = "Auth Operator";
const OAUTH_STATE_TTL_SECONDS: i64 = 300;

#[derive(Clone)]
pub struct UserService {
    repository: Arc<Mutex<Box<dyn UserRepository>>>,
    password_hasher: Arc<dyn PasswordHasher>,
    auth_config: AuthConfig,
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

impl UserService {
    pub fn new(
        repository: impl UserRepository + 'static,
        password_hasher: impl PasswordHasher + 'static,
        auth_config: AuthConfig,
    ) -> Self {
        Self {
            repository: Arc::new(Mutex::new(Box::new(repository))),
            password_hasher: Arc::new(password_hasher),
            auth_config,
        }
    }

    pub fn access_token_ttl_seconds(&self) -> i64 {
        self.auth_config.access_token_ttl_seconds
    }

    pub fn refresh_token_ttl_seconds(&self) -> i64 {
        self.auth_config.refresh_token_ttl_seconds
    }

    pub fn cookie_secure(&self) -> bool {
        self.auth_config.cookie_secure
    }

    pub fn auth_providers(&self) -> Vec<AuthProviderDescriptor> {
        let mut providers = vec![AuthProviderDescriptor {
            code: "internal".into(),
            name: "Internal Account".into(),
            kind: AuthProviderKind::Internal,
        }];

        if self.auth_config.dev_oauth2_enabled {
            providers.push(self.dev_provider_descriptor());
        }

        providers
    }

    pub fn login_internal(
        &self,
        login: &str,
        password: &str,
    ) -> Result<AuthorizedSession, AuthError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = repository
            .find_user_by_login(login)?
            .ok_or(AuthError::InvalidCredentials)?;
        ensure_active_user(&user)?;

        if !self
            .password_hasher
            .verify_password(password, &user.password_hash)?
        {
            return Err(AuthError::InvalidCredentials);
        }

        self.create_session(
            repository.as_mut(),
            &user,
            AuthMethod::Internal,
            None,
            None,
        )
    }

    pub fn oauth2_authorize(
        &self,
        provider_code: &str,
        redirect_path: Option<&str>,
    ) -> Result<OAuth2AuthorizeResult, AuthError> {
        if provider_code != "dev" || !self.auth_config.dev_oauth2_enabled {
            return Err(AuthError::ProviderNotFound);
        }

        let redirect_path = normalize_redirect_path(redirect_path);
        let now = now_seconds();
        let claims = OAuthStateClaims {
            provider: provider_code.into(),
            redirect_path: redirect_path.clone(),
            nonce: format!("oauth-state-{}", now_nanos()),
            exp: to_usize(now + OAUTH_STATE_TTL_SECONDS)?,
            iat: to_usize(now)?,
            kind: "oauth_state".into(),
        };
        let state_token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
        )
        .map_err(|_| AuthError::TokenEncodingFailed)?;

        Ok(OAuth2AuthorizeResult {
            redirect_location: format!(
                "/api/v1/auth/oauth2/callback/dev?code={DEV_OAUTH2_CODE}&state={state_token}"
            ),
            state_token,
        })
    }

    pub fn oauth2_callback(
        &self,
        provider_code: &str,
        state_cookie: Option<&str>,
        state_query: &str,
        code: &str,
    ) -> Result<AuthorizedSession, AuthError> {
        if provider_code != "dev" || !self.auth_config.dev_oauth2_enabled {
            return Err(AuthError::ProviderNotFound);
        }
        if code.trim() != DEV_OAUTH2_CODE {
            return Err(AuthError::OAuth2CodeInvalid);
        }
        if state_cookie.is_none() || state_cookie != Some(state_query) {
            return Err(AuthError::OAuth2StateInvalid);
        }

        let claims = decode::<OAuthStateClaims>(
            state_query,
            &DecodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
            &oauth_state_validation(),
        )
        .map_err(|_| AuthError::OAuth2StateInvalid)?
        .claims;
        if claims.kind != "oauth_state" || claims.provider != provider_code {
            return Err(AuthError::OAuth2StateInvalid);
        }

        let profile = OAuth2Profile {
            provider_code: "dev".into(),
            provider_name: self.auth_config.dev_oauth2_name.clone(),
            subject: DEV_OAUTH2_SUBJECT.into(),
            email: DEV_OAUTH2_EMAIL.into(),
            display_name: DEV_OAUTH2_DISPLAY_NAME.into(),
        };

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = match repository.find_user_by_provider_subject(
            &profile.provider_code,
            &profile.subject,
        )? {
            Some(user) => user,
            None => repository
                .find_active_user_by_email(&profile.email)?
                .ok_or(AuthError::UserNotLinked)?,
        };
        ensure_active_user(&user)?;

        let now = now_seconds();
        let identity = OAuth2IdentityLink {
            id: repository.next_id().map_err(map_user_error_to_auth)?,
            user_id: user.id.value(),
            user_code: user.user_code.clone(),
            provider_code: profile.provider_code.clone(),
            provider_subject: profile.subject.clone(),
            email: profile.email.to_ascii_lowercase(),
            display_name: profile.display_name.clone(),
            linked_at: now,
            updated_at: now,
        };
        repository.upsert_oauth_identity(&identity)?;

        self.create_session(
            repository.as_mut(),
            &user,
            AuthMethod::Oauth2,
            Some(self.dev_provider_descriptor()),
            Some(claims.redirect_path),
        )
    }

    pub fn current_session(
        &self,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
    ) -> Result<Option<AuthorizedSession>, AuthError> {
        if let Some(access_token) = access_token
            && let Ok(session) = self.session_from_access_token(access_token)
        {
            return Ok(Some(AuthorizedSession {
                session,
                access_token: access_token.into(),
                refresh_token: refresh_token.unwrap_or_default().into(),
                redirect_path: None,
            }));
        }

        if let Some(refresh_token) = refresh_token {
            return self.refresh_session(Some(refresh_token)).map(Some);
        }

        Ok(None)
    }

    pub fn refresh_session(
        &self,
        refresh_token: Option<&str>,
    ) -> Result<AuthorizedSession, AuthError> {
        let refresh_token = refresh_token.ok_or(AuthError::RefreshInvalid)?;
        let claims = decode_refresh_token(
            refresh_token,
            &self.auth_config.jwt_secret,
        )?;
        let now = now_seconds();

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut session = repository
            .find_refresh_session(&claims.sid)?
            .ok_or(AuthError::RefreshInvalid)?;
        if session.revoked_at.is_some() || session.expires_at <= now {
            return Err(AuthError::RefreshInvalid);
        }
        if !self
            .password_hasher
            .verify_password(refresh_token, &session.refresh_token_hash)?
        {
            return Err(AuthError::RefreshInvalid);
        }

        let user = repository
            .get_user(&session.user_code)
            .map_err(map_user_error_to_auth)?;
        ensure_active_user(&user)?;

        let refresh_token = self.refresh_token_for_session(
            &session.session_code,
            &user,
            session.auth_method,
            session.provider_code.clone(),
        )?;
        let access_token = self.access_token_for_session(
            &session.session_code,
            &user,
            session.auth_method,
            session.provider_code.clone(),
        )?;

        session.refresh_token_hash = self
            .password_hasher
            .hash_password(&refresh_token)
            .map_err(|_| AuthError::StorageFailure)?;
        session.expires_at = now + self.auth_config.refresh_token_ttl_seconds;
        session.updated_at = now;
        session.last_used_at = Some(now);
        session.revoked_at = None;
        repository.replace_refresh_session(&session)?;

        Ok(AuthorizedSession {
            session: CurrentSession {
                authenticated: true,
                auth_method: session.auth_method,
                provider: session
                    .provider_code
                    .as_ref()
                    .map(|_| self.dev_provider_descriptor()),
                user: AuthenticatedUser::from(&user),
            },
            access_token,
            refresh_token,
            redirect_path: None,
        })
    }

    pub fn logout_session(&self, refresh_token: Option<&str>) -> Result<(), AuthError> {
        let Some(refresh_token) = refresh_token else {
            return Ok(());
        };

        let claims = match decode_refresh_token(refresh_token, &self.auth_config.jwt_secret) {
            Ok(claims) => claims,
            Err(_) => return Ok(()),
        };

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.revoke_refresh_session(&claims.sid, now_seconds())
    }

    pub fn authorize_access_token(&self, access_token: Option<&str>) -> Result<(), AuthError> {
        let access_token = access_token.ok_or(AuthError::Unauthorized)?;
        self.session_from_access_token(access_token).map(|_| ())
    }

    pub fn list_users(&self, query: ListUsersQuery) -> Result<PagedUsers, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.list_users(query)
    }

    pub fn get_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.get_user(user_code)
    }

    pub fn statistics(&self) -> Result<UserStatistics, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        repository.statistics()
    }

    pub fn create_user(&self, command: CreateUserCommand) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let username = required_profile_field(command.username)?;
        let first_name = required_profile_field(command.first_name)?;
        let last_name = required_profile_field(command.last_name)?;
        let display_name = required_profile_field(command.display_name)?;
        let email = required_profile_field(command.email)?;
        let explicit_user_code = optional_trimmed(command.user_code);

        if let Some(user_code) = explicit_user_code.as_deref() {
            validate_user_code(user_code)?;
        }
        validate_role_codes(&command.role_codes)?;
        validate_space_codes(&command.space_codes)?;
        if let Some(initial_password) = command.initial_password.as_deref() {
            validate_initial_password(initial_password)?;
        }

        let id = repository.next_id()?;
        let user_code = if let Some(user_code) = explicit_user_code {
            if repository.user_code_exists(&user_code, None)? {
                return Err(UserError::DuplicateUserCode);
            }
            user_code
        } else {
            repository.next_user_code(&username)?
        };
        if repository.username_exists(&username, None)? {
            return Err(UserError::DuplicateUsername);
        }
        if repository.email_exists(&email, None)? {
            return Err(UserError::DuplicateEmail);
        }

        let initial_password = command
            .initial_password
            .unwrap_or_else(|| format!("TronInitial{id}!"));
        let password_hash = self.password_hasher.hash_password(&initial_password)?;

        let user = User {
            id: SnowflakeId::new(id),
            user_code: user_code.clone(),
            username,
            first_name,
            last_name,
            display_name,
            email,
            phone: command.phone,
            avatar_url: command.avatar_url,
            locale: command.locale.unwrap_or_else(|| "en-US".into()),
            timezone: command
                .timezone
                .unwrap_or_else(|| "America/Los_Angeles".into()),
            status: UserStatus::Active,
            role_codes: command.role_codes,
            space_codes: command.space_codes,
            password_hash,
            password_reset_required: command.password_reset_required.unwrap_or(true),
            last_active_at: None,
            created_at: DEFAULT_TIMESTAMP.into(),
            updated_at: DEFAULT_TIMESTAMP.into(),
            archived_at: None,
        };

        repository.insert_user(&user)?;
        Ok(user)
    }

    pub fn patch_user(
        &self,
        user_code: &str,
        command: UpdateUserCommand,
    ) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;

        if let Some(username) = command.username {
            if repository.username_exists(&username, Some(user_code))? {
                return Err(UserError::DuplicateUsername);
            }
            user.username = username;
        }
        if let Some(first_name) = command.first_name {
            user.first_name = first_name;
        }
        if let Some(last_name) = command.last_name {
            user.last_name = last_name;
        }
        if let Some(display_name) = command.display_name {
            user.display_name = display_name;
        }
        if let Some(email) = command.email {
            if repository.email_exists(&email, Some(user_code))? {
                return Err(UserError::DuplicateEmail);
            }
            user.email = email;
        }
        if let Some(phone) = command.phone {
            user.phone = phone;
        }
        if let Some(avatar_url) = command.avatar_url {
            user.avatar_url = avatar_url;
        }
        if let Some(locale) = command.locale {
            user.locale = locale;
        }
        if let Some(timezone) = command.timezone {
            user.timezone = timezone;
        }
        if let Some(status) = command.status {
            let status = parse_user_status(&status)?;
            user.status = status;
            user.archived_at = match status {
                UserStatus::Active => None,
                UserStatus::Archived => Some(DEFAULT_TIMESTAMP.into()),
            };
        }
        user.updated_at = DEFAULT_TIMESTAMP.into();

        repository.update_user(&user)?;
        Ok(user.clone())
    }

    pub fn archive_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.status = UserStatus::Archived;
        user.archived_at = Some(DEFAULT_TIMESTAMP.into());
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.update_user(&user)?;
        Ok(user.clone())
    }

    pub fn restore_user(&self, user_code: &str) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.status = UserStatus::Active;
        user.archived_at = None;
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.update_user(&user)?;
        Ok(user.clone())
    }

    pub fn replace_roles(
        &self,
        user_code: &str,
        role_codes: Vec<String>,
    ) -> Result<User, UserError> {
        validate_role_codes(&role_codes)?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.role_codes = unique_preserving_order(role_codes);
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.replace_role_codes(&user)?;
        Ok(user.clone())
    }

    pub fn replace_spaces(
        &self,
        user_code: &str,
        space_codes: Vec<String>,
    ) -> Result<User, UserError> {
        validate_space_codes(&space_codes)?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.space_codes = unique_preserving_order(space_codes);
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.replace_space_codes(&user)?;
        Ok(user.clone())
    }

    pub fn reset_password(&self, user_code: &str) -> Result<PasswordResetResult, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        let reset_id = repository.next_id()?;
        let temporary_password = format!("TronTmp{reset_id}!");
        let password_hash = self.password_hasher.hash_password(&temporary_password)?;

        user.password_hash = password_hash;
        user.password_reset_required = true;
        user.updated_at = DEFAULT_TIMESTAMP.into();
        repository.record_password_reset(&user, reset_id)?;

        Ok(PasswordResetResult {
            user_code: user.user_code.clone(),
            temporary_password,
            password_reset_required: true,
        })
    }

    fn create_session(
        &self,
        repository: &mut dyn UserRepository,
        user: &User,
        auth_method: AuthMethod,
        provider: Option<AuthProviderDescriptor>,
        redirect_path: Option<String>,
    ) -> Result<AuthorizedSession, AuthError> {
        let now = now_seconds();
        let session_id = repository.next_id().map_err(map_user_error_to_auth)?;
        let session_code = format!("SESS-{session_id}");
        let access_token = self.access_token_for_session(
            &session_code,
            user,
            auth_method,
            provider.as_ref().map(|descriptor| descriptor.code.clone()),
        )?;
        let refresh_token = self.refresh_token_for_session(
            &session_code,
            user,
            auth_method,
            provider.as_ref().map(|descriptor| descriptor.code.clone()),
        )?;

        let session = RefreshSession {
            id: session_id,
            session_code: session_code.clone(),
            user_id: user.id.value(),
            user_code: user.user_code.clone(),
            auth_method,
            provider_code: provider.as_ref().map(|descriptor| descriptor.code.clone()),
            refresh_token_hash: self
                .password_hasher
                .hash_password(&refresh_token)
                .map_err(|_| AuthError::StorageFailure)?,
            expires_at: now + self.auth_config.refresh_token_ttl_seconds,
            created_at: now,
            updated_at: now,
            last_used_at: Some(now),
            revoked_at: None,
        };
        repository.create_refresh_session(&session)?;

        Ok(AuthorizedSession {
            session: CurrentSession {
                authenticated: true,
                auth_method,
                provider,
                user: AuthenticatedUser::from(user),
            },
            access_token,
            refresh_token,
            redirect_path,
        })
    }

    fn session_from_access_token(&self, access_token: &str) -> Result<CurrentSession, AuthError> {
        let claims = decode_access_token(access_token, &self.auth_config.jwt_secret)?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = repository
            .get_user(&claims.sub)
            .map_err(map_user_error_to_auth)?;
        ensure_active_user(&user)?;

        Ok(CurrentSession {
            authenticated: true,
            auth_method: auth_method_from_claims(&claims.auth_method)?,
            provider: claims.provider_code.map(|_| self.dev_provider_descriptor()),
            user: AuthenticatedUser::from(&user),
        })
    }

    fn access_token_for_session(
        &self,
        session_code: &str,
        user: &User,
        auth_method: AuthMethod,
        provider_code: Option<String>,
    ) -> Result<String, AuthError> {
        let now = now_seconds();
        let claims = AccessTokenClaims {
            sub: user.user_code.clone(),
            sid: session_code.into(),
            auth_method: auth_method_as_str(auth_method).into(),
            provider_code,
            exp: to_usize(now + self.auth_config.access_token_ttl_seconds)?,
            iat: to_usize(now)?,
            kind: "access".into(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
        )
        .map_err(|_| AuthError::TokenEncodingFailed)
    }

    fn refresh_token_for_session(
        &self,
        session_code: &str,
        user: &User,
        auth_method: AuthMethod,
        provider_code: Option<String>,
    ) -> Result<String, AuthError> {
        let now = now_seconds();
        let claims = AccessTokenClaims {
            sub: user.user_code.clone(),
            sid: session_code.into(),
            auth_method: auth_method_as_str(auth_method).into(),
            provider_code,
            exp: to_usize(now + self.auth_config.refresh_token_ttl_seconds)?,
            iat: to_usize(now)?,
            kind: "refresh".into(),
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
        )
        .map_err(|_| AuthError::TokenEncodingFailed)
    }

    fn dev_provider_descriptor(&self) -> AuthProviderDescriptor {
        AuthProviderDescriptor {
            code: "dev".into(),
            name: self.auth_config.dev_oauth2_name.clone(),
            kind: AuthProviderKind::Oauth2,
        }
    }
}

fn decode_access_token(token: &str, secret: &str) -> Result<AccessTokenClaims, AuthError> {
    let claims = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &token_validation(),
    )
    .map_err(|_| AuthError::Unauthorized)?
    .claims;
    if claims.kind != "access" {
        return Err(AuthError::Unauthorized);
    }
    Ok(claims)
}

fn decode_refresh_token(token: &str, secret: &str) -> Result<AccessTokenClaims, AuthError> {
    let claims = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &token_validation(),
    )
    .map_err(|_| AuthError::RefreshInvalid)?
    .claims;
    if claims.kind != "refresh" {
        return Err(AuthError::RefreshInvalid);
    }
    Ok(claims)
}

fn token_validation() -> Validation {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation
}

fn oauth_state_validation() -> Validation {
    token_validation()
}

fn auth_method_from_claims(value: &str) -> Result<AuthMethod, AuthError> {
    match value {
        "internal" => Ok(AuthMethod::Internal),
        "oauth2" => Ok(AuthMethod::Oauth2),
        _ => Err(AuthError::Unauthorized),
    }
}

fn auth_method_as_str(value: AuthMethod) -> &'static str {
    match value {
        AuthMethod::Internal => "internal",
        AuthMethod::Oauth2 => "oauth2",
    }
}

fn ensure_active_user(user: &User) -> Result<(), AuthError> {
    if user.status == UserStatus::Active {
        Ok(())
    } else {
        Err(AuthError::UserInactive)
    }
}

fn map_user_error_to_auth(error: UserError) -> AuthError {
    match error {
        UserError::NotFound => AuthError::Unauthorized,
        _ => AuthError::StorageFailure,
    }
}

fn normalize_redirect_path(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some(path) if path.starts_with('/') && !path.starts_with("//") => path.to_owned(),
        _ => "/dashboard".into(),
    }
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_secs() as i64
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos()
}

fn to_usize(value: i64) -> Result<usize, AuthError> {
    usize::try_from(value).map_err(|_| AuthError::TokenEncodingFailed)
}
