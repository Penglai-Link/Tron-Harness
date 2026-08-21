use super::*;

impl UserService {
    pub(super) fn dev_oauth2_authorize(
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

    pub(super) fn github_oauth2_authorize(
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

    pub(super) fn dev_oauth2_callback(
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

        let profile = AuthIdentityProfile {
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
        let session = self.create_external_auth_session(
            repository.as_mut(),
            &profile,
            AuthMethod::Oauth2,
            self.dev_provider_descriptor(),
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

    pub(super) async fn github_oauth2_callback(
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
        let session = self.create_external_auth_session(
            repository.as_mut(),
            &profile,
            AuthMethod::Oauth2,
            self.github_provider_descriptor(),
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

    pub(super) fn dev_provider_descriptor(&self) -> AuthProviderDescriptor {
        AuthProviderDescriptor {
            code: "dev".into(),
            name: self.auth_config.dev_oauth2_name.clone(),
            kind: AuthProviderKind::Oauth2,
        }
    }

    pub(super) fn github_provider_descriptor_if_enabled(&self) -> Option<AuthProviderDescriptor> {
        self.auth_config
            .github_oauth
            .as_ref()
            .map(|config| AuthProviderDescriptor {
                code: GITHUB_PROVIDER_CODE.into(),
                name: config.name.clone(),
                kind: AuthProviderKind::Oauth2,
            })
    }

    pub(super) fn ldap_provider_descriptor_if_enabled(&self) -> Option<AuthProviderDescriptor> {
        self.auth_config
            .ldap
            .as_ref()
            .map(|config| AuthProviderDescriptor {
                code: LDAP_PROVIDER_CODE.into(),
                name: config.name.clone(),
                kind: AuthProviderKind::Ldap,
            })
    }

    pub(super) fn ldap_provider_descriptor(&self) -> AuthProviderDescriptor {
        self.ldap_provider_descriptor_if_enabled()
            .unwrap_or(AuthProviderDescriptor {
                code: LDAP_PROVIDER_CODE.into(),
                name: "LDAP".into(),
                kind: AuthProviderKind::Ldap,
            })
    }

    pub(super) fn github_provider_descriptor(&self) -> AuthProviderDescriptor {
        self.github_provider_descriptor_if_enabled()
            .unwrap_or(AuthProviderDescriptor {
                code: GITHUB_PROVIDER_CODE.into(),
                name: "GitHub".into(),
                kind: AuthProviderKind::Oauth2,
            })
    }

    pub(super) fn session_provider_descriptor(
        &self,
        provider_code: &str,
    ) -> Option<AuthProviderDescriptor> {
        match provider_code {
            GITHUB_PROVIDER_CODE => Some(self.github_provider_descriptor()),
            LDAP_PROVIDER_CODE => Some(self.ldap_provider_descriptor()),
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

    pub(super) fn ldap_config(&self) -> Result<&LdapConfig, AuthError> {
        self.auth_config
            .ldap
            .as_ref()
            .ok_or(AuthError::ProviderNotFound)
    }

    pub(super) fn create_external_auth_session(
        &self,
        repository: &mut dyn UserRepository,
        profile: &AuthIdentityProfile,
        auth_method: AuthMethod,
        provider: AuthProviderDescriptor,
        redirect_path: Option<String>,
    ) -> Result<AuthorizedSession, AuthError> {
        let user = match repository
            .find_user_by_provider_subject(&profile.provider_code, &profile.subject)?
        {
            Some(user) => user,
            None => match repository.find_active_user_by_email(&profile.email)? {
                Some(user) => user,
                None => self.auto_provision_external_user(repository, profile)?,
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

        self.create_session(
            repository,
            &user,
            auth_method,
            Some(provider),
            redirect_path,
        )
    }

    fn auto_provision_external_user(
        &self,
        repository: &mut dyn UserRepository,
        profile: &AuthIdentityProfile,
    ) -> Result<User, AuthError> {
        let (first_name, last_name) = split_display_name(&profile.display_name);
        let username = self
            .next_available_external_username(repository, &profile.login, &profile.email)
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
            locale: Some(EXTERNAL_AUTH_AUTO_PROVISION_DEFAULT_LOCALE.into()),
            timezone: Some(EXTERNAL_AUTH_AUTO_PROVISION_DEFAULT_TIMEZONE.into()),
            initial_password: Some(self.external_auth_auto_provision_password(profile)),
            password_reset_required: Some(true),
            role_codes: vec![EXTERNAL_AUTH_AUTO_PROVISION_DEFAULT_ROLE_CODE.into()],
            space_codes: vec![EXTERNAL_AUTH_AUTO_PROVISION_DEFAULT_SPACE_CODE.into()],
        };
        self.create_user_in_repository(repository, command)
            .map_err(map_user_error_to_auth)
    }

    fn next_available_external_username(
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

    fn external_auth_auto_provision_password(&self, profile: &AuthIdentityProfile) -> String {
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

    async fn github_profile_from_code(&self, code: &str) -> Result<AuthIdentityProfile, AuthError> {
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

        Ok(AuthIdentityProfile {
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
    let first_name = parts
        .next()
        .unwrap_or(EXTERNAL_AUTH_AUTO_PROVISION_FALLBACK_NAME);
    let last_name = parts.collect::<Vec<_>>().join(" ");
    if last_name.is_empty() {
        (first_name.to_owned(), first_name.to_owned())
    } else {
        (first_name.to_owned(), last_name)
    }
}

fn normalize_redirect_path(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some(path) if path.starts_with('/') && !path.starts_with("//") => path.to_owned(),
        _ => "/apps".into(),
    }
}

fn oauth_state_validation() -> Validation {
    token_validation()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::outbound::{crypto::BcryptPasswordHasher, persistence::SqliteUserRepository},
        config::AppConfig,
        domain::{auth::AuthIdentityProfile, user::validate_initial_password},
    };

    #[test]
    fn oauth2_auto_provision_password_satisfies_password_policy_without_subject_digits() {
        let service = UserService::new(
            SqliteUserRepository::connect(":memory:")
                .expect("in-memory user repository should initialize"),
            BcryptPasswordHasher,
            AppConfig::default().auth,
        );
        let profile = AuthIdentityProfile {
            provider_code: "dev".into(),
            provider_name: "OAuth2".into(),
            subject: "dev-auth-operator".into(),
            login: "auth.operator".into(),
            email: "auth.operator@tron.ai".into(),
            display_name: "Auth Operator".into(),
        };

        let password = service.external_auth_auto_provision_password(&profile);
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
