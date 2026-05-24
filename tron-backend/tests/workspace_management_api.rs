mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use serde_json::json;
use tron_backend::config::AppConfig;

use common::{app, app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn list_workspaces_returns_seeded_tree_and_statistics() {
    let router = app();

    let list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/workspaces", None).await;
    assert_eq!(
        list_response.status(),
        StatusCode::OK,
        "AC-WS backend RED: workspace tree route must exist",
    );

    let list_json = response_json(list_response).await;
    assert_eq!(list_json["code"], "SUCCESS");
    let records = list_json["data"]
        .as_array()
        .expect("workspace list data should be an array");
    assert!(
        records
            .iter()
            .any(|item| item["workspaceCode"] == "ENTERPRISE")
    );
    assert!(
        records
            .iter()
            .any(|item| item["workspaceCode"] == "COMPLIANCE")
    );

    let count_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/workspaces/statistics/total-workspaces",
        None,
    )
    .await;
    let count_json = response_json(count_response).await;
    assert_eq!(count_json["data"]["count"], json!(4));
}

#[tokio::test]
async fn list_workspace_options_returns_dropdown_labels_without_fetching_the_full_tree() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/workspaces/options",
        None,
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-WS-12 RED: workspace dropdown options route must exist",
    );

    let payload = response_json(response).await;
    let records = payload["data"]
        .as_array()
        .expect("workspace options should return an array");
    let tron_core = records
        .iter()
        .find(|item| item["workspaceCode"] == "TRON_CORE")
        .expect("workspace options should include active descendant workspaces");
    assert_eq!(
        tron_core["label"],
        json!("Enterprise / Platform / Tron Core"),
    );
    assert_eq!(tron_core["depth"], json!(3));
}

#[tokio::test]
async fn workspace_switcher_returns_root_groups_and_selectable_descendants() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/workspaces/switcher",
        None,
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-WS-13 RED: workspace switcher route must exist",
    );

    let payload = response_json(response).await;
    let primary_spaces = payload["data"]["primarySpaces"]
        .as_array()
        .expect("workspace switcher should return primary spaces");
    assert!(
        primary_spaces
            .iter()
            .any(|item| item["workspaceCode"] == "ENTERPRISE"),
    );
    let secondary_spaces = payload["data"]["secondarySpaces"]
        .as_array()
        .expect("workspace switcher should return secondary spaces");
    let tron_core = secondary_spaces
        .iter()
        .find(|item| item["workspaceCode"] == "TRON_CORE")
        .expect("workspace switcher should include selectable descendants");
    assert_eq!(tron_core["primaryWorkspaceCode"], json!("ENTERPRISE"));
    assert_eq!(
        tron_core["label"],
        json!("Enterprise / Platform / Tron Core")
    );
}

#[tokio::test]
async fn create_workspace_generates_unique_random_workspace_code() {
    let database_url = temp_database_url("workspace-code-generation");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let first_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces",
        Some(json!({
            "name": "Risk",
            "description": "Risk governance workspace"
        })),
    )
    .await;
    assert_eq!(
        first_response.status(),
        StatusCode::CREATED,
        "AC-WS-09 RED: workspace create must not require client-supplied workspaceCode",
    );
    let first_json = response_json(first_response).await;
    let first_code = first_json["data"]["workspaceCode"]
        .as_str()
        .expect("created workspace must return generated workspaceCode");
    assert!(
        first_code.starts_with("WS-"),
        "AC-WS-09 RED: generated workspaceCode should use the server-owned random code namespace",
    );
    assert_ne!(
        first_code, "RISK",
        "AC-WS-09 RED: generated workspaceCode must not be derived from the workspace name",
    );

    let second_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces",
        Some(json!({
            "name": "Risk",
            "description": "Another risk workspace"
        })),
    )
    .await;
    assert_eq!(second_response.status(), StatusCode::CREATED);
    let second_json = response_json(second_response).await;
    let second_code = second_json["data"]["workspaceCode"]
        .as_str()
        .expect("second workspace must return generated workspaceCode");
    assert_ne!(
        second_code, first_code,
        "AC-WS-09 RED: generated workspaceCode must be unique per workspace",
    );
    assert_ne!(
        second_code, "RISK",
        "AC-WS-09 RED: repeated names must still receive non-name workspaceCode values",
    );
}

#[tokio::test]
async fn list_workspaces_include_archived_returns_archived_rows() {
    let database_url = temp_database_url("workspace-include-archived");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let archive_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces/COMPLIANCE/archive",
        None,
    )
    .await;
    assert_eq!(archive_response.status(), StatusCode::OK);

    let active_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/workspaces", None).await;
    let active_json = response_json(active_response).await;
    let active_records = active_json["data"]
        .as_array()
        .expect("default workspace list should return records");
    assert!(
        !active_records
            .iter()
            .any(|item| item["workspaceCode"] == "COMPLIANCE"),
        "default workspace list should hide archived workspaces",
    );

    let all_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/workspaces?includeArchived=true",
        None,
    )
    .await;
    assert_eq!(all_response.status(), StatusCode::OK);
    let all_json = response_json(all_response).await;
    let all_records = all_json["data"]
        .as_array()
        .expect("includeArchived workspace list should return records");
    let compliance = all_records
        .iter()
        .find(|item| item["workspaceCode"] == "COMPLIANCE")
        .expect("includeArchived=true should include archived workspaces");
    assert_eq!(compliance["status"], json!("Archived"));
}

#[tokio::test]
async fn create_child_workspace_rejects_level_four_and_move_rejects_cycle() {
    let database_url = temp_database_url("workspace-hierarchy");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces",
        Some(json!({
            "name": "Level Four",
            "parentWorkspaceCode": "TRON_CORE"
        })),
    )
    .await;
    assert_eq!(
        create_response.status(),
        StatusCode::BAD_REQUEST,
        "AC-WS backend RED: creating level-four workspace must fail",
    );
    let create_json = response_json(create_response).await;
    assert_eq!(
        create_json["code"],
        json!("ADMIN_WORKSPACE_HIERARCHY_INVALID"),
    );

    let move_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces/ENTERPRISE/move",
        Some(json!({
            "parentWorkspaceCode": "TRON_CORE"
        })),
    )
    .await;
    assert_eq!(move_response.status(), StatusCode::BAD_REQUEST);
    let move_json = response_json(move_response).await;
    assert_eq!(
        move_json["code"],
        json!("ADMIN_WORKSPACE_HIERARCHY_INVALID"),
    );
}

#[tokio::test]
async fn replacing_direct_and_role_access_persists_and_effective_users_deduplicate_sources() {
    let database_url = temp_database_url("workspace-access");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config.clone());

    let direct_replace = authenticated_request(
        &router,
        Method::PUT,
        "/api/v1/admin/workspaces/PLATFORM/access/users",
        Some(json!({
            "users": [
                { "userCode": "USR-LARGE-ID", "isOwner": true }
            ]
        })),
    )
    .await;
    assert_eq!(
        direct_replace.status(),
        StatusCode::OK,
        "AC-WS backend RED: direct-user grant replacement endpoint must persist workspace access",
    );

    let role_replace = authenticated_request(
        &router,
        Method::PUT,
        "/api/v1/admin/workspaces/PLATFORM/access/roles",
        Some(json!({
            "roles": [
                { "roleCode": "WORKSPACE_ADMIN" }
            ]
        })),
    )
    .await;
    assert_eq!(
        role_replace.status(),
        StatusCode::OK,
        "AC-WS backend RED: role grant replacement endpoint must persist workspace access",
    );

    let effective_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/workspaces/PLATFORM/access/effective-users",
        None,
    )
    .await;
    assert_eq!(effective_response.status(), StatusCode::OK);
    let effective_json = response_json(effective_response).await;
    let records = effective_json["data"]["records"]
        .as_array()
        .expect("effective access records should be paged");
    let large_id = records
        .iter()
        .find(|item| item["userCode"] == "USR-LARGE-ID")
        .expect(
            "workspace effective users should include the overlapping direct and role-based user",
        );
    assert_eq!(large_id["source"], json!("DirectAndRole"));
    assert_eq!(large_id["isOwner"], json!(true));
    assert_eq!(large_id["matchedRoleCodes"], json!(["WORKSPACE_ADMIN"]));

    let direct_list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/workspaces/PLATFORM/access/users",
        None,
    )
    .await;
    let direct_json = response_json(direct_list).await;
    assert_eq!(direct_json["data"]["total"], json!(1));

    let role_list = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/workspaces/PLATFORM/access/roles",
        None,
    )
    .await;
    let role_json = response_json(role_list).await;
    assert_eq!(role_json["data"][0]["matchedUserCount"], json!(1));
}

#[tokio::test]
async fn archived_parent_blocks_child_restore_after_subtree_archive() {
    let database_url = temp_database_url("workspace-archive");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let archive_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces/ENTERPRISE/archive",
        None,
    )
    .await;
    assert_eq!(
        archive_response.status(),
        StatusCode::OK,
        "AC-WS backend RED: subtree archive endpoint must exist",
    );
    let archive_json = response_json(archive_response).await;
    assert_eq!(archive_json["data"]["affectedWorkspaceCount"], json!(3));

    let restore_child_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces/TRON_CORE/restore",
        None,
    )
    .await;
    assert_eq!(restore_child_response.status(), StatusCode::BAD_REQUEST);
    let restore_child_json = response_json(restore_child_response).await;
    assert_eq!(
        restore_child_json["code"],
        json!("ADMIN_WORKSPACE_ARCHIVED_INVALID"),
    );

    let restore_root_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/workspaces/ENTERPRISE/restore",
        None,
    )
    .await;
    assert_eq!(restore_root_response.status(), StatusCode::OK);
    let restore_root_json = response_json(restore_root_response).await;
    assert_eq!(
        restore_root_json["data"]["affectedWorkspaceCount"],
        json!(3)
    );
    assert_eq!(restore_root_json["data"]["status"], json!("Active"));
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
