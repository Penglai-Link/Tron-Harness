mod common;

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode, header};
use rusqlite::Connection;
use serde_json::json;
use tron_backend::config::AppConfig;

use common::{
    AUTH_LOGIN, AUTH_PASSWORD, app_with_config, cookie_header_from_set_cookie, header_values,
    request,
};

#[tokio::test]
async fn refresh_sessions_and_oauth2_identity_links_persist_without_plaintext_tokens() {
    let database_path = unique_database_path();
    let config = AppConfig {
        bind_address: "127.0.0.1".into(),
        port: 8080,
        database_url: database_path.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let app = app_with_config(config);

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
        "AC-AUTH-06 RED: internal login should create a persisted refresh session",
    );
    let refresh_cookie_headers = header_values(login_response.headers(), header::SET_COOKIE);
    let refresh_cookie_header = cookie_header_from_set_cookie(&refresh_cookie_headers, &["tron_refresh_token"]);
    let refresh_cookie_value = refresh_cookie_header
        .strip_prefix("tron_refresh_token=")
        .expect("refresh cookie should be present")
        .to_owned();

    let authorize_response = request(
        &app,
        Method::GET,
        "/api/v1/auth/oauth2/authorize/dev",
        None,
        None,
    )
    .await;
    assert_eq!(authorize_response.status(), StatusCode::SEE_OTHER);
    let state_cookie = cookie_header_from_set_cookie(
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
        "AC-AUTH-06 RED: OAuth2 callback should persist an identity link",
    );

    let connection = Connection::open(&database_path).expect("auth test database should be readable");
    let stored_hash: String = connection
        .query_row(
            "SELECT refresh_token_hash FROM auth_refresh_sessions ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("refresh session hash should persist");
    assert!(
        !stored_hash.is_empty() && stored_hash != refresh_cookie_value,
        "refresh session storage must not keep the plaintext refresh token",
    );

    let link_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM user_oauth_identities WHERE provider_code = 'dev'",
            [],
            |row| row.get(0),
        )
        .expect("OAuth2 identity link should persist");
    assert!(link_count >= 1, "OAuth2 callback should create at least one identity link");

    let _ = fs::remove_file(&database_path);
}

fn unique_database_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-auth-{nanos}.sqlite3"))
}
