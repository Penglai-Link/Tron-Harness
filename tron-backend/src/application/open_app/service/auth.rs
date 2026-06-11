use crate::{
    domain::open_app::{
        OPEN_API_SIGNATURE_VERSION, OpenApiCanonicalRequest, OpenApiNonceRecord, OpenApiRouteGrant,
        OpenAppError, build_open_api_canonical_request, verify_open_api_signature,
    },
    shared::{
        id::SnowflakeId,
        random::random_hex,
        time::{current_timestamp, current_timestamp_seconds},
    },
};

use super::{
    AuthenticatedOpenAppRequest, OpenAppService, SignedOpenApiRequest,
    shared::{ensure_open_app_active, required_trimmed},
};

impl OpenAppService {
    pub fn authenticate_signed_request(
        &self,
        request: SignedOpenApiRequest,
        route_grant: OpenApiRouteGrant,
    ) -> Result<AuthenticatedOpenAppRequest, OpenAppError> {
        let open_app_code = required_trimmed(request.open_app_code, OpenAppError::Unauthorized)?;
        let user_code = required_trimmed(request.user_code, OpenAppError::Unauthorized)?;
        let nonce = required_trimmed(request.nonce, OpenAppError::Unauthorized)?;
        let signature = required_trimmed(request.signature, OpenAppError::Unauthorized)?;
        if request.signature_version.trim() != OPEN_API_SIGNATURE_VERSION {
            return Err(OpenAppError::InvalidSignatureVersion);
        }

        let now = current_timestamp_seconds();
        if (request.timestamp - now).abs() > self.replay_window_seconds {
            return Err(OpenAppError::InvalidTimestamp);
        }

        let mut repository = self
            .repository
            .lock()
            .expect("open-app repository mutex poisoned");
        repository.purge_expired_nonces(now)?;
        let Some(auth_state) = repository.load_auth_state(&open_app_code)? else {
            return Err(OpenAppError::Unauthorized);
        };
        ensure_open_app_active(&auth_state)?;
        let credential = auth_state
            .active_credential
            .as_ref()
            .ok_or(OpenAppError::Unauthorized)?;
        let secret = self
            .secret_box
            .decrypt(&credential.secret_ciphertext)
            .map_err(|_| OpenAppError::SecretCryptoFailure)?;
        let canonical = build_open_api_canonical_request(&OpenApiCanonicalRequest {
            method: &request.method,
            path: &request.path,
            raw_query: request.raw_query.as_deref(),
            body: &request.body,
            open_app_code: &open_app_code,
            user_code: &user_code,
            timestamp: request.timestamp,
            nonce: &nonce,
        })?;
        let signature_valid = verify_open_api_signature(&secret, &canonical, &signature)?;
        if !signature_valid {
            return Err(OpenAppError::InvalidSignature);
        }

        let nonce_record = OpenApiNonceRecord {
            id: SnowflakeId::new(repository.next_id()?),
            nonce_code: format!("OPENAPP-NONCE-{}", random_hex(10)),
            open_app_code: open_app_code.clone(),
            nonce,
            signed_at: request.timestamp,
            expires_at: request.timestamp + self.replay_window_seconds,
            created_at: current_timestamp(),
        };
        repository.insert_nonce(&nonce_record)?;
        drop(repository);

        let authenticated = self.ensure_active_user(&user_code)?;
        let resolved_user_code = authenticated.token_identity.user_code.clone();
        let user_granted = auth_state
            .user_grants
            .iter()
            .any(|grant| grant.user_code == resolved_user_code);
        if !user_granted {
            return Err(OpenAppError::UserForbidden);
        }
        let operation_granted = auth_state.capability_grants.iter().any(|grant| {
            grant.capability_code == route_grant.capability_code
                && grant.operation_code == route_grant.operation_code
        });
        if !operation_granted {
            return Err(OpenAppError::OperationForbidden);
        }

        Ok(AuthenticatedOpenAppRequest {
            open_app: auth_state.open_app,
            authenticated,
            route_grant,
        })
    }

    pub fn sign_request_for_tests(
        &self,
        secret: &str,
        request: &SignedOpenApiRequest,
    ) -> Result<String, OpenAppError> {
        let canonical = build_open_api_canonical_request(&OpenApiCanonicalRequest {
            method: &request.method,
            path: &request.path,
            raw_query: request.raw_query.as_deref(),
            body: &request.body,
            open_app_code: &request.open_app_code,
            user_code: &request.user_code,
            timestamp: request.timestamp,
            nonce: &request.nonce,
        })?;
        crate::domain::open_app::compute_open_api_signature(secret, &canonical)
    }
}
