mod common;

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    middleware,
    routing::get,
};
use serde_json::{Value, json};
use tron_backend::{
    adapters::inbound::http::auth::{RequestAuthContext, require_auth},
    app::AppState,
    config::AppConfig,
};

use common::{AUTH_LOGIN, app, authenticated_bearer_token, prepared_config, request_with_headers, response_json};

#[tokio::test]
async fn protected_admin_routes_accept_authorization_bearer_tokens() {
    let app = app();
    let access_token = authenticated_bearer_token(&app).await;
    let authorization = format!("Bearer {access_token}");

    let response = request_with_headers(
        &app,
        Method::GET,
        "/api/v1/admin/users",
        None,
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-AUTH-08 RED: protected admin routes should accept Authorization bearer JWTs",
    );
}

#[tokio::test]
async fn auth_middleware_injects_current_user_context_for_protected_handlers() {
    let login_app = app();
    let access_token = authenticated_bearer_token(&login_app).await;
    let authorization = format!("Bearer {access_token}");

    let state = AppState::new(prepared_config(AppConfig::default()));
    let router = Router::new()
        .route("/protected", get(protected_identity))
        .route_layer(middleware::from_fn_with_state(
            state.users.clone(),
            require_auth,
        ));

    let response = request_with_headers(
        &router,
        Method::GET,
        "/protected",
        None,
        None,
        Some(&authorization),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-AUTH-05 RED: auth middleware should expose current user context to downstream handlers",
    );

    let json = response_json(response).await;
    assert_eq!(json["user"]["username"], AUTH_LOGIN);
    assert_eq!(json["authenticated"], json!(true));
}

async fn protected_identity(auth: RequestAuthContext) -> Json<Value> {
    Json(json!({
        "authenticated": auth.session.authenticated,
        "user": {
            "userCode": auth.user().user_code,
            "username": auth.user().username,
        }
    }))
}
