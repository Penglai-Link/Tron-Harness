mod common;

use axum::http::{Method, StatusCode, header};
use common::{app, cookie_header_from_set_cookie, header_value_to_string, header_values, request, response_json};

#[tokio::test]
async fn oauth2_authorize_and_callback_issue_an_authenticated_session() {
    let app = app();
    let authorize_response = request(
        &app,
        Method::GET,
        "/api/v1/auth/oauth2/authorize/dev?redirect=%2Fadmin%2Fusers",
        None,
        None,
    )
    .await;

    assert_eq!(
        authorize_response.status(),
        StatusCode::SEE_OTHER,
        "AC-AUTH-02 RED: OAuth2 authorize endpoint should redirect to a provider or callback flow",
    );

    let state_cookie = cookie_header_from_set_cookie(
        &header_values(authorize_response.headers(), header::SET_COOKIE),
        &["tron_oauth2_state"],
    );
    let callback_location = header_value_to_string(
        authorize_response
            .headers()
            .get(header::LOCATION)
            .expect("authorize redirect should include Location"),
    );

    let callback_response = request(
        &app,
        Method::GET,
        &callback_location,
        None,
        Some(&state_cookie),
    )
    .await;

    assert_eq!(
        callback_response.status(),
        StatusCode::SEE_OTHER,
        "AC-AUTH-02 RED: OAuth2 callback should complete the session flow and redirect back to the frontend callback page",
    );
    let set_cookie_headers = header_values(callback_response.headers(), header::SET_COOKIE);
    let callback_redirect = header_value_to_string(
        callback_response
            .headers()
            .get(header::LOCATION)
            .expect("callback redirect should include Location"),
    );
    assert!(
        callback_redirect.starts_with("/auth/callback"),
        "callback should redirect to the frontend auth callback route, got {callback_redirect}",
    );

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
    assert_eq!(session_json["data"]["authMethod"], "oauth2");
    assert_eq!(session_json["data"]["provider"]["code"], "dev");
}

