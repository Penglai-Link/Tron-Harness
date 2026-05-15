mod common;

use axum::http::{Method, StatusCode, header};
use serde_json::json;

use common::{
    AUTH_LOGIN, AUTH_PASSWORD, app, cookie_header_from_set_cookie, header_values, request,
    response_json,
};

#[tokio::test]
async fn providers_refresh_and_logout_follow_the_uniform_auth_contract() {
    let app = app();

    let providers_response = request(&app, Method::GET, "/api/v1/auth/providers", None, None).await;
    assert_eq!(
        providers_response.status(),
        StatusCode::OK,
        "AC-AUTH-05 RED: provider discovery endpoint should exist for the login page",
    );
    let providers_json = response_json(providers_response).await;
    assert_eq!(providers_json["code"], "SUCCESS");
    assert!(
        providers_json["data"]["providers"].is_array(),
        "provider discovery should return a providers array",
    );

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
    assert_eq!(login_response.status(), StatusCode::OK);
    let set_cookie_headers = header_values(login_response.headers(), header::SET_COOKIE);
    let refresh_cookie_header = cookie_header_from_set_cookie(&set_cookie_headers, &["tron_refresh_token"]);

    let refresh_response = request(
        &app,
        Method::POST,
        "/api/v1/auth/refresh",
        Some(json!({})),
        Some(&refresh_cookie_header),
    )
    .await;
    assert_eq!(
        refresh_response.status(),
        StatusCode::OK,
        "AC-AUTH-05 RED: refresh endpoint should rotate auth cookies for a valid refresh session",
    );
    let refresh_set_cookie_headers = header_values(refresh_response.headers(), header::SET_COOKIE);
    let refresh_json = response_json(refresh_response).await;
    assert_eq!(refresh_json["code"], "SUCCESS");

    let logout_cookie_header =
        cookie_header_from_set_cookie(&refresh_set_cookie_headers, &["tron_refresh_token"]);
    let logout_response = request(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        Some(json!({})),
        Some(&logout_cookie_header),
    )
    .await;
    assert_eq!(
        logout_response.status(),
        StatusCode::OK,
        "AC-AUTH-05 RED: logout endpoint should clear the authenticated session contract",
    );
    let logout_json = response_json(logout_response).await;
    assert_eq!(logout_json["code"], "SUCCESS");
}
