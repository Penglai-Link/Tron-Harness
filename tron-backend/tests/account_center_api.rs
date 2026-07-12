mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;

use common::{
    AUTH_LOGIN, AUTH_PASSWORD, app, authenticated_bearer_token, request, request_with_headers,
    response_json,
};

#[tokio::test]
async fn account_profile_requires_authentication_and_returns_only_the_current_user() {
    let router = app();
    let anonymous = request(
        &router,
        Method::GET,
        "/api/v1/account/get-profile",
        None,
        None,
    )
    .await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let authorization = authorization_header(&router).await;
    let response = request_with_headers(
        &router,
        Method::GET,
        "/api/v1/account/get-profile",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let payload = response_json(response).await;
    assert_eq!(payload["code"], "SUCCESS");
    assert_eq!(payload["data"]["userCode"], "USR-AUTH-0001");
    assert_eq!(payload["data"]["displayName"], "Auth Operator");
    assert_eq!(payload["data"]["passwordChangeAllowed"], true);
    assert!(payload["data"].get("passwordHash").is_none());
}

#[tokio::test]
async fn account_profile_updates_allowed_fields_and_refreshes_the_session_projection() {
    let router = app();
    let authorization = authorization_header(&router).await;
    let update = request_with_headers(
        &router,
        Method::POST,
        "/api/v1/account/update-profile",
        Some(json!({
            "firstName": "Alex",
            "lastName": "Morgan",
            "displayName": "Alex Morgan",
            "phone": {
                "countryCode": "+86",
                "number": "13800138000"
            },
            "locale": "zh-CN",
            "timezone": "Asia/Shanghai"
        })),
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(update.status(), StatusCode::OK);

    let update_payload = response_json(update).await;
    assert_eq!(update_payload["data"]["displayName"], "Alex Morgan");
    assert_eq!(update_payload["data"]["phone"]["countryCode"], "+86");
    assert_eq!(update_payload["data"]["locale"], "zh-CN");
    assert_eq!(update_payload["data"]["username"], AUTH_LOGIN);

    let session = request_with_headers(
        &router,
        Method::GET,
        "/api/v1/auth/session",
        None,
        None,
        Some(&authorization),
    )
    .await;
    assert_eq!(session.status(), StatusCode::OK);
    let session_payload = response_json(session).await;
    assert_eq!(
        session_payload["data"]["user"]["displayName"],
        "Alex Morgan"
    );
}

#[tokio::test]
async fn account_password_change_validates_current_password_and_rotates_credentials() {
    let router = app();
    let authorization = authorization_header(&router).await;

    let incorrect_current = change_password(
        &router,
        &authorization,
        "not-the-current-password",
        "TronNewPass456!",
    )
    .await;
    assert_eq!(incorrect_current.status(), StatusCode::BAD_REQUEST);
    let incorrect_payload = response_json(incorrect_current).await;
    assert_eq!(
        incorrect_payload["code"],
        "ACCOUNT_CURRENT_PASSWORD_INCORRECT"
    );

    let weak = change_password(&router, &authorization, AUTH_PASSWORD, "weak").await;
    assert_eq!(weak.status(), StatusCode::BAD_REQUEST);
    let weak_payload = response_json(weak).await;
    assert_eq!(weak_payload["code"], "ACCOUNT_PASSWORD_WEAK");

    let new_password = "TronNewPass456!";
    let changed = change_password(&router, &authorization, AUTH_PASSWORD, new_password).await;
    assert_eq!(changed.status(), StatusCode::OK);
    let changed_payload = response_json(changed).await;
    assert_eq!(changed_payload["data"]["changed"], true);
    assert_eq!(changed_payload["data"]["passwordResetRequired"], false);

    let old_login = login(&router, AUTH_PASSWORD).await;
    assert_eq!(old_login.status(), StatusCode::UNAUTHORIZED);

    let new_login = login(&router, new_password).await;
    assert_eq!(new_login.status(), StatusCode::OK);
}

async fn authorization_header(router: &axum::Router) -> String {
    format!("Bearer {}", authenticated_bearer_token(router).await)
}

async fn change_password(
    router: &axum::Router,
    authorization: &str,
    current_password: &str,
    new_password: &str,
) -> axum::response::Response {
    request_with_headers(
        router,
        Method::POST,
        "/api/v1/account/change-password",
        Some(json!({
            "currentPassword": current_password,
            "newPassword": new_password
        })),
        None,
        Some(authorization),
    )
    .await
}

async fn login(router: &axum::Router, password: &str) -> axum::response::Response {
    request(
        router,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": AUTH_LOGIN,
            "password": password
        })),
        None,
    )
    .await
}
