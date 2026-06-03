use super::*;

impl UserService {
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
        if let Some(provider) = self.ldap_provider_descriptor_if_enabled() {
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

    pub async fn login_ldap(
        &self,
        login: &str,
        password: &str,
    ) -> Result<AuthorizedSession, AuthError> {
        let login = login.trim();
        if login.is_empty() || password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }

        let ldap_config = self.ldap_config()?;
        let profile = self
            .ldap_authenticator
            .authenticate(ldap_config, login, password)
            .await?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let session = self.create_external_auth_session(
            repository.as_mut(),
            &profile,
            AuthMethod::Ldap,
            self.ldap_provider_descriptor(),
            None,
        )?;
        drop(repository);
        self.publish_auth_session_event(
            "auth.session.created",
            "Created LDAP session",
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
                session: session.session,
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
    ) -> Result<AuthenticatedAccess, AuthError> {
        let access_token = access_token.ok_or(AuthError::Unauthorized)?;
        self.session_from_access_token(access_token)
    }

    pub fn authenticate_open_api_user(
        &self,
        user_identifier: &str,
    ) -> Result<AuthenticatedAccess, AuthError> {
        let user_code = user_identifier.trim();
        if user_code.is_empty() {
            return Err(AuthError::Unauthorized);
        }

        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = repository
            .get_user(user_code)
            .map_err(map_user_error_to_auth)?;
        ensure_active_user(&user)?;

        Ok(AuthenticatedAccess {
            session: CurrentSession {
                authenticated: true,
                auth_method: AuthMethod::Internal,
                provider: None,
                user: AuthenticatedUser::from(&user),
            },
            token_identity: AccessTokenIdentity {
                user_code: user.user_code.clone(),
                session_code: format!("OPENAPI-{}", user.user_code),
                role_codes: user.role_codes.clone(),
                space_codes: user.space_codes.clone(),
                auth_method: AuthMethod::Internal,
                provider_code: None,
            },
        })
    }

    pub fn verify_step_up_password(
        &self,
        user_code: &str,
        password: &str,
        target: &StepUpVariableRevealTarget,
    ) -> Result<crate::application::user::StepUpToken, AuthError> {
        let target = normalized_step_up_target(target)?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = repository
            .get_user(user_code)
            .map_err(map_user_error_to_auth)?;
        ensure_active_user(&user)?;
        if user.password_hash.trim().is_empty() {
            return Err(AuthError::StepUpUnsupported);
        }
        if !self
            .password_hasher
            .verify_password(password, &user.password_hash)?
        {
            return Err(AuthError::StepUpFailed);
        }

        let now = now_seconds();
        let expires_at_seconds = now + 300;
        let claims = StepUpClaims {
            sub: user.user_code,
            purpose: "variable-reveal".into(),
            scope: step_up_scope_claim(target.scope).into(),
            workspace_code: target.workspace_code,
            variable_key: target.variable_key,
            exp: to_usize(expires_at_seconds)?,
            iat: to_usize(now)?,
            kind: "variable-step-up".into(),
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
        )
        .map_err(|_| AuthError::TokenEncodingFailed)?;
        let expires_at = chrono::DateTime::from_timestamp(expires_at_seconds, 0)
            .ok_or(AuthError::TokenEncodingFailed)?
            .to_rfc3339();
        Ok(crate::application::user::StepUpToken { token, expires_at })
    }

    pub fn verify_step_up_token(
        &self,
        user_code: &str,
        step_up_token: &str,
        target: &StepUpVariableRevealTarget,
    ) -> Result<(), AuthError> {
        let target = normalized_step_up_target(target)?;
        let claims = decode::<StepUpClaims>(
            step_up_token,
            &DecodingKey::from_secret(self.auth_config.jwt_secret.as_bytes()),
            &token_validation(),
        )
        .map_err(|error| match error.kind() {
            JwtErrorKind::ExpiredSignature => AuthError::StepUpExpired,
            _ => AuthError::StepUpTokenMismatch,
        })?
        .claims;
        if claims.kind != "variable-step-up" {
            return Err(AuthError::StepUpTokenMismatch);
        }
        if claims.purpose != "variable-reveal" {
            return Err(AuthError::StepUpTokenMismatch);
        }
        if claims.sub != user_code {
            return Err(AuthError::StepUpTokenMismatch);
        }
        if claims.scope != step_up_scope_claim(target.scope) {
            return Err(AuthError::StepUpTokenMismatch);
        }
        if claims.workspace_code != target.workspace_code {
            return Err(AuthError::StepUpTokenMismatch);
        }
        if claims.variable_key != target.variable_key {
            return Err(AuthError::StepUpTokenMismatch);
        }
        Ok(())
    }
}

fn normalized_step_up_target(
    target: &StepUpVariableRevealTarget,
) -> Result<StepUpVariableRevealTarget, AuthError> {
    let variable_key = target.variable_key.trim().to_string();
    if variable_key.is_empty() {
        return Err(AuthError::StepUpScopeInvalid);
    }

    match target.scope {
        StepUpVariableRevealScope::Platform => Ok(StepUpVariableRevealTarget {
            scope: StepUpVariableRevealScope::Platform,
            workspace_code: None,
            variable_key,
        }),
        StepUpVariableRevealScope::Workspace => {
            let workspace_code = target
                .workspace_code
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or(AuthError::StepUpScopeInvalid)?
                .to_string();
            Ok(StepUpVariableRevealTarget {
                scope: StepUpVariableRevealScope::Workspace,
                workspace_code: Some(workspace_code),
                variable_key,
            })
        }
    }
}

fn step_up_scope_claim(scope: StepUpVariableRevealScope) -> &'static str {
    match scope {
        StepUpVariableRevealScope::Platform => "platform-variable",
        StepUpVariableRevealScope::Workspace => "workspace-variable",
    }
}
