mod error_mapper;
mod handler;
mod request;
mod response;
mod routes;

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, header, request::Parts},
    middleware::Next,
    response::Response,
};

use crate::{
    application::user::UserService,
    domain::auth::{AuthError, AuthenticatedUser, CurrentSession},
};

pub use routes::routes;

use self::error_mapper::auth_error_response;

#[derive(Debug, Clone)]
pub struct RequestAuthContext {
    pub access_token: String,
    pub session: CurrentSession,
}

impl RequestAuthContext {
    pub fn user(&self) -> &AuthenticatedUser {
        &self.session.user
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
    read_bearer_token(headers).or_else(|| read_cookie(headers, "tron_access_token"))
}

pub async fn require_auth(
    State(users): State<UserService>,
    mut request: Request,
    next: Next,
) -> Response {
    let access_token = read_access_token(request.headers());
    match users.authenticate_access_token(access_token.as_deref()) {
        Ok(session) => {
            request.extensions_mut().insert(RequestAuthContext {
                access_token: access_token
                    .expect("authenticated requests must include an access token"),
                session,
            });
            next.run(request).await
        }
        Err(error) => auth_error_response(error),
    }
}
