mod common;

use axum::http::{Method, StatusCode, header};
use serde_json::json;

use common::{
    AUTH_LOGIN, AUTH_PASSWORD, app, cookie_header_from_set_cookie, header_values, request,
    request_with_headers, response_json,
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
    let set_cookie_headers = header_values(login_response.headers(), header::SET_COOKIE);
    let login_json = response_json(login_response).await;
    let refresh_cookie_header = cookie_header_from_set_cookie(
        &set_cookie_headers,
        &["tron_refresh_token"],
    );
    let authorization = format!(
        "Bearer {}",
        login_json["data"]["accessToken"]
            .as_str()
            .expect("login should return an access token"),
    );

    let cookie_only_response = request(
        &app,
        Method::GET,
        "/api/v1/admin/users",
        None,
        Some(&refresh_cookie_header),
    )
    .await;
    assert_eq!(
        cookie_only_response.status(),
        StatusCode::UNAUTHORIZED,
        "AC-AUTH-08 RED: admin APIs should no longer accept cookie-only access after the standard Bearer contract lands",
    );

    let authenticated_response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/admin/users",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(
        authenticated_response.status(),
        StatusCode::OK,
        "AC-AUTH-08 RED: user-management APIs should remain usable with a Bearer access token after successful sign-in",
    );
}
