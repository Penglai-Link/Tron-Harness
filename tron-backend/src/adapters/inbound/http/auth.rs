mod error_mapper;
mod handler;
mod request;
mod response;
mod routes;

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::application::user::UserService;

pub use routes::routes;

use self::error_mapper::auth_error_response;

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

pub async fn require_auth(
    State(users): State<UserService>,
    request: Request,
    next: Next,
) -> Response {
    let access_token = read_cookie(request.headers(), "tron_access_token");
    match users.authorize_access_token(access_token.as_deref()) {
        Ok(()) => next.run(request).await,
        Err(error) => auth_error_response(error),
    }
}
