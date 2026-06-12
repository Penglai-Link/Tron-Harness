use std::collections::BTreeSet;

use crate::{
    domain::{
        auth::AuthError,
        open_app::{
            OpenAppAuthState, OpenAppCredential, OpenAppCredentialStatus, OpenAppError,
            OpenAppStatus, open_app_secret_fingerprint,
        },
        user::{UserError, UserStatus},
    },
    shared::{crypto::Aes256SecretBox, id::SnowflakeId},
};

use super::{
    CapabilityGrantInput, ManagedOpenAppCapabilityGrant, ManagedOpenAppDetail,
    ManagedOpenAppSummary, ManagedOpenAppUserGrant, OpenAppService,
};

pub(super) fn map_summary_from_auth_state(auth_state: &OpenAppAuthState) -> ManagedOpenAppSummary {
    ManagedOpenAppSummary {
        open_app_code: auth_state.open_app.open_app_code.clone(),
        name: auth_state.open_app.name.clone(),
        description: auth_state.open_app.description.clone(),
        status: auth_state.open_app.status,
        app_key_fingerprint: auth_state
            .active_credential
            .as_ref()
            .map(|credential| credential.secret_fingerprint.clone())
            .unwrap_or_default(),
        created_at: auth_state.open_app.created_at.clone(),
        updated_at: auth_state.open_app.updated_at.clone(),
        last_rotated_at: auth_state.open_app.last_rotated_at.clone(),
        user_grant_count: auth_state.user_grants.len(),
        capability_grant_count: auth_state.capability_grants.len(),
    }
}

pub(super) fn map_detail_from_auth_state(auth_state: &OpenAppAuthState) -> ManagedOpenAppDetail {
    ManagedOpenAppDetail {
        open_app_code: auth_state.open_app.open_app_code.clone(),
        name: auth_state.open_app.name.clone(),
        description: auth_state.open_app.description.clone(),
        status: auth_state.open_app.status,
        app_key_fingerprint: auth_state
            .active_credential
            .as_ref()
            .map(|credential| credential.secret_fingerprint.clone())
            .unwrap_or_default(),
        created_at: auth_state.open_app.created_at.clone(),
        updated_at: auth_state.open_app.updated_at.clone(),
        last_rotated_at: auth_state.open_app.last_rotated_at.clone(),
        user_grants: auth_state
            .user_grants
            .iter()
            .map(|grant| ManagedOpenAppUserGrant {
                user_code: grant.user_code.clone(),
            })
            .collect(),
        capability_grants: auth_state
            .capability_grants
            .iter()
            .map(|grant| ManagedOpenAppCapabilityGrant {
                capability_code: grant.capability_code.clone(),
                operation_code: grant.operation_code.clone(),
            })
            .collect(),
    }
}

pub(super) fn ensure_open_app_active(auth_state: &OpenAppAuthState) -> Result<(), OpenAppError> {
    if auth_state.open_app.status != OpenAppStatus::Enabled {
        return Err(OpenAppError::OpenAppDisabled);
    }
    if auth_state
        .active_credential
        .as_ref()
        .is_some_and(|credential| credential.status != OpenAppCredentialStatus::Active)
    {
        return Err(OpenAppError::OpenAppDisabled);
    }
    Ok(())
}

pub(super) fn build_active_credential(
    next_id: &mut i64,
    secret_box: &Aes256SecretBox,
    app_key: &str,
    now: &str,
    open_app_code: &str,
) -> Result<OpenAppCredential, OpenAppError> {
    let credential = OpenAppCredential {
        id: SnowflakeId::new(*next_id),
        credential_code: format!("OPENAPP-CRED-{}", *next_id),
        open_app_code: open_app_code.to_owned(),
        status: OpenAppCredentialStatus::Active,
        secret_ciphertext: secret_box
            .encrypt(app_key)
            .map_err(|_| OpenAppError::SecretCryptoFailure)?,
        secret_fingerprint: open_app_secret_fingerprint(app_key),
        created_at: now.to_owned(),
        revoked_at: None,
    };
    *next_id += 1;
    Ok(credential)
}

pub(super) fn normalize_user_codes(user_codes: Vec<String>) -> Result<Vec<String>, OpenAppError> {
    let mut normalized = BTreeSet::new();
    for user_code in user_codes {
        normalized.insert(required_trimmed(user_code, OpenAppError::InvalidUserCode)?);
    }
    Ok(normalized.into_iter().collect())
}

pub(super) fn normalize_capability_grants(
    grants: Vec<CapabilityGrantInput>,
) -> Result<Vec<CapabilityGrantInput>, OpenAppError> {
    let mut normalized = BTreeSet::new();
    for grant in grants {
        let capability_code =
            required_trimmed(grant.capability_code, OpenAppError::InvalidCapability)?;
        let operation_code =
            required_trimmed(grant.operation_code, OpenAppError::InvalidOperation)?;
        normalized.insert((capability_code, operation_code));
    }

    Ok(normalized
        .into_iter()
        .map(|(capability_code, operation_code)| CapabilityGrantInput {
            capability_code,
            operation_code,
        })
        .collect())
}

pub(super) fn required_trimmed(value: String, error: OpenAppError) -> Result<String, OpenAppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(error)
    } else {
        Ok(trimmed.to_owned())
    }
}

pub(super) fn optional_trimmed(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

pub(super) fn map_user_error(error: UserError) -> OpenAppError {
    match error {
        UserError::NotFound | UserError::InvalidUserCode | UserError::InvalidStatus => {
            OpenAppError::InvalidUserCode
        }
        UserError::StorageFailure => OpenAppError::StorageFailure,
        UserError::DuplicateUserCode
        | UserError::DuplicateUsername
        | UserError::DuplicateEmail
        | UserError::RequiredProfileField
        | UserError::WeakInitialPassword
        | UserError::InvalidRoleCode
        | UserError::InvalidSpaceCode
        | UserError::PasswordHashFailed => OpenAppError::StorageFailure,
    }
}

pub(super) fn map_auth_error(error: AuthError) -> OpenAppError {
    match error {
        AuthError::Unauthorized
        | AuthError::InvalidCredentials
        | AuthError::UserInactive
        | AuthError::UserNotLinked => OpenAppError::UserForbidden,
        AuthError::StorageFailure
        | AuthError::TokenDecodingFailed
        | AuthError::TokenEncodingFailed
        | AuthError::PasswordVerifyFailed => OpenAppError::StorageFailure,
        AuthError::RefreshInvalid
        | AuthError::ProviderNotFound
        | AuthError::OAuth2StateInvalid
        | AuthError::OAuth2CodeInvalid
        | AuthError::OAuth2ProviderError
        | AuthError::OAuth2ProfileInvalid
        | AuthError::OAuth2UpstreamFailed
        | AuthError::LdapProfileInvalid
        | AuthError::LdapUpstreamFailed
        | AuthError::StepUpFailed
        | AuthError::StepUpExpired
        | AuthError::StepUpUnsupported
        | AuthError::StepUpTokenMismatch
        | AuthError::StepUpScopeInvalid => OpenAppError::StorageFailure,
    }
}

impl OpenAppService {
    pub(super) fn ensure_active_user(
        &self,
        user_code: &str,
    ) -> Result<crate::domain::auth::AuthenticatedAccess, OpenAppError> {
        self.users
            .authenticate_open_api_user(user_code)
            .map_err(map_auth_error)
    }

    pub(super) fn ensure_grantable_user(&self, user_code: &str) -> Result<(), OpenAppError> {
        let user_code = required_trimmed(user_code.to_owned(), OpenAppError::InvalidUserCode)?;
        let user = self.users.get_user(&user_code).map_err(map_user_error)?;
        if user.status != UserStatus::Active {
            return Err(OpenAppError::InvalidUserCode);
        }
        Ok(())
    }
}
