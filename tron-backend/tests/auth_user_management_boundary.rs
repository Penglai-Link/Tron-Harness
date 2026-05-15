mod common;

use axum::http::{Method, StatusCode, header};
use serde_json::json;

use common::{
    AUTH_LOGIN, AUTH_PASSWORD, app, cookie_header_from_set_cookie, header_values, request,
};

#[tokio::test]
async fn user_management_routes_require_authentication_and_work_after_login() {
    let app = app();

    let anonymous_response = request(&app, Method::GET, "/api/v1/admin/users", None, None).await;
    assert_eq!(
        anonymous_response.status(),
        StatusCode::UNAUTHORIZED,
        "AC-AUTH-08 RED: user-management APIs should reject anonymous requests once auth is enabled",
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
    let session_cookie_header = cookie_header_from_set_cookie(
        &header_values(login_response.headers(), header::SET_COOKIE),
        &["tron_access_token", "tron_refresh_token"],
    );

    let authenticated_response = request(
        &app,
        Method::GET,
        "/api/v1/admin/users",
        None,
        Some(&session_cookie_header),
    )
    .await;
    assert_eq!(
        authenticated_response.status(),
        StatusCode::OK,
        "AC-AUTH-08 RED: user-management APIs should remain usable after successful sign-in",
    );
}

