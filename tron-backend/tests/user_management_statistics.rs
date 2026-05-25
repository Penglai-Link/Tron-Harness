use axum::{
    body::to_bytes,
    http::{Method, StatusCode},
};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tron_backend::{app::build_router, config::AppConfig};

mod common;

use common::authenticated_request_with_config;

#[tokio::test]
async fn user_management_statistics_total_users_endpoint_returns_count_envelope() {
    assert_count_endpoint(
        "total-users",
        "/api/v1/admin/users/statistics/total-users",
        "SELECT COUNT(*) FROM users",
    )
    .await;
}

#[tokio::test]
async fn user_management_statistics_active_users_endpoint_returns_count_envelope() {
    assert_count_endpoint(
        "active-users",
        "/api/v1/admin/users/statistics/active-users",
        "SELECT COUNT(*) FROM users WHERE status = 'active'",
    )
    .await;
}

#[tokio::test]
async fn user_management_statistics_role_assignments_endpoint_returns_count_envelope() {
    assert_count_endpoint(
        "role-assignments",
        "/api/v1/admin/users/statistics/role-assignments",
        "SELECT COUNT(*) FROM user_role_assignments",
    )
    .await;
}

#[tokio::test]
async fn user_management_statistics_space_assignments_endpoint_returns_count_envelope() {
    assert_count_endpoint(
        "space-assignments",
        "/api/v1/admin/users/statistics/space-assignments",
        "SELECT COUNT(*) FROM user_space_assignments",
    )
    .await;
}

#[tokio::test]
async fn user_management_statistics_default_list_excludes_archived_users() {
    let fixture = TestFixture::new("default-active-list");
    let response = request(
        fixture.config.clone(),
        Method::GET,
        "/api/v1/admin/users?current=1&size=100",
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-15 RED: list endpoint must stay available before list/statistic consistency can be verified",
    );

    let json = response_json(response).await;
    assert_paged_success_envelope(&json);
    assert_eq!(
        page_total(&json),
        fixture.count("SELECT COUNT(*) FROM users WHERE status = 'active'"),
        "default list total must equal the active-user database count before pagination",
    );

    let records = page_records(&json);
    assert!(
        !records.is_empty(),
        "deterministic seed data must include active users",
    );
    assert!(
        records.iter().all(|record| record["status"] == "Active"),
        "default list must exclude archived users",
    );

    fixture.cleanup();
}

#[tokio::test]
async fn user_management_statistics_archived_status_search_returns_real_archived_users_only() {
    let fixture = TestFixture::new("archived-status-search");
    let response = request(
        fixture.config.clone(),
        Method::GET,
        "/api/v1/admin/users?status=Archived&search=archived&current=1&size=100",
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-15 RED: status=Archived search must be backed by persisted archived users",
    );

    let json = response_json(response).await;
    assert_paged_success_envelope(&json);
    assert_eq!(
        page_total(&json),
        fixture.count(
            "SELECT COUNT(*) FROM users
             WHERE status = 'archived'
               AND (
                 lower(user_code) LIKE '%archived%'
                 OR lower(username) LIKE '%archived%'
                 OR lower(display_name) LIKE '%archived%'
                 OR lower(email) LIKE '%archived%'
               )",
        ),
        "status=Archived plus search total must equal archived filtered database count",
    );

    let records = page_records(&json);
    assert!(
        !records.is_empty(),
        "deterministic seed data must include searchable archived users",
    );
    assert!(
        records.iter().all(|record| record["status"] == "Archived"),
        "status=Archived must return real archived users only",
    );

    fixture.cleanup();
}

#[tokio::test]
async fn user_management_statistics_include_archived_returns_all_statuses() {
    let fixture = TestFixture::new("include-archived-list");
    let response = request(
        fixture.config.clone(),
        Method::GET,
        "/api/v1/admin/users?includeArchived=true&current=1&size=100",
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-15 RED: includeArchived=true must be backed by all persisted statuses",
    );

    let json = response_json(response).await;
    assert_paged_success_envelope(&json);
    assert_eq!(
        page_total(&json),
        fixture.count("SELECT COUNT(*) FROM users"),
        "includeArchived=true total must equal all persisted users",
    );

    let records = page_records(&json);
    assert!(
        records.iter().any(|record| record["status"] == "Active"),
        "includeArchived=true must include active users",
    );
    assert!(
        records.iter().any(|record| record["status"] == "Archived"),
        "includeArchived=true must include archived users",
    );

    fixture.cleanup();
}

#[tokio::test]
async fn user_management_statistics_pagination_total_uses_filtered_database_count() {
    let fixture = TestFixture::new("pagination-total");
    let response = request(
        fixture.config.clone(),
        Method::GET,
        "/api/v1/admin/users?current=1&size=2",
    )
    .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-15 RED: paged list endpoint must be available for total-count consistency",
    );

    let json = response_json(response).await;
    assert_paged_success_envelope(&json);

    let records = page_records(&json);
    let active_count = fixture.count("SELECT COUNT(*) FROM users WHERE status = 'active'");
    assert_eq!(
        records.len(),
        2,
        "fixture must contain at least two active users for a deterministic first page",
    );
    assert!(
        page_total(&json) > records.len() as i64,
        "page total must be the filtered database count, not the current page length",
    );
    assert_eq!(
        page_total(&json),
        active_count,
        "default paged total must equal active users before pagination",
    );

    fixture.cleanup();
}

#[tokio::test]
async fn user_management_statistics_seed_data_supports_pagination_and_assignment_counts() {
    let fixture = TestFixture::new("seed-size");

    let total_users = fixture.count("SELECT COUNT(*) FROM users");
    let active_users = fixture.count("SELECT COUNT(*) FROM users WHERE status = 'active'");
    let archived_users = fixture.count("SELECT COUNT(*) FROM users WHERE status = 'archived'");
    let role_assignments = fixture.count("SELECT COUNT(*) FROM user_role_assignments");
    let space_assignments = fixture.count("SELECT COUNT(*) FROM user_space_assignments");

    assert!(
        active_users > 2,
        "deterministic seed data must contain more active users than a small page size",
    );
    assert!(
        archived_users >= 2,
        "deterministic seed data must include multiple archived users",
    );
    assert_eq!(
        total_users,
        active_users + archived_users,
        "seed users must use only Active and Archived statuses",
    );
    assert!(
        role_assignments > total_users && space_assignments > total_users,
        "role ({role_assignments}) and space ({space_assignments}) assignment seed rows must both be larger than total users ({total_users}) to prove assignment counts are independent",
    );

    fixture.cleanup();
}

async fn assert_count_endpoint(statistic_name: &str, uri: &str, expected_sql: &str) {
    let fixture = TestFixture::new(statistic_name);
    let expected_count = fixture.count(expected_sql);

    let response = request(fixture.config.clone(), Method::GET, uri).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "TDD-UM-14 RED: statistic endpoint `{uri}` must exist and return HTTP 200",
    );

    let json = response_json(response).await;
    assert_eq!(
        json["code"], "SUCCESS",
        "statistic endpoint `{uri}` must use the uniform success code",
    );
    assert_eq!(
        json["message"], "ok",
        "statistic endpoint `{uri}` must use the uniform success message",
    );
    assert!(
        json["data"].is_object(),
        "statistic endpoint `{uri}` must return an object in data",
    );
    assert_eq!(
        json["data"]["count"]
            .as_i64()
            .expect("data.count must be a JSON integer"),
        expected_count,
        "statistic endpoint `{uri}` must return canonical data.count from persisted repository state",
    );

    fixture.cleanup();
}

async fn request(config: AppConfig, method: Method, uri: &str) -> axum::response::Response {
    authenticated_request_with_config(config, method, uri, None).await
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response body should be valid JSON")
}

fn assert_paged_success_envelope(json: &Value) {
    assert_eq!(json["code"], "SUCCESS");
    assert_eq!(json["message"], "ok");
    assert!(json["data"].is_object(), "data must be an object");
    assert!(
        json["data"]["current"].is_number(),
        "data.current is required"
    );
    assert!(json["data"]["size"].is_number(), "data.size is required");
    assert!(json["data"]["total"].is_number(), "data.total is required");
    assert!(
        json["data"]["records"].is_array(),
        "data.records is required"
    );
}

fn page_total(json: &Value) -> i64 {
    json["data"]["total"]
        .as_i64()
        .expect("data.total must be a JSON integer")
}

fn page_records(json: &Value) -> &[Value] {
    json["data"]["records"]
        .as_array()
        .expect("data.records must be a JSON array")
}

struct TestFixture {
    config: AppConfig,
    database_path: PathBuf,
}

impl TestFixture {
    fn new(test_name: &str) -> Self {
        let database_path = temp_database_path(test_name);
        let config = common::prepared_config(AppConfig {
            database_url: database_path.to_string_lossy().into_owned(),
            ..AppConfig::default()
        });

        let _router = build_router(config.clone());

        Self {
            config,
            database_path,
        }
    }

    fn count(&self, sql: &str) -> i64 {
        let connection =
            Connection::open(&self.database_path).expect("SQLite fixture database should open");
        connection
            .query_row(sql, [], |row| row.get(0))
            .expect("fixture count query should succeed")
    }

    fn cleanup(self) {
        let _ = fs::remove_file(self.database_path);
    }
}

fn temp_database_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
