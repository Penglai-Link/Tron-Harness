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

        if let Some(provider) = self.github_provider_descriptor_if_enabled() {
            providers.push(provider);
        }
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

        let session =
            self.create_session(repository.as_mut(), &user, AuthMethod::Internal, None, None)?;
        drop(repository);
        self.publish_auth_session_event(
            "auth.session.created",
            "Created authenticated session",
            &session.session.user,
            Some(AuthMethod::Internal),
            session
                .session
                .provider
                .as_ref()
                .map(|provider| provider.code.as_str()),
            None,
            BusPriority::Critical,
        );
        Ok(session)
    }

    pub fn oauth2_authorize(
        &self,
        provider_code: &str,
        redirect_path: Option<&str>,
    ) -> Result<OAuth2AuthorizeResult, AuthError> {
        match provider_code {
            GITHUB_PROVIDER_CODE => self.github_oauth2_authorize(redirect_path),
            "dev" => self.dev_oauth2_authorize(redirect_path),
            _ => Err(AuthError::ProviderNotFound),
        }
    }

    pub async fn oauth2_callback(
        &self,
        provider_code: &str,
        state_cookie: Option<&str>,
        state_query: Option<&str>,
        code: Option<&str>,
        provider_error: Option<&str>,
    ) -> Result<AuthorizedSession, AuthError> {
        match provider_code {
            GITHUB_PROVIDER_CODE => {
                self.github_oauth2_callback(state_cookie, state_query, code, provider_error)
                    .await
            }
            "dev" => self.dev_oauth2_callback(state_cookie, state_query, code),
            _ => Err(AuthError::ProviderNotFound),
        }
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
        let claims = decode_refresh_token(refresh_token, &self.auth_config.jwt_secret)?;
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

        let authorized_session = AuthorizedSession {
            session: CurrentSession {
                authenticated: true,
                auth_method: session.auth_method,
                provider: session
                    .provider_code
                    .as_deref()
                    .and_then(|value| self.session_provider_descriptor(value)),
                user: AuthenticatedUser::from(&user),
            },
            access_token,
            refresh_token,
            redirect_path: None,
        };
        drop(repository);
        self.publish_auth_session_event(
            "auth.session.refreshed",
            "Refreshed authenticated session",
            &authorized_session.session.user,
            Some(authorized_session.session.auth_method),
            authorized_session
                .session
                .provider
                .as_ref()
                .map(|provider| provider.code.as_str()),
            Some(session.session_code.as_str()),
            BusPriority::Standard,
        );
        Ok(authorized_session)
    }

    fn dev_oauth2_authorize(
        &self,
        redirect_path: Option<&str>,
    ) -> Result<OAuth2AuthorizeResult, AuthError> {
        if !self.auth_config.dev_oauth2_enabled {
            return Err(AuthError::ProviderNotFound);
        }

        let redirect_path = normalize_redirect_path(redirect_path);
        let state_token = self.oauth_state_token("dev", &redirect_path)?;

        Ok(OAuth2AuthorizeResult {
            redirect_location: format!(
                "/api/v1/auth/oauth2/callback/dev?code={DEV_OAUTH2_CODE}&state={state_token}"
            ),
            state_token,
        })
    }

    fn github_oauth2_authorize(
        &self,
        redirect_path: Option<&str>,
    ) -> Result<OAuth2AuthorizeResult, AuthError> {
        let github = self.github_oauth_config()?;
        let redirect_path = normalize_redirect_path(redirect_path);
        let state_token = self.oauth_state_token(GITHUB_PROVIDER_CODE, &redirect_path)?;
        let mut authorize_url =
            Url::parse(&github.authorize_url).map_err(|_| AuthError::OAuth2UpstreamFailed)?;
        authorize_url
            .query_pairs_mut()
            .append_pair("client_id", &github.client_id)
            .append_pair("redirect_uri", &github.redirect_uri)
            .append_pair("scope", &github.scope)
            .append_pair("state", &state_token);

        Ok(OAuth2AuthorizeResult {
            redirect_location: authorize_url.to_string(),
            state_token,
        })
    }

    fn dev_oauth2_callback(
        &self,
        state_cookie: Option<&str>,
        state_query: Option<&str>,
        code: Option<&str>,
    ) -> Result<AuthorizedSession, AuthError> {
        if !self.auth_config.dev_oauth2_enabled {
            return Err(AuthError::ProviderNotFound);
        }
        let code = code.ok_or(AuthError::OAuth2CodeInvalid)?;
        if code.trim() != DEV_OAUTH2_CODE {
            return Err(AuthError::OAuth2CodeInvalid);
        }
        let claims = self.validate_oauth_state("dev", state_cookie, state_query)?;

        let profile = OAuth2Profile {
            provider_code: "dev".into(),
            provider_name: self.auth_config.dev_oauth2_name.clone(),
            subject: DEV_OAUTH2_SUBJECT.into(),
            login: DEV_OAUTH2_LOGIN.into(),
            email: DEV_OAUTH2_EMAIL.into(),
            display_name: DEV_OAUTH2_DISPLAY_NAME.into(),
        };

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = match repository
            .find_user_by_provider_subject(&profile.provider_code, &profile.subject)?
        {
            Some(user) => user,
            None => match repository.find_active_user_by_email(&profile.email)? {
                Some(user) => user,
                None => self.auto_provision_oauth2_user(repository.as_mut(), &profile)?,
            },
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

        let session = self.create_session(
            repository.as_mut(),
            &user,
            AuthMethod::Oauth2,
            Some(self.dev_provider_descriptor()),
            Some(claims.redirect_path),
        )?;
        drop(repository);
        self.publish_auth_session_event(
            "auth.session.created",
            "Created OAuth2 session",
            &session.session.user,
            Some(session.session.auth_method),
            session
                .session
                .provider
                .as_ref()
                .map(|provider| provider.code.as_str()),
            None,
            BusPriority::Critical,
        );
        Ok(session)
    }

    async fn github_oauth2_callback(
        &self,
        state_cookie: Option<&str>,
        state_query: Option<&str>,
        code: Option<&str>,
        provider_error: Option<&str>,
    ) -> Result<AuthorizedSession, AuthError> {
        let claims = self.validate_oauth_state(GITHUB_PROVIDER_CODE, state_cookie, state_query)?;
        if provider_error.is_some() {
            return Err(AuthError::OAuth2ProviderError);
        }

        let code = code.ok_or(AuthError::OAuth2CodeInvalid)?;
        let profile = self.github_profile_from_code(code).await?;

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = match repository
            .find_user_by_provider_subject(&profile.provider_code, &profile.subject)?
        {
            Some(user) => user,
            None => match repository.find_active_user_by_email(&profile.email)? {
                Some(user) => user,
                None => self.auto_provision_oauth2_user(repository.as_mut(), &profile)?,
            },
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

        let session = self.create_session(
            repository.as_mut(),
            &user,
            AuthMethod::Oauth2,
            Some(self.github_provider_descriptor()),
            Some(claims.redirect_path),
        )?;
        drop(repository);
        self.publish_auth_session_event(
            "auth.session.created",
            "Created OAuth2 session",
            &session.session.user,
            Some(session.session.auth_method),
            session
                .session
                .provider
                .as_ref()
                .map(|provider| provider.code.as_str()),
            None,
            BusPriority::Critical,
        );
        Ok(session)
    }

    pub fn logout_session(&self, refresh_token: Option<&str>) -> Result<(), AuthError> {
        let session_code = refresh_token.and_then(|token| {
            decode_refresh_token(token, &self.auth_config.jwt_secret)
                .ok()
                .map(|claims| claims.sid)
        });
        let Some(session_code) = session_code else {
            return Ok(());
        };

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let session = repository.find_refresh_session(&session_code)?;
        repository.revoke_refresh_session(&session_code, now_seconds())?;
        drop(repository);
        if let Some(session) = session {
            let _ = self.event_bus.publish(
                BusEventDraft::audit("auth.session.revoked", "auth_service")
                    .priority(BusPriority::Standard)
                    .subject("session", session.session_code)
                    .summary("Revoked authenticated session")
                    .attr("userCode", session.user_code)
                    .attr("authMethod", auth_method_name(session.auth_method))
                    .attr(
                        "providerCode",
                        session.provider_code.unwrap_or_else(|| "internal".into()),
                    ),
            );
        }
        Ok(())
    }

    pub fn authenticate_access_token(
        &self,
        access_token: Option<&str>,
    ) -> Result<CurrentSession, AuthError> {
        let access_token = access_token.ok_or(AuthError::Unauthorized)?;
        self.session_from_access_token(access_token)
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

    pub fn create_user(
        &self,
        actor: &AuthenticatedUser,
        command: CreateUserCommand,
    ) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = self.create_user_in_repository(repository.as_mut(), command)?;
        drop(repository);
        self.publish_user_event(
            "user.created",
            "Created user",
            actor,
            &user,
            BusPriority::Standard,
        );
        Ok(user)
    }

    fn create_user_in_repository(
        &self,
        repository: &mut dyn UserRepository,
        command: CreateUserCommand,
    ) -> Result<User, UserError> {
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
            source_type: UserSourceType::Internal,
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
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            archived_at: None,
        };

        repository.insert_user(&user)?;
        Ok(user)
    }

    pub fn patch_user(
        &self,
        actor: &AuthenticatedUser,
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
                UserStatus::Archived => Some(current_timestamp()),
            };
        }
        user.updated_at = current_timestamp();

        repository.update_user(&user)?;
        let updated = user.clone();
        drop(repository);
        self.publish_user_event(
            "user.updated",
            "Updated user profile",
            actor,
            &updated,
            BusPriority::Standard,
        );
        Ok(updated)
    }

    pub fn archive_user(
        &self,
        actor: &AuthenticatedUser,
        user_code: &str,
    ) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.status = UserStatus::Archived;
        user.archived_at = Some(current_timestamp());
        user.updated_at = current_timestamp();
        repository.update_user(&user)?;
        let archived = user.clone();
        drop(repository);
        self.publish_user_event(
            "user.archived",
            "Archived user",
            actor,
            &archived,
            BusPriority::Critical,
        );
        Ok(archived)
    }

    pub fn restore_user(
        &self,
        actor: &AuthenticatedUser,
        user_code: &str,
    ) -> Result<User, UserError> {
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let mut user = repository.get_user(user_code)?;
        user.status = UserStatus::Active;
        user.archived_at = None;
        user.updated_at = current_timestamp();
        repository.update_user(&user)?;
        let restored = user.clone();
        drop(repository);
        self.publish_user_event(
            "user.restored",
            "Restored user",
            actor,
            &restored,
            BusPriority::Standard,
        );
        Ok(restored)
    }

    pub fn replace_roles(
        &self,
        actor: &AuthenticatedUser,
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
        user.updated_at = current_timestamp();
        repository.replace_role_codes(&user)?;
        let updated = user.clone();
        drop(repository);
        self.publish_user_event(
            "user.roles_replaced",
            "Replaced user role assignments",
            actor,
            &updated,
            BusPriority::Standard,
        );
        Ok(updated)
    }

    pub fn replace_spaces(
        &self,
        actor: &AuthenticatedUser,
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
        user.updated_at = current_timestamp();
        repository.replace_space_codes(&user)?;
        let updated = user.clone();
        drop(repository);
        self.publish_user_event(
            "user.spaces_replaced",
            "Replaced user space assignments",
            actor,
            &updated,
            BusPriority::Standard,
        );
        Ok(updated)
    }

    pub fn reset_password(
        &self,
        actor: &AuthenticatedUser,
        user_code: &str,
    ) -> Result<PasswordResetResult, UserError> {
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
        user.updated_at = current_timestamp();
        repository.record_password_reset(&user, reset_id)?;

        let result = PasswordResetResult {
            user_code: user.user_code.clone(),
            temporary_password,
            password_reset_required: true,
        };
        drop(repository);
        let _ = self.event_bus.publish(
            BusEventDraft::audit("user.password_reset", "user_service")
                .priority(BusPriority::Critical)
                .subject("user", result.user_code.clone())
                .actor(actor)
                .summary("Reset user password")
                .attr(
                    "passwordResetRequired",
                    result.password_reset_required.to_string(),
                ),
        );
        Ok(result)
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

    fn publish_user_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        actor: &AuthenticatedUser,
        user: &User,
        priority: BusPriority,
    ) {
        let _ = self.event_bus.publish(
            BusEventDraft::audit(event_name, "user_service")
                .priority(priority)
                .subject("user", user.user_code.clone())
                .actor(actor)
                .summary(summary)
                .attr("status", format!("{:?}", user.status))
                .attr("roleCount", user.role_codes.len().to_string())
                .attr("spaceCount", user.space_codes.len().to_string())
                .attr(
                    "passwordResetRequired",
                    user.password_reset_required.to_string(),
                ),
        );
    }

    fn publish_auth_session_event(
        &self,
        event_name: &'static str,
        summary: &'static str,
        user: &AuthenticatedUser,
        auth_method: Option<AuthMethod>,
        provider_code: Option<&str>,
        session_code: Option<&str>,
        priority: BusPriority,
    ) {
        let mut draft = BusEventDraft::audit(event_name, "auth_service")
            .priority(priority)
            .subject("user", user.user_code.clone())
            .actor(user)
            .summary(summary)
            .attr(
                "authMethod",
                auth_method
                    .map(auth_method_name)
                    .unwrap_or("unknown")
                    .to_string(),
            )
            .attr(
                "providerCode",
                provider_code.unwrap_or("internal").to_string(),
            );
        if let Some(session_code) = session_code {
            draft = draft.attr("sessionCode", session_code.to_string());
        }
        let _ = self.event_bus.publish(draft);
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
            provider: claims
                .provider_code
                .as_deref()
                .and_then(|value| self.session_provider_descriptor(value)),
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

    fn github_provider_descriptor_if_enabled(&self) -> Option<AuthProviderDescriptor> {
        self.auth_config
            .github_oauth
            .as_ref()
            .map(|config| AuthProviderDescriptor {
                code: GITHUB_PROVIDER_CODE.into(),
                name: config.name.clone(),
                kind: AuthProviderKind::Oauth2,
            })
    }

    fn github_provider_descriptor(&self) -> AuthProviderDescriptor {
        self.github_provider_descriptor_if_enabled()
            .unwrap_or(AuthProviderDescriptor {
                code: GITHUB_PROVIDER_CODE.into(),
                name: "GitHub".into(),
                kind: AuthProviderKind::Oauth2,
            })
    }

    fn session_provider_descriptor(&self, provider_code: &str) -> Option<AuthProviderDescriptor> {
        match provider_code {
            GITHUB_PROVIDER_CODE => Some(self.github_provider_descriptor()),
            "dev" => Some(self.dev_provider_descriptor()),
            _ => None,
        }
    }

    fn github_oauth_config(&self) -> Result<&GitHubOAuthConfig, AuthError> {
        self.auth_config
            .github_oauth
            .as_ref()
            .ok_or(AuthError::ProviderNotFound)
    }

    fn auto_provision_oauth2_user(
        &self,
        repository: &mut dyn UserRepository,
        profile: &OAuth2Profile,
    ) -> Result<User, AuthError> {
        let (first_name, last_name) = split_display_name(&profile.display_name);
        let username = self
            .next_available_oauth_username(repository, &profile.login, &profile.email)
            .map_err(map_user_error_to_auth)?;
        let command = CreateUserCommand {
            user_code: None,
            username: Some(username),
            first_name: Some(first_name),
            last_name: Some(last_name),
            display_name: Some(profile.display_name.clone()),
            email: Some(profile.email.clone()),
            phone: None,
            avatar_url: None,
            locale: Some(GITHUB_AUTO_PROVISION_DEFAULT_LOCALE.into()),
            timezone: Some(GITHUB_AUTO_PROVISION_DEFAULT_TIMEZONE.into()),
            initial_password: Some(self.oauth2_auto_provision_password(profile)),
            password_reset_required: Some(true),
            role_codes: vec![GITHUB_AUTO_PROVISION_DEFAULT_ROLE_CODE.into()],
            space_codes: vec![GITHUB_AUTO_PROVISION_DEFAULT_SPACE_CODE.into()],
        };
        self.create_user_in_repository(repository, command)
            .map_err(map_user_error_to_auth)
    }

    fn next_available_oauth_username(
        &self,
        repository: &mut dyn UserRepository,
        login_hint: &str,
        email: &str,
    ) -> Result<String, UserError> {
        let email_local_part = email
            .split('@')
            .next()
            .unwrap_or("github-user")
            .trim()
            .to_ascii_lowercase();
        let base_candidate = optional_trimmed(Some(login_hint.to_ascii_lowercase()))
            .or_else(|| optional_trimmed(Some(email_local_part)))
            .unwrap_or_else(|| "github-user".into());
        if !repository.username_exists(&base_candidate, None)? {
            return Ok(base_candidate);
        }

        let normalized_suffix = normalize_code_segment(login_hint);
        let suffix = if normalized_suffix.is_empty() {
            "USER".into()
        } else {
            normalized_suffix.to_ascii_lowercase()
        };

        let mut counter = 2;
        loop {
            let candidate = format!("{base_candidate}-{suffix}-{counter}");
            if !repository.username_exists(&candidate, None)? {
                return Ok(candidate);
            }
            counter += 1;
        }
    }

    fn oauth2_auto_provision_password(&self, profile: &OAuth2Profile) -> String {
        // Keep the auto-provisioned password within the existing complexity policy
        // even when provider identifiers contain no digits, such as the local dev OAuth2 flow.
        format!(
            "Gh1!{}:{}:{}:Aa",
            profile.provider_code, profile.subject, self.auth_config.jwt_secret
        )
    }

    fn oauth_state_token(
        &self,
        provider_code: &str,
        redirect_path: &str,
    ) -> Result<String, AuthError> {
        let now = now_seconds();
        let claims = OAuthStateClaims {
            provider: provider_code.into(),
            redirect_path: redirect_path.into(),
            nonce: format!("oauth-state-{}", now_nanos()),
            exp: to_usize(now + OAUTH_STATE_TTL_SECONDS)?,
            iat: to_usize(now)?,
            kind: "oauth_state".into(),
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
        )
        .map_err(|_| AuthError::TokenEncodingFailed)
    }

    fn validate_oauth_state(
        &self,
        provider_code: &str,
        state_cookie: Option<&str>,
        state_query: Option<&str>,
    ) -> Result<OAuthStateClaims, AuthError> {
        let state_query = state_query.ok_or(AuthError::OAuth2StateInvalid)?;
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
        Ok(claims)
    }

    async fn github_profile_from_code(&self, code: &str) -> Result<OAuth2Profile, AuthError> {
        let github = self.github_oauth_config()?;
        let token_response = self
            .oauth_client
            .post(&github.token_url)
            .header("Accept", "application/json")
            .header("User-Agent", GITHUB_USER_AGENT)
            .form(&[
                ("client_id", github.client_id.as_str()),
                ("client_secret", github.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", github.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(|_| AuthError::OAuth2UpstreamFailed)?;
        if !token_response.status().is_success() {
            return Err(AuthError::OAuth2UpstreamFailed);
        }
        let token_payload = token_response
            .json::<GitHubTokenResponse>()
            .await
            .map_err(|_| AuthError::OAuth2UpstreamFailed)?;
        let access_token = match (token_payload.access_token, token_payload.error.as_deref()) {
            (Some(token), _) if !token.trim().is_empty() => token,
            (_, Some("bad_verification_code")) => return Err(AuthError::OAuth2CodeInvalid),
            (_, Some(_)) => return Err(AuthError::OAuth2ProviderError),
            _ => return Err(AuthError::OAuth2UpstreamFailed),
        };

        let user_response = self
            .oauth_client
            .get(&github.user_api_url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", GITHUB_USER_AGENT)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|_| AuthError::OAuth2UpstreamFailed)?;
        if !user_response.status().is_success() {
            return Err(AuthError::OAuth2UpstreamFailed);
        }
        let user_payload = user_response
            .json::<GitHubUserResponse>()
            .await
            .map_err(|_| AuthError::OAuth2ProfileInvalid)?;

        let emails_response = self
            .oauth_client
            .get(&github.emails_api_url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", GITHUB_USER_AGENT)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|_| AuthError::OAuth2UpstreamFailed)?;
        if !emails_response.status().is_success() {
            return Err(AuthError::OAuth2UpstreamFailed);
        }
        let email_payload = emails_response
            .json::<Vec<GitHubEmailResponse>>()
            .await
            .map_err(|_| AuthError::OAuth2ProfileInvalid)?;
        let email =
            select_github_verified_email(&email_payload).ok_or(AuthError::OAuth2ProfileInvalid)?;
        let display_name =
            optional_trimmed(user_payload.name).unwrap_or_else(|| user_payload.login.clone());

        Ok(OAuth2Profile {
            provider_code: GITHUB_PROVIDER_CODE.into(),
            provider_name: github.name.clone(),
            subject: user_payload.id.to_string(),
            login: user_payload.login.clone(),
            email,
            display_name,
        })
    }
}

fn select_github_verified_email(emails: &[GitHubEmailResponse]) -> Option<String> {
    emails
        .iter()
        .find(|email| email.primary && email.verified)
        .or_else(|| emails.iter().find(|email| email.verified))
        .map(|email| email.email.to_ascii_lowercase())
}

fn split_display_name(display_name: &str) -> (String, String) {
    let mut parts = display_name.split_whitespace();
    let first_name = parts.next().unwrap_or(OAUTH2_AUTO_PROVISION_FALLBACK_NAME);
    let last_name = parts.collect::<Vec<_>>().join(" ");
    if last_name.is_empty() {
        (first_name.to_owned(), first_name.to_owned())
    } else {
        (first_name.to_owned(), last_name)
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

fn auth_method_name(value: AuthMethod) -> &'static str {
    auth_method_as_str(value)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::outbound::{crypto::BcryptPasswordHasher, persistence::SqliteUserRepository},
        config::AppConfig,
        domain::{auth::OAuth2Profile, user::validate_initial_password},
    };

    #[test]
    fn oauth2_auto_provision_password_satisfies_password_policy_without_subject_digits() {
        let service = UserService::new(
            SqliteUserRepository::connect(":memory:")
                .expect("in-memory user repository should initialize"),
            BcryptPasswordHasher,
            AppConfig::default().auth,
        );
        let profile = OAuth2Profile {
            provider_code: "dev".into(),
            provider_name: "OAuth2".into(),
            subject: "dev-auth-operator".into(),
            login: "auth.operator".into(),
            email: "auth.operator@tron.ai".into(),
            display_name: "Auth Operator".into(),
        };

        let password = service.oauth2_auto_provision_password(&profile);
        assert!(
            validate_initial_password(&password).is_ok(),
            "generated OAuth2 bootstrap password should satisfy the initial password policy",
        );
    }

    #[test]
    fn split_display_name_avoids_provider_name_fallback_for_single_token_profiles() {
        let (first_name, last_name) = split_display_name("CloudS3n");

        assert_eq!(first_name, "CloudS3n");
        assert_eq!(last_name, "CloudS3n");
    }
}
