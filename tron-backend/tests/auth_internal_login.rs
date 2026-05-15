mod common;

use axum::http::{Method, StatusCode, header};
use serde_json::json;

use common::{
    AUTH_EMAIL, AUTH_LOGIN, AUTH_PASSWORD, app, cookie_header_from_set_cookie, header_values,
    request, response_json,
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
    assert_eq!(login_json["data"]["user"]["username"], AUTH_LOGIN);
    assert_eq!(login_json["data"]["authenticated"], true);

    let session_cookie_header = cookie_header_from_set_cookie(
        &set_cookie_headers,
        &["tron_access_token", "tron_refresh_token"],
    );
    let session_response = request(
        &app,
        Method::GET,
        "/api/v1/auth/session",
        None,
        Some(&session_cookie_header),
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
    assert_eq!(json["data"]["user"]["email"], AUTH_EMAIL);
}

