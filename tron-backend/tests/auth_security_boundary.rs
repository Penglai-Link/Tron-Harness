mod common;

use axum::http::{Method, StatusCode, header};
use serde_json::json;

use common::{ARCHIVED_LOGIN, AUTH_LOGIN, AUTH_PASSWORD, app, cookie_header_from_set_cookie, header_values, request, response_json};

#[tokio::test]
async fn invalid_password_archived_user_invalid_refresh_and_bad_oauth_state_are_rejected() {
    let app = app();

    let invalid_password_response = request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": AUTH_LOGIN,
            "password": "wrong-password",
        })),
        None,
    )
    .await;
    assert_eq!(
        invalid_password_response.status(),
        StatusCode::UNAUTHORIZED,
        "AC-AUTH-07 RED: wrong passwords should be rejected with an auth failure",
    );
    let invalid_password_json = response_json(invalid_password_response).await;
    assert_eq!(invalid_password_json["code"], "AUTH_INVALID_CREDENTIALS");

    let archived_response = request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": ARCHIVED_LOGIN,
            "password": AUTH_PASSWORD,
        })),
        None,
    )
    .await;
    assert_eq!(
        archived_response.status(),
        StatusCode::FORBIDDEN,
        "AC-AUTH-07 RED: archived users must not be able to sign in",
    );
    let archived_json = response_json(archived_response).await;
    assert_eq!(archived_json["code"], "AUTH_USER_INACTIVE");

    let invalid_refresh_response = request(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({})),
        Some("tron_refresh_token=invalid-token"),
    )
    .await;
    assert_eq!(
        invalid_refresh_response.status(),
        StatusCode::UNAUTHORIZED,
        "AC-AUTH-07 RED: invalid refresh tokens should be rejected",
    );
    let invalid_refresh_json = response_json(invalid_refresh_response).await;
    assert_eq!(invalid_refresh_json["code"], "AUTH_REFRESH_INVALID");

    let authorize_response = request(&app, Method::GET, "/api/v1/auth/oauth2/authorize/dev", None, None).await;
    assert_eq!(authorize_response.status(), StatusCode::SEE_OTHER);
    let _good_state_cookie = cookie_header_from_set_cookie(
        &header_values(authorize_response.headers(), header::SET_COOKIE),
        &["tron_oauth2_state"],
    );
    let callback_location = authorize_response
        .headers()
        .get(header::LOCATION)
        .expect("authorize redirect should include Location")
        .to_str()
        .expect("Location should be valid text")
        .to_owned();
    let bad_callback_location = callback_location.replace("state=", "state=bad-");

    let bad_state_response = request(
        &app,
        Method::GET,
        &bad_callback_location,
        None,
        Some("tron_oauth2_state=another-state"),
    )
    .await;
    assert_eq!(
        bad_state_response.status(),
        StatusCode::BAD_REQUEST,
        "AC-AUTH-07 RED: OAuth2 callbacks must reject mismatched state",
    );
    let bad_state_json = response_json(bad_state_response).await;
    assert_eq!(bad_state_json["code"], "AUTH_OAUTH2_STATE_INVALID");
}

