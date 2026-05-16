mod common;

use axum::http::{Method, StatusCode, header};
use serde_json::json;

use common::{
    AUTH_EMAIL, AUTH_LOGIN, AUTH_PASSWORD, app, cookie_header_from_set_cookie, header_values,
    request, request_with_headers, response_json,
};

#[tokio::test]
async fn internal_login_accepts_username_and_returns_current_session() {
    let app = app();
    let login_response = request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": AUTH_LOGIN,
            "password": AUTH_PASSWORD,
        })),
        None,
    )
    .await;

    assert_eq!(
        login_response.status(),
        StatusCode::OK,
        "AC-AUTH-01 RED: internal login endpoint should authenticate a valid active operator",
    );
    let set_cookie_headers = header_values(login_response.headers(), header::SET_COOKIE);
    let login_json = response_json(login_response).await;
    assert_eq!(login_json["code"], "SUCCESS");
    assert_eq!(login_json["data"]["session"]["user"]["username"], AUTH_LOGIN);
    assert_eq!(login_json["data"]["session"]["authenticated"], true);
    assert_eq!(login_json["data"]["tokenType"], "Bearer");
    assert!(
        login_json["data"]["accessToken"].as_str().is_some_and(|value| !value.is_empty()),
        "login should return a bearer access token in the response body",
    );
    assert!(
        set_cookie_headers
            .iter()
            .any(|value| value.starts_with("tron_refresh_token=")),
        "login should set the refresh-token cookie",
    );
    assert!(
        !set_cookie_headers
            .iter()
            .any(|value| value.starts_with("tron_access_token=")),
        "login should not set an access-token cookie anymore",
    );

    let refresh_cookie_header =
        cookie_header_from_set_cookie(&set_cookie_headers, &["tron_refresh_token"]);
    let cookie_only_session_response = request(
        &app,
        Method::GET,
        "/api/v1/auth/session",
        None,
        Some(&refresh_cookie_header),
    )
    .await;
    assert_eq!(
        cookie_only_session_response.status(),
        StatusCode::UNAUTHORIZED,
        "AC-AUTH-01 RED: session inspection should no longer accept cookie-based access fallback",
    );

    let authorization = format!(
        "Bearer {}",
        login_json["data"]["accessToken"]
            .as_str()
            .expect("login should return an accessToken"),
    );
    let session_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/auth/session",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(session_response.status(), StatusCode::OK);
    let session_json = response_json(session_response).await;
    assert_eq!(session_json["code"], "SUCCESS");
    assert_eq!(session_json["data"]["user"]["username"], AUTH_LOGIN);
    assert_eq!(session_json["data"]["authMethod"], "internal");
}

#[tokio::test]
async fn internal_login_accepts_email_for_the_same_operator() {
    let app = app();
    let response = request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": AUTH_EMAIL,
            "password": AUTH_PASSWORD,
        })),
        None,
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-AUTH-01 RED: internal login should accept email as the login identifier",
    );
    let json = response_json(response).await;
    assert_eq!(json["data"]["session"]["user"]["email"], AUTH_EMAIL);
}
