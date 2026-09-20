pub(crate) mod error_mapper;
pub(crate) mod handler;
mod request;
mod response;
mod routes;

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    application::profile::{CurrentProfileContext, ProfileRegistryError, ProfileRegistryService},
    application::user::UserService,
    domain::auth::{
        AccessTokenIdentity, AuthError, AuthMethod, AuthProviderDescriptor, AuthProviderKind,
        AuthenticatedUser, CurrentSession,
    },
    shared::{
        auth_holder::{AuthHolder, AuthHolderContext},
        locale::resolve_request_locale,
    },
};

pub use routes::routes;

use self::error_mapper::auth_error_response;

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
            role_codes: Vec::new(),
            space_codes: Vec::new(),
            status: crate::domain::user::UserStatus::Active,
            password_reset_required: false,
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
                role_codes: Vec::new(),
                space_codes: Vec::new(),
                auth_method: AuthMethod::Internal,
                provider_code: Some("person-ai".into()),
            },
        }
    }
}

impl RequestAuthContext {
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
            .ok_or_else(|| auth_error_response(AuthError::Unauthorized))
    }
}

pub(crate) fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|header| header.to_str().ok())
        .and_then(|raw| {
            raw.split(';').find_map(|pair| {
                let trimmed = pair.trim();
                let prefix = format!("{name}=");
                trimmed
                    .strip_prefix(&prefix)
                    .map(|value| value.trim().to_owned())
            })
        })
}

pub(crate) fn read_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|raw| {
            let trimmed = raw.trim();
            let (scheme, token) = trimmed.split_once(' ')?;
            if !scheme.eq_ignore_ascii_case("Bearer") {
                return None;
            }

            let token = token.trim();
            if token.is_empty() {
                None
            } else {
                Some(token.to_owned())
            }
        })
}

pub(crate) fn read_access_token(headers: &HeaderMap) -> Option<String> {
    read_bearer_token(headers)
}

pub(crate) fn authenticate_headers(
    users: &UserService,
    headers: &HeaderMap,
) -> Result<RequestAuthContext, AuthError> {
    let access_token = read_access_token(headers);
    let authenticated = users.authenticate_access_token(access_token.as_deref())?;
    let locale = resolve_request_locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok()),
    );

    Ok(RequestAuthContext {
        access_token: access_token.expect("authenticated requests must include an access token"),
        locale,
        session: authenticated.session,
        token_identity: authenticated.token_identity,
    })
}

pub async fn require_auth(
    State(users): State<UserService>,
    mut request: Request,
    next: Next,
) -> Response {
    match authenticate_headers(&users, request.headers()) {
        Ok(auth) => {
            let holder_context = auth.holder_context();
            request.extensions_mut().insert(auth);
            AuthHolder::scope(holder_context, next.run(request)).await
        }
        Err(error) => auth_error_response(error),
    }
}

pub async fn require_profile(
    State(profiles): State<ProfileRegistryService>,
    mut request: Request,
    next: Next,
) -> Response {
    let context = match profiles.current_context() {
        Ok(context) => context,
        Err(ProfileRegistryError::ProfileNotFound(_)) => {
            return profile_required_response();
        }
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
        axum::Json(crate::response::ApiResponse::<crate::response::EmptyData>::error(
            "PERSONAI_PROFILE_REQUIRED",
            "select a PersonAI profile before using this resource",
        )),
    )
        .into_response()
}

fn profile_registry_error_response() -> Response {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(crate::response::ApiResponse::<crate::response::EmptyData>::error(
            "PERSONAI_PROFILE_REGISTRY_FAILED",
            "the active PersonAI profile could not be resolved",
        )),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use axum::{
        Json, Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
        middleware,
        response::IntoResponse,
        routing::get,
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use crate::{app::AppState, config::AppConfig, shared::auth_holder::AuthHolder};

    use super::{RequestAuthContext, require_auth};

    #[tokio::test]
    async fn require_auth_populates_auth_holder_from_valid_access_token() {
        let (config, database_path) = test_config("auth-holder");
        let state = AppState::new(config).expect("app state should build");
        let authorized = state
            .users
            .login_internal("admin", "TronAdminPass123!")
            .expect("bootstrap admin login should succeed");
        let router = Router::new()
            .route("/probe", get(probe_auth_holder))
            .route("/probe/spawn", get(probe_auth_holder_spawn))
            .route_layer(middleware::from_fn_with_state(
                state.users.clone(),
                require_auth,
            ))
            .with_state(state);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/probe")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", authorized.access_token),
                    )
                    .header(header::ACCEPT_LANGUAGE, "zh-CN")
                    .body(Body::empty())
                    .expect("probe request should build"),
            )
            .await
            .expect("probe request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload = response_json(response).await;
        assert_eq!(payload["userCode"], "USR-ADMIN");
        assert_eq!(payload["holderUserCode"], "USR-ADMIN");
        assert_eq!(payload["locale"], "zh-CN");
        assert_eq!(
            payload["holderSessionCode"].as_str(),
            payload["tokenSessionCode"].as_str()
        );
        assert!(
            payload["roleCodes"]
                .as_array()
                .expect("role codes should be an array")
                .iter()
                .any(|value| value == "ADMIN"),
            "token role codes should include ADMIN"
        );

        let _ = fs::remove_file(database_path);
    }

    #[tokio::test]
    async fn require_auth_propagates_auth_holder_into_spawned_tasks() {
        let (config, database_path) = test_config("auth-holder-spawn");
        let state = AppState::new(config).expect("app state should build");
        let authorized = state
            .users
            .login_internal("admin", "TronAdminPass123!")
            .expect("bootstrap admin login should succeed");
        let router = Router::new()
            .route("/probe/spawn", get(probe_auth_holder_spawn))
            .route_layer(middleware::from_fn_with_state(
                state.users.clone(),
                require_auth,
            ))
            .with_state(state);

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/probe/spawn")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", authorized.access_token),
                    )
                    .body(Body::empty())
                    .expect("spawn probe request should build"),
            )
            .await
            .expect("spawn probe request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let payload = response_json(response).await;
        assert_eq!(payload["userCode"], "USR-ADMIN");
        assert_eq!(payload["holderUserCode"], "USR-ADMIN");
        assert_eq!(payload["roleCodes"][0], "ADMIN");

        let _ = fs::remove_file(database_path);
    }

    async fn probe_auth_holder(auth: RequestAuthContext) -> impl IntoResponse {
        tokio::task::yield_now().await;
        let holder = AuthHolder::get().expect("auth holder should be populated");
        Json(json!({
            "userCode": auth.user().user_code.clone(),
            "holderUserCode": holder.user_code(),
            "holderSessionCode": holder.session_code(),
            "tokenSessionCode": auth.token_identity.session_code.clone(),
            "locale": holder.locale.clone(),
            "roleCodes": holder.role_codes(),
        }))
    }

    async fn probe_auth_holder_spawn(auth: RequestAuthContext) -> impl IntoResponse {
        let payload = AuthHolder::spawn(async move {
            let holder = AuthHolder::get().expect("auth holder should be populated in spawn");
            json!({
                "userCode": auth.user().user_code.clone(),
                "holderUserCode": holder.user_code(),
                "holderSessionCode": holder.session_code(),
                "roleCodes": holder.role_codes(),
            })
        })
        .await
        .expect("spawned auth probe should complete");

        Json(payload)
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&bytes).expect("response body should decode as JSON")
    }

    fn test_config(prefix: &str) -> (AppConfig, PathBuf) {
        let database_path = temp_database_path(prefix);
        let mut config = AppConfig::default();
        config.database_url = database_path.display().to_string();
        (config, database_path)
    }

    fn temp_database_path(prefix: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current system time should be after unix epoch")
            .as_millis();
        std::env::temp_dir().join(format!("{prefix}-{}-{millis}.sqlite3", std::process::id()))
    }
}
