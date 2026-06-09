use super::*;

impl UserService {
    pub(super) fn create_session(
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

    pub(super) fn session_from_access_token(
        &self,
        access_token: &str,
    ) -> Result<AuthenticatedAccess, AuthError> {
        let claims = decode_access_token(access_token, &self.auth_config.jwt_secret)?;
        let mut repository = self
            .repository
            .lock()
            .expect("user repository mutex poisoned");
        let user = repository
            .get_user(claims.resolved_user_code())
            .map_err(map_user_error_to_auth)?;
        ensure_active_user(&user)?;
        let auth_method = auth_method_from_claims(&claims.auth_method)?;
        let session = CurrentSession {
            authenticated: true,
            auth_method,
            provider: claims
                .provider_code
                .as_deref()
                .and_then(|value| self.session_provider_descriptor(value)),
            user: AuthenticatedUser::from(&user),
        };

        Ok(AuthenticatedAccess {
            session,
            token_identity: AccessTokenIdentity {
                user_code: claims.resolved_user_code().to_string(),
                session_code: claims.resolved_session_code().to_string(),
                role_codes: if claims.role_codes.is_empty() {
                    user.role_codes.clone()
                } else {
                    claims.role_codes.clone()
                },
                space_codes: if claims.space_codes.is_empty() {
                    user.space_codes.clone()
                } else {
                    claims.space_codes.clone()
                },
                auth_method,
                provider_code: claims.provider_code.clone(),
            },
        })
    }

    pub(super) fn access_token_for_session(
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
            user_code: Some(user.user_code.clone()),
            session_code: Some(session_code.into()),
            auth_method: auth_method_as_str(auth_method).into(),
            provider_code,
            role_codes: user.role_codes.clone(),
            space_codes: user.space_codes.clone(),
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

    pub(super) fn refresh_token_for_session(
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
            user_code: Some(user.user_code.clone()),
            session_code: Some(session_code.into()),
            auth_method: auth_method_as_str(auth_method).into(),
            provider_code,
            role_codes: user.role_codes.clone(),
            space_codes: user.space_codes.clone(),
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

pub(super) fn decode_refresh_token(
    token: &str,
    secret: &str,
) -> Result<AccessTokenClaims, AuthError> {
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

pub(super) fn token_validation() -> Validation {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation
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

pub(super) fn auth_method_name(value: AuthMethod) -> &'static str {
    auth_method_as_str(value)
}

pub(super) fn ensure_active_user(user: &User) -> Result<(), AuthError> {
    if user.status == UserStatus::Active {
        Ok(())
    } else {
        Err(AuthError::UserInactive)
    }
}

pub(super) fn map_user_error_to_auth(error: UserError) -> AuthError {
    match error {
        UserError::NotFound => AuthError::Unauthorized,
        _ => AuthError::StorageFailure,
    }
}

pub(super) fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_secs() as i64
}

pub(super) fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos()
}

pub(super) fn to_usize(value: i64) -> Result<usize, AuthError> {
    usize::try_from(value).map_err(|_| AuthError::TokenEncodingFailed)
}
