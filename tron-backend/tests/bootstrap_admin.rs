use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode};
use rusqlite::Connection;
use serde_json::json;
use tron_backend::{app::build_router, config::AppConfig};

mod common;
use common::{request, response_json};

static TEST_DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn startup_bootstraps_platform_admin_when_database_is_empty() {
    let database_path = temp_database_path("bootstrap-admin");
    let mut config = AppConfig::default();
    config.database_url = database_path.to_string_lossy().into_owned();

    let router = build_router(config);

    let login_response = request(
        &router,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": "admin",
            "password": "TronAdminPass123!",
        })),
        None,
    )
    .await;

    assert_eq!(
        login_response.status(),
        StatusCode::OK,
        "empty database startup should create a usable bootstrap admin login",
    );
    let login_json = response_json(login_response).await;
    assert_eq!(login_json["data"]["session"]["user"]["username"], "admin");
    assert_eq!(
        login_json["data"]["session"]["user"]["roleCodes"],
        json!(["ADMIN"])
    );
    assert_eq!(
        login_json["data"]["session"]["user"]["spaceCodes"],
        json!(["ENTERPRISE"])
    );
    assert_eq!(
        login_json["data"]["session"]["user"]["passwordResetRequired"],
        true
    );

    let connection = Connection::open(&database_path).expect("bootstrap database should exist");
    let admin_role_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM user_role_assignments WHERE user_code = 'USR-ADMIN' AND role_code = 'ADMIN'",
            [],
            |row| row.get(0),
        )
        .expect("admin role assignment should be queryable");
    assert_eq!(admin_role_count, 1);

    let enterprise_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM workspaces WHERE workspace_code = 'ENTERPRISE' AND status = 'active'",
            [],
            |row| row.get(0),
        )
        .expect("enterprise workspace should be queryable");
    assert_eq!(enterprise_count, 1);

    let owner_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM workspace_user_access WHERE workspace_code = 'ENTERPRISE' AND user_code = 'USR-ADMIN' AND is_owner = 1",
            [],
            |row| row.get(0),
        )
        .expect("enterprise owner grant should be queryable");
    assert_eq!(owner_count, 1);

    let _ = fs::remove_file(database_path);
}

#[tokio::test]
async fn startup_backfills_existing_admin_related_data() {
    let database_path = temp_database_path("bootstrap-existing-admin");
    let connection = Connection::open(&database_path).expect("bootstrap database should open");
    connection
        .execute_batch(include_str!(
            "../migrations/20260511_0001_create_user_management_users.sql"
        ))
        .expect("users table should initialize");
    connection
        .execute(
            "INSERT INTO users (
                id, user_code, username, first_name, last_name, display_name, email,
                phone_country_code, phone_number, avatar_url, locale, timezone, status,
                password_hash, password_hash_algorithm, password_updated_at,
                password_reset_required, last_active_at, archived_at, created_at, updated_at
            ) VALUES (
                734200000000000099, 'CUSTOM-ADMIN', 'admin', 'Existing', 'Admin',
                'Existing Admin', 'existing.admin@tron.ai', NULL, NULL, NULL, 'en-US', 'UTC',
                'active', '$2b$04$D9rkxKMIxlImJxFTmA7FCer4A/bma13H4Yq7ooBXLCHx8xv5lwMvW',
                'bcrypt', 1779753600000, 1, NULL, NULL, 1779753600000, 1779753600000
            )",
            [],
        )
        .expect("existing admin should seed");
    drop(connection);

    let mut config = AppConfig::default();
    config.database_url = database_path.to_string_lossy().into_owned();

    let router = build_router(config);

    let login_response = request(
        &router,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({
            "login": "admin",
            "password": "TronAdminPass123!",
        })),
        None,
    )
    .await;

    assert_eq!(
        login_response.status(),
        StatusCode::OK,
        "existing admin should receive missing bootstrap assignments",
    );
    let login_json = response_json(login_response).await;
    assert_eq!(login_json["data"]["session"]["user"]["username"], "admin");
    assert_eq!(
        login_json["data"]["session"]["user"]["roleCodes"],
        json!(["ADMIN"])
    );
    assert_eq!(
        login_json["data"]["session"]["user"]["spaceCodes"],
        json!(["ENTERPRISE"])
    );

    let connection =
        Connection::open(&database_path).expect("bootstrap database should remain queryable");
    let inserted_admin_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM users WHERE username = 'admin'",
            [],
            |row| row.get(0),
        )
        .expect("admin users should be queryable");
    assert_eq!(inserted_admin_count, 1);

    let existing_role_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM user_role_assignments WHERE user_code = 'CUSTOM-ADMIN' AND role_code = 'ADMIN'",
            [],
            |row| row.get(0),
        )
        .expect("existing admin role assignment should be queryable");
    assert_eq!(existing_role_count, 1);

    let existing_owner_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM workspace_user_access WHERE workspace_code = 'ENTERPRISE' AND user_code = 'CUSTOM-ADMIN' AND is_owner = 1",
            [],
            |row| row.get(0),
        )
        .expect("existing admin owner grant should be queryable");
    assert_eq!(existing_owner_count, 1);

    let _ = fs::remove_file(database_path);
}

fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let sequence = TEST_DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("tron-tests-{test_name}-{nanos}-{sequence}.sqlite3"))
}
