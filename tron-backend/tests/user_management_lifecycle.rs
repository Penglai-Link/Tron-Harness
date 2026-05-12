use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use tron_backend::{app::build_router, config::AppConfig};

#[tokio::test]
async fn create_user_returns_stringified_id_and_user_code() {
    let response = request(
        Method::POST,
        "/api/v1/admin/users",
        Some(valid_create_payload(
            None,
            "lena.create",
            "lena.create@tron.ai",
        )),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "TDD-UM-02 RED: create endpoint must exist and return 201 with lifecycle payload",
    );

    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    assert!(
        json["data"]["id"].is_string(),
        "Snowflake ID must be a decimal string"
    );
    assert!(
        json["data"]["userCode"].is_string(),
        "stable userCode must be returned"
    );
    assert_eq!(json["data"]["username"], "lena.create");
    assert_eq!(json["data"]["email"], "lena.create@tron.ai");
}

#[tokio::test]
async fn create_user_rejects_duplicate_user_code_without_overwriting_existing_user() {
    let response = request(
        Method::POST,
        "/api/v1/admin/users",
        Some(valid_create_payload(
            Some("USR-0001"),
            "duplicate.code",
            "duplicate.code@tron.ai",
        )),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::CONFLICT,
        "TDD-UM-02 RED: duplicate explicit userCode must be rejected before insertion",
    );
    let json = response_json(response).await;
    assert_eq!(json["code"], "ADMIN_USER_CODE_DUPLICATE");

    let detail_response = request(Method::GET, "/api/v1/admin/users/USR-0001", None).await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_json = response_json(detail_response).await;
    assert_eq!(
        detail_json["data"]["username"], "alex.johnson",
        "duplicate create must not overwrite the existing user",
    );
    assert_eq!(detail_json["data"]["email"], "alex.johnson@tron.ai");
}

#[tokio::test]
async fn create_user_rejects_duplicate_username_and_email() {
    let duplicate_username = request(
        Method::POST,
        "/api/v1/admin/users",
        Some(valid_create_payload(
            Some("USR-DUPLICATE-USERNAME"),
            "alex.johnson",
            "unique.duplicate.username@tron.ai",
        )),
    )
    .await;
    assert_eq!(
        duplicate_username.status(),
        StatusCode::CONFLICT,
        "TDD-UM-02 RED: duplicate username must be rejected",
    );
    let duplicate_username_json = response_json(duplicate_username).await;
    assert_eq!(
        duplicate_username_json["code"],
        "ADMIN_USER_USERNAME_DUPLICATE"
    );

    let duplicate_email = request(
        Method::POST,
        "/api/v1/admin/users",
        Some(valid_create_payload(
            Some("USR-DUPLICATE-EMAIL"),
            "unique.duplicate.email",
            "alex.johnson@tron.ai",
        )),
    )
    .await;
    assert_eq!(
        duplicate_email.status(),
        StatusCode::CONFLICT,
        "TDD-UM-02 RED: duplicate email must be rejected",
    );
    let duplicate_email_json = response_json(duplicate_email).await;
    assert_eq!(duplicate_email_json["code"], "ADMIN_USER_EMAIL_DUPLICATE");
}

#[tokio::test]
async fn create_user_rejects_missing_or_blank_required_profile_fields() {
    for field in ["username", "firstName", "lastName", "displayName", "email"] {
        let mut missing_payload = valid_create_payload(
            Some(&format!("USR-MISSING-{field}")),
            &format!("missing.{field}"),
            &format!("missing.{field}@tron.ai"),
        );
        missing_payload
            .as_object_mut()
            .expect("payload must be a JSON object")
            .remove(field);

        let missing_response =
            request(Method::POST, "/api/v1/admin/users", Some(missing_payload)).await;
        assert_eq!(
            missing_response.status(),
            StatusCode::BAD_REQUEST,
            "TDD-UM-02 RED: missing {field} must be rejected",
        );
        let missing_json = response_json(missing_response).await;
        assert_eq!(missing_json["code"], "ADMIN_USER_PROFILE_FIELD_REQUIRED");

        let mut blank_payload = valid_create_payload(
            Some(&format!("USR-BLANK-{field}")),
            &format!("blank.{field}"),
            &format!("blank.{field}@tron.ai"),
        );
        blank_payload
            .as_object_mut()
            .expect("payload must be a JSON object")
            .insert(field.to_owned(), Value::String("   ".into()));

        let blank_response =
            request(Method::POST, "/api/v1/admin/users", Some(blank_payload)).await;
        assert_eq!(
            blank_response.status(),
            StatusCode::BAD_REQUEST,
            "TDD-UM-02 RED: blank {field} must be rejected",
        );
        let blank_json = response_json(blank_response).await;
        assert_eq!(blank_json["code"], "ADMIN_USER_PROFILE_FIELD_REQUIRED");
    }
}

#[tokio::test]
async fn create_user_rejects_weak_provided_initial_password() {
    let mut weak_password_payload = valid_create_payload(
        Some("USR-WEAK-PASSWORD"),
        "weak.password",
        "weak.password@tron.ai",
    );
    weak_password_payload
        .as_object_mut()
        .expect("payload must be a JSON object")
        .insert("initialPassword".into(), Value::String("short".into()));

    let response = request(
        Method::POST,
        "/api/v1/admin/users",
        Some(weak_password_payload),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "TDD-UM-02 RED: obvious weak provided initial passwords must be rejected",
    );
    let json = response_json(response).await;
    assert_eq!(json["code"], "ADMIN_USER_PASSWORD_WEAK");
}

#[tokio::test]
async fn create_user_accepts_strong_provided_initial_password() {
    let mut strong_password_payload = valid_create_payload(
        Some("USR-STRONG-PASSWORD"),
        "strong.password",
        "strong.password@tron.ai",
    );
    strong_password_payload
        .as_object_mut()
        .expect("payload must be a JSON object")
        .insert(
            "initialPassword".into(),
            Value::String("StrongerPass1!".into()),
        );

    let response = request(
        Method::POST,
        "/api/v1/admin/users",
        Some(strong_password_payload),
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "TDD-UM-02 RED: strong provided initial passwords should remain accepted",
    );
    let json = response_json(response).await;
    assert_eq!(json["code"], "SUCCESS");
    assert_eq!(json["data"]["userCode"], "USR-STRONG-PASSWORD");
}

#[tokio::test]
async fn archive_then_restore_transitions_user_status() {
    let archive_response =
        request(Method::POST, "/api/v1/admin/users/USR-0009/archive", None).await;
    assert_eq!(
        archive_response.status(),
        StatusCode::OK,
        "TDD-UM-02 RED: archive transition endpoint is missing",
    );

    let restore_response =
        request(Method::POST, "/api/v1/admin/users/USR-0009/restore", None).await;
    assert_eq!(
        restore_response.status(),
        StatusCode::OK,
        "TDD-UM-02 RED: restore transition endpoint is missing",
    );

    let detail_response = request(Method::GET, "/api/v1/admin/users/USR-0009", None).await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let json = response_json(detail_response).await;
    assert_eq!(json["data"]["status"], "Active");
}

fn valid_create_payload(user_code: Option<&str>, username: &str, email: &str) -> Value {
    let mut payload = json!({
        "username": username,
        "firstName": "Lena",
        "lastName": "Create",
        "displayName": "Lena Create",
        "email": email,
        "roleCodes": ["DEVELOPER"],
        "spaceCodes": ["PLATFORM"]
    });

    if let Some(user_code) = user_code {
        payload
            .as_object_mut()
            .expect("payload must be a JSON object")
            .insert("userCode".into(), Value::String(user_code.into()));
    }

    payload
}

async fn request(method: Method, uri: &str, body: Option<Value>) -> axum::response::Response {
    let router = build_router(AppConfig::default());
    let mut builder = Request::builder().method(method).uri(uri);
    let request_body = if let Some(json_body) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(json_body.to_string())
    } else {
        Body::empty()
    };

    router
        .oneshot(
            builder
                .body(request_body)
                .expect("request should be constructible"),
        )
        .await
        .expect("router should return a response")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}
