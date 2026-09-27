//! Profile-scoped request context.
//!
//! Browser requests are scoped by the selected PersonAI profile.  This keeps
//! the legacy handler context shape, but its identity is always derived from
//! the active profile; there is no local login or token validation here.

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    application::profile::{CurrentProfileContext, ProfileRegistryError, ProfileRegistryService},
    domain::auth::{
        AccessTokenIdentity, AuthMethod, AuthProviderDescriptor, AuthProviderKind,
        AuthenticatedUser, CurrentSession,
    },
    shared::{
        auth_holder::{AuthHolder, AuthHolderContext},
        locale::resolve_request_locale,
    },
};

#[derive(Debug, Clone)]
pub struct RequestAuthContext {
    pub access_token: String,
    pub locale: String,
    pub session: CurrentSession,
    pub token_identity: AccessTokenIdentity,
}

impl RequestAuthContext {
    pub fn from_profile(context: &CurrentProfileContext, locale: String) -> Self {
        let user = AuthenticatedUser {
            avatar_url: None,
            user_code: context.person_ai_user_id.clone(),
            username: context.person_ai_username.clone(),
            display_name: context.person_ai_username.clone(),
            email: String::new(),
        };
        Self {
            access_token: String::new(),
            locale,
            session: CurrentSession {
                authenticated: true,
                auth_method: AuthMethod::Internal,
                provider: Some(AuthProviderDescriptor {
                    code: "person-ai".into(),
                    name: "PersonAI".into(),
                    kind: AuthProviderKind::Internal,
                }),
                user,
            },
            token_identity: AccessTokenIdentity {
                user_code: context.person_ai_user_id.clone(),
                session_code: format!("PERSONAI-PROFILE-{}", context.profile_id),
                auth_method: AuthMethod::Internal,
                provider_code: Some("person-ai".into()),
            },
        }
    }

    pub fn user(&self) -> &AuthenticatedUser {
        &self.session.user
    }

    pub fn holder_context(&self) -> AuthHolderContext {
        AuthHolderContext {
            access_token: self.access_token.clone(),
            locale: self.locale.clone(),
            session: self.session.clone(),
            token_identity: self.token_identity.clone(),
        }
    }
}

impl<S> FromRequestParts<S> for RequestAuthContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or_else(profile_required_response)
    }
}

pub async fn require_profile(
    State(profiles): State<ProfileRegistryService>,
    mut request: Request,
    next: Next,
) -> Response {
    let context = match profiles.current_context() {
        Ok(context) => context,
        Err(ProfileRegistryError::ProfileNotFound(_)) => return profile_required_response(),
        Err(_) => return profile_registry_error_response(),
    };
    let locale = resolve_request_locale(
        request
            .headers()
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    );
    let auth = RequestAuthContext::from_profile(&context, locale);
    request.extensions_mut().insert(context);
    request.extensions_mut().insert(auth.clone());
    AuthHolder::scope(auth.holder_context(), next.run(request)).await
}

fn profile_required_response() -> Response {
    (
        axum::http::StatusCode::CONFLICT,
        axum::Json(
            crate::response::ApiResponse::<crate::response::EmptyData>::error(
                "PERSONAI_PROFILE_REQUIRED",
                "select a PersonAI profile before using this resource",
            ),
        ),
    )
        .into_response()
}

fn profile_registry_error_response() -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(
            crate::response::ApiResponse::<crate::response::EmptyData>::error(
                "PERSONAI_PROFILE_REGISTRY_FAILED",
                "the active PersonAI profile could not be resolved",
            ),
        ),
    )
        .into_response()
}
