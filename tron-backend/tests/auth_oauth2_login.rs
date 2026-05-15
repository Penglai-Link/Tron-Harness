mod common;

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode, header};
use common::{
    GITHUB_TEST_CODE, GitHubEmailFixture, GitHubProfileFixture, app_with_config,
    cookie_header_from_set_cookie, github_mock_server_with_profile, github_oauth_config,
    header_value_to_string, header_values, request, response_json,
};
use serde_json::json;
use url::Url;

#[tokio::test]
async fn oauth2_authorize_and_callback_auto_provision_an_authenticated_session() {
    let database_path = unique_database_path();
    let github = github_mock_server_with_profile(GitHubProfileFixture {
        id: 25_652_685,
        login: "CloudSen".into(),
        name: Some("CloudS3n".into()),
        emails: vec![GitHubEmailFixture {
            email: "www.402645063@gmail.com".into(),
            primary: true,
            verified: true,
        }],
    })
    .await;
    let mut config = github_oauth_config(&github);
    config.database_url = database_path.to_string_lossy().into_owned();
    let app = app_with_config(config);
    let authorize_response = request(
        &app,
        Method::GET,
        "/api/v1/auth/oauth2/authorize/github?redirect=%2Fadmin%2Fusers",
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
    let authorize_url = Url::parse(&callback_location).expect("authorize redirect should be a valid URL");
    assert_eq!(authorize_url.path(), "/login/oauth/authorize");
    assert_eq!(
        authorize_url
            .query_pairs()
            .find(|(key, _)| key == "client_id")
            .map(|(_, value)| value.into_owned()),
        Some("github-client-id".into()),
    );
    assert_eq!(
        authorize_url
            .query_pairs()
            .find(|(key, _)| key == "redirect_uri")
            .map(|(_, value)| value.into_owned()),
        Some("http://127.0.0.1:5173/api/v1/auth/oauth2/callback/github".into()),
    );
    let state = authorize_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .expect("authorize redirect should include an oauth2 state token");

    let callback_response = request(
        &app,
        Method::GET,
        &format!("/api/v1/auth/oauth2/callback/github?code={GITHUB_TEST_CODE}&state={state}"),
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
    assert_eq!(session_json["data"]["provider"]["code"], "github");
    assert_eq!(session_json["data"]["user"]["username"], "cloudsen");
    assert_eq!(session_json["data"]["user"]["email"], "www.402645063@gmail.com");
    assert_eq!(session_json["data"]["user"]["roleCodes"], json!(["VIEWER"]));
    assert_eq!(session_json["data"]["user"]["spaceCodes"], json!(["PLATFORM"]));
    assert_eq!(session_json["data"]["user"]["passwordResetRequired"], true);

    let _ = fs::remove_file(&database_path);
}

#[tokio::test]
async fn oauth2_callback_reuses_existing_subject_link_when_github_email_changes() {
    let database_path = unique_database_path();
    let first_github = github_mock_server_with_profile(GitHubProfileFixture {
        id: 25_652_685,
        login: "CloudSen".into(),
        name: Some("CloudS3n".into()),
        emails: vec![GitHubEmailFixture {
            email: "www.402645063@gmail.com".into(),
            primary: true,
            verified: true,
        }],
    })
    .await;
    let mut first_config = github_oauth_config(&first_github);
    first_config.database_url = database_path.to_string_lossy().into_owned();
    let first_app = app_with_config(first_config);

    let first_session_json =
        oauth2_session_json(&first_app, "/api/v1/auth/oauth2/authorize/github?redirect=%2Fdashboard").await;
    let first_user_code = first_session_json["data"]["user"]["userCode"]
        .as_str()
        .expect("first OAuth2 login should return a userCode")
        .to_owned();
    drop(first_app);
    drop(first_github);

    let second_github = github_mock_server_with_profile(GitHubProfileFixture {
        id: 25_652_685,
        login: "CloudSen".into(),
        name: Some("CloudS3n".into()),
        emails: vec![GitHubEmailFixture {
            email: "cloudsen.changed@example.com".into(),
            primary: true,
            verified: true,
        }],
    })
    .await;
    let mut second_config = github_oauth_config(&second_github);
    second_config.database_url = database_path.to_string_lossy().into_owned();
    let second_app = app_with_config(second_config);

    let second_session_json =
        oauth2_session_json(&second_app, "/api/v1/auth/oauth2/authorize/github?redirect=%2Fdashboard").await;
    assert_eq!(second_session_json["data"]["user"]["userCode"], json!(first_user_code));
    assert_eq!(second_session_json["data"]["user"]["email"], json!("www.402645063@gmail.com"));

    let _ = fs::remove_file(&database_path);
}

async fn oauth2_session_json(app: &axum::Router, authorize_uri: &str) -> serde_json::Value {
    let authorize_response = request(app, Method::GET, authorize_uri, None, None).await;
    assert_eq!(authorize_response.status(), StatusCode::SEE_OTHER);

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
    let authorize_url = Url::parse(&callback_location).expect("authorize redirect should be a valid URL");
    let state = authorize_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .expect("authorize redirect should include an oauth2 state token");

    let callback_response = request(
        app,
        Method::GET,
        &format!("/api/v1/auth/oauth2/callback/github?code={GITHUB_TEST_CODE}&state={state}"),
        None,
        Some(&state_cookie),
    )
    .await;
    assert_eq!(callback_response.status(), StatusCode::SEE_OTHER);

    let session_cookie_header = cookie_header_from_set_cookie(
        &header_values(callback_response.headers(), header::SET_COOKIE),
        &["tron_access_token", "tron_refresh_token"],
    );
    let session_response = request(
        app,
        Method::GET,
        "/api/v1/auth/session",
        None,
        Some(&session_cookie_header),
    )
    .await;
    assert_eq!(session_response.status(), StatusCode::OK);
    response_json(session_response).await
}

fn unique_database_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-auth-auto-provision-{nanos}.sqlite3"))
}
