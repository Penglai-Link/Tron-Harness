use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{application::user::UserService, domain::auth::CurrentSession, response::ApiResponse};

use super::{
    error_mapper::auth_error_response,
    read_access_token, read_cookie,
    request::{InternalLoginRequest, OAuth2AuthorizeQuery, OAuth2CallbackQuery},
    response::{AuthGrantDto, AuthProvidersDto, LogoutDto},
};

const ACCESS_COOKIE_NAME: &str = "tron_access_token";
const REFRESH_COOKIE_NAME: &str = "tron_refresh_token";
const OAUTH2_STATE_COOKIE_NAME: &str = "tron_oauth2_state";

pub async fn providers(State(users): State<UserService>) -> Response {
    Json(ApiResponse::success(AuthProvidersDto {
        providers: users.auth_providers(),
    }))
    .into_response()
}

pub async fn login(
    State(users): State<UserService>,
    Json(request): Json<InternalLoginRequest>,
) -> Response {
    match users.login_internal(&request.login, &request.password) {
        Ok(session) => response_with_refresh_cookie(
            StatusCode::OK,
            Json(ApiResponse::success(auth_grant_response(
                &users,
                &session.access_token,
                &session.session,
            ))),
            &users,
            &session.refresh_token,
        ),
        Err(error) => auth_error_response(error),
    }
}

pub async fn current_session(State(users): State<UserService>, headers: HeaderMap) -> Response {
    let access_token = read_access_token(&headers);
    match users.authenticate_access_token(access_token.as_deref()) {
        Ok(authenticated) => Json(ApiResponse::success(authenticated.session)).into_response(),
        Err(error) => auth_error_response(error),
    }
}

pub async fn refresh(State(users): State<UserService>, headers: HeaderMap) -> Response {
    let refresh_token = read_cookie(&headers, REFRESH_COOKIE_NAME);
    match users.refresh_session(refresh_token.as_deref()) {
        Ok(session) => response_with_refresh_cookie(
            StatusCode::OK,
            Json(ApiResponse::success(auth_grant_response(
                &users,
                &session.access_token,
                &session.session,
            ))),
            &users,
            &session.refresh_token,
        ),
        Err(error) => auth_error_response(error),
    }
}

pub async fn logout(State(users): State<UserService>, headers: HeaderMap) -> Response {
    let refresh_token = read_cookie(&headers, REFRESH_COOKIE_NAME);
    match users.logout_session(refresh_token.as_deref()) {
        Ok(()) => response_with_clear_cookies(
            StatusCode::OK,
            Json(ApiResponse::success(LogoutDto { logged_out: true })),
            &users,
        ),
        Err(error) => auth_error_response(error),
    }
}

pub async fn oauth2_authorize(
    State(users): State<UserService>,
    Path(provider_code): Path<String>,
    Query(query): Query<OAuth2AuthorizeQuery>,
) -> Response {
    match users.oauth2_authorize(&provider_code, query.redirect.as_deref()) {
        Ok(result) => {
            let mut response = StatusCode::SEE_OTHER.into_response();
            response.headers_mut().insert(
                header::LOCATION,
                HeaderValue::from_str(&result.redirect_location)
                    .expect("oauth2 authorize redirect should be valid"),
            );
            response.headers_mut().append(
                header::SET_COOKIE,
                HeaderValue::from_str(&state_cookie(&result.state_token, users.cookie_secure()))
                    .expect("oauth2 state cookie should be valid"),
            );
            response
        }
        Err(error) => auth_error_response(error),
    }
}

pub async fn oauth2_callback(
    State(users): State<UserService>,
    Path(provider_code): Path<String>,
    Query(query): Query<OAuth2CallbackQuery>,
    headers: HeaderMap,
) -> Response {
    let state_cookie = read_cookie(&headers, OAUTH2_STATE_COOKIE_NAME);
    match users
        .oauth2_callback(
            &provider_code,
            state_cookie.as_deref(),
            query.state.as_deref(),
            query.code.as_deref(),
            query
                .error
                .as_deref()
                .or(query.error_description.as_deref()),
        )
        .await
    {
        Ok(session) => {
            let redirect_target = session.redirect_path.unwrap_or_else(|| "/dashboard".into());
            let mut response = StatusCode::SEE_OTHER.into_response();
            response.headers_mut().insert(
                header::LOCATION,
                HeaderValue::from_str(&format!("/auth/callback?redirect={redirect_target}"))
                    .expect("oauth2 callback redirect should be valid"),
            );
            append_auth_cookies(response.headers_mut(), &users, &session.refresh_token);
            response.headers_mut().append(
                header::SET_COOKIE,
                HeaderValue::from_str(&clear_state_cookie(users.cookie_secure()))
                    .expect("oauth2 state clear cookie should be valid"),
            );
            response
        }
        Err(error) => auth_error_response(error),
    }
}

fn response_with_refresh_cookie<T: IntoResponse>(
    status: StatusCode,
    body: T,
    users: &UserService,
    refresh_token: &str,
) -> Response {
    let mut response = (status, body).into_response();
    append_auth_cookies(response.headers_mut(), users, refresh_token);
    response
}

fn response_with_clear_cookies<T: IntoResponse>(
    status: StatusCode,
    body: T,
    users: &UserService,
) -> Response {
    let mut response = (status, body).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_auth_cookie(
            ACCESS_COOKIE_NAME,
            users.cookie_secure(),
        ))
        .expect("clear access cookie should be valid"),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_auth_cookie(
            REFRESH_COOKIE_NAME,
            users.cookie_secure(),
        ))
        .expect("clear refresh cookie should be valid"),
    );
    response
}

fn append_auth_cookies(headers: &mut HeaderMap, users: &UserService, refresh_token: &str) {
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&auth_cookie(
            REFRESH_COOKIE_NAME,
            refresh_token,
            users.refresh_token_ttl_seconds(),
            users.cookie_secure(),
        ))
        .expect("refresh cookie should be valid"),
    );
}

fn auth_grant_response(
    users: &UserService,
    access_token: &str,
    session: &CurrentSession,
) -> AuthGrantDto {
    AuthGrantDto {
        access_token: access_token.to_owned(),
        token_type: "Bearer".into(),
        expires_in: users.access_token_ttl_seconds(),
        session: session.clone(),
    }
}

fn auth_cookie(name: &str, value: &str, max_age: i64, secure: bool) -> String {
    format!(
        "{name}={value}; HttpOnly; Path=/; SameSite=Lax; Max-Age={max_age}{}",
        if secure { "; Secure" } else { "" }
    )
}

fn clear_auth_cookie(name: &str, secure: bool) -> String {
    format!(
        "{name}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0{}",
        if secure { "; Secure" } else { "" }
    )
}

fn state_cookie(value: &str, secure: bool) -> String {
    format!(
        "{OAUTH2_STATE_COOKIE_NAME}={value}; HttpOnly; Path=/; SameSite=Lax; Max-Age=300{}",
        if secure { "; Secure" } else { "" }
    )
}

fn clear_state_cookie(secure: bool) -> String {
    format!(
        "{OAUTH2_STATE_COOKIE_NAME}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0{}",
        if secure { "; Secure" } else { "" }
    )
}
