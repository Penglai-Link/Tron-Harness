mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode};
use rusqlite::Connection;
use serde_json::json;
use tron_backend::config::AppConfig;

use common::{app, app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn platform_prompt_variable_options_expose_registered_runtime_variables() {
    let router = app();
    // The domain-discovery prompt's inventory variable is registered again, and
    // surviving RUNTIME options such as the terminal `user_request` variable
    // must still be exposed.
    let platform_runtime_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/platform/variables/prompt-options?search=user_request",
        None,
    )
    .await;
    assert_eq!(platform_runtime_response.status(), StatusCode::OK);
    assert!(
        response_json(platform_runtime_response).await["data"]
            .as_array()
            .expect("platform runtime prompt options should be an array")
            .iter()
            .any(|option| {
                option["key"] == json!("user_request")
                    && option["sourceKind"] == json!("RUNTIME")
            })
    );

    let visible_domains_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/platform/variables/prompt-options?search=visible_domains",
        None,
    )
    .await;
    assert_eq!(visible_domains_response.status(), StatusCode::OK);
    let options = response_json(visible_domains_response).await;
    // Domain inventory is a typed context contributor, no longer a template
    // runtime variable. Do not advertise a placeholder that cannot be resolved.
    assert!(options["data"].as_array().unwrap().iter().all(|option| option["key"] != "visible_domains"));
}

#[tokio::test]
async fn list_variables_returns_effective_catalog_with_inherited_and_override_sources() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-VAR backend RED: effective workspace variable route must exist",
    );

    let payload = response_json(response).await;
    assert_eq!(payload["code"], "SUCCESS");
    let records = payload["data"]
        .as_array()
        .expect("effective variable list should return an array");

    let default_region = records
        .iter()
        .find(|item| item["key"] == "DEFAULT_REGION")
        .expect("child workspace should inherit root variables");
    assert_eq!(default_region["value"], json!("us-east-1"));
    assert_eq!(default_region["sourceKind"], json!("Inherited"));
    assert_eq!(default_region["sourceWorkspaceCode"], json!("ENTERPRISE"));

    let timeout = records
        .iter()
        .find(|item| item["key"] == "ROUTER_TIMEOUT_MS")
        .expect("child workspace should expose the effective timeout value");
    assert_eq!(timeout["value"], json!("400"));
    assert_eq!(timeout["sourceKind"], json!("Override"));
    assert_eq!(timeout["sourceWorkspaceCode"], json!("TRON_CORE"));
    assert_eq!(timeout["shadowedWorkspaceCode"], json!("PLATFORM"));
}

#[tokio::test]
async fn platform_variables_are_inherited_by_workspace_catalogs() {
    let database_url = temp_database_url("variable-platform-inheritance");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let platform_create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({
            "key": "GLOBAL_LLM_TIMEOUT_MS",
            "value": "1200",
            "description": "Platform default timeout"
        })),
    )
    .await;
    assert_eq!(platform_create.status(), StatusCode::CREATED);
    let platform_json = response_json(platform_create).await;
    assert_eq!(platform_json["data"]["key"], json!("GLOBAL_LLM_TIMEOUT_MS"));

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    let records = list_json["data"]
        .as_array()
        .expect("workspace variable list should return records");
    let inherited = records
        .iter()
        .find(|item| item["key"] == "GLOBAL_LLM_TIMEOUT_MS")
        .expect("workspace should inherit platform variables");
    assert_eq!(inherited["value"], json!("1200"));
    assert_eq!(inherited["sourceKind"], json!("Inherited"));
    assert_eq!(inherited["sourceLabel"], json!("Platform Configuration"));
    assert_eq!(inherited["sourceWorkspaceCode"], json!("PLATFORM"));

    let override_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/workspaces/TRON_CORE",
        Some(json!({
            "key": "GLOBAL_LLM_TIMEOUT_MS",
            "value": "600",
            "description": "Workspace timeout override"
        })),
    )
    .await;
    assert_eq!(override_response.status(), StatusCode::CREATED);
    let override_json = response_json(override_response).await;
    assert_eq!(override_json["data"]["sourceKind"], json!("Override"));
    assert_eq!(
        override_json["data"]["shadowedLabel"],
        json!("Platform Configuration")
    );

    let delete_response = authenticated_request(
        &router,
        Method::DELETE,
        "/api/v1/variables/workspaces/TRON_CORE/GLOBAL_LLM_TIMEOUT_MS",
        None,
    )
    .await;
    assert_eq!(delete_response.status(), StatusCode::OK);

    let fallback_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    let fallback_json = response_json(fallback_response).await;
    let fallback_records = fallback_json["data"]
        .as_array()
        .expect("workspace variable list should remain readable");
    let fallback = fallback_records
        .iter()
        .find(|item| item["key"] == "GLOBAL_LLM_TIMEOUT_MS")
        .expect("deleting workspace override should reveal platform value");
    assert_eq!(fallback["value"], json!("1200"));
    assert_eq!(fallback["sourceKind"], json!("Inherited"));
    assert_eq!(fallback["sourceLabel"], json!("Platform Configuration"));
}

#[tokio::test]
async fn plain_platform_variable_create_and_list_return_full_display_contract() {
    let database_url = temp_database_url("variable-platform-display-contract");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({
            "key": "GLOBAL_VISIBLE_ENDPOINT",
            "value": "https://api.example.test",
            "description": "Visible platform endpoint",
            "isSensitive": false,
            "valueMode": "REPLACE"
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_json = response_json(create_response).await;
    assert_eq!(create_json["code"], json!("SUCCESS"));

    let created = &create_json["data"];
    assert_eq!(created["key"], json!("GLOBAL_VISIBLE_ENDPOINT"));
    assert_eq!(created["value"], json!("https://api.example.test"));
    assert_eq!(created["description"], json!("Visible platform endpoint"));
    assert_eq!(created["isSensitive"], json!(false));
    assert_eq!(created["valueMasked"], json!(false));
    assert_eq!(created["hasValue"], json!(true));
    let created_id = created["id"]
        .as_str()
        .expect("platform variable id should serialize as a string");
    assert!(
        created_id.parse::<i64>().is_ok(),
        "platform variable id should use the SnowflakeId decimal string contract"
    );
    let created_at = created["createdAt"]
        .as_str()
        .expect("platform variable create response should include createdAt");
    assert!(
        !created_at.is_empty(),
        "platform variable createdAt should not be empty"
    );
    assert!(
        created["updatedAt"].is_string(),
        "platform variable create response should include updatedAt"
    );

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/platform/variables",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    assert_eq!(list_json["code"], json!("SUCCESS"));
    let records = list_json["data"]
        .as_array()
        .expect("platform variable list should return records");
    let listed = records
        .iter()
        .find(|item| item["key"] == "GLOBAL_VISIBLE_ENDPOINT")
        .expect("created platform variable should be listed");
    for expected_field in [
        "id",
        "key",
        "value",
        "description",
        "isSensitive",
        "valueMasked",
        "hasValue",
        "createdAt",
        "updatedAt",
    ] {
        assert!(
            listed.get(expected_field).is_some(),
            "platform variable list record should include {expected_field}"
        );
    }
    assert_eq!(listed["id"], json!(created_id));
    assert_eq!(listed["createdAt"], json!(created_at));
    assert_eq!(listed["value"], json!("https://api.example.test"));
    assert_eq!(listed["isSensitive"], json!(false));
    assert_eq!(listed["valueMasked"], json!(false));
    assert_eq!(listed["hasValue"], json!(true));
}

#[tokio::test]
async fn create_override_and_delete_falls_back_to_inherited_value() {
    let database_url = temp_database_url("variable-override-fallback");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/workspaces/TRON_CORE",
        Some(json!({
            "key": "DEFAULT_REGION",
            "value": "ap-southeast-1",
            "description": "Workspace override"
        })),
    )
    .await;
    assert_eq!(
        create_response.status(),
        StatusCode::CREATED,
        "AC-VAR backend RED: creating a local override should succeed",
    );
    let create_json = response_json(create_response).await;
    assert_eq!(create_json["data"]["sourceKind"], json!("Override"));
    assert_eq!(
        create_json["data"]["sourceWorkspaceCode"],
        json!("TRON_CORE")
    );

    let delete_response = authenticated_request(
        &router,
        Method::DELETE,
        "/api/v1/variables/workspaces/TRON_CORE/DEFAULT_REGION",
        None,
    )
    .await;
    assert_eq!(delete_response.status(), StatusCode::OK);

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    let list_json = response_json(list_response).await;
    let records = list_json["data"]
        .as_array()
        .expect("variable list should remain readable after delete");
    let default_region = records
        .iter()
        .find(|item| item["key"] == "DEFAULT_REGION")
        .expect("deleting the override should reveal the inherited ancestor value");
    assert_eq!(default_region["value"], json!("us-east-1"));
    assert_eq!(default_region["sourceKind"], json!("Inherited"));
    assert_eq!(default_region["sourceWorkspaceCode"], json!("ENTERPRISE"));
}

#[tokio::test]
async fn update_local_variable_persists_new_value() {
    let database_url = temp_database_url("variable-update-local");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/variables/workspaces/TRON_CORE/OPENAI_API_KEY",
        Some(json!({
            "value": "sk-tron-core-rotated",
            "description": "Rotated key"
        })),
    )
    .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "AC-VAR backend RED: local workspace variables must be editable",
    );
    let payload = response_json(response).await;
    assert_eq!(payload["data"]["value"], json!("sk-tron-core-rotated"));
    assert_eq!(payload["data"]["sourceKind"], json!("Override"));
}

#[tokio::test]
async fn duplicate_local_keys_and_inherited_updates_are_rejected() {
    let router = app();

    let duplicate_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/workspaces/TRON_CORE",
        Some(json!({
            "key": "OPENAI_API_KEY",
            "value": "duplicate"
        })),
    )
    .await;
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);
    let duplicate_json = response_json(duplicate_response).await;
    assert_eq!(duplicate_json["code"], json!("VARIABLE_KEY_DUPLICATE"));

    let inherited_update_response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/variables/workspaces/TRON_CORE/DEFAULT_REGION",
        Some(json!({
            "value": "eu-central-1"
        })),
    )
    .await;
    assert_eq!(inherited_update_response.status(), StatusCode::BAD_REQUEST);
    let inherited_update_json = response_json(inherited_update_response).await;
    assert_eq!(
        inherited_update_json["code"],
        json!("VARIABLE_INHERITED_MUTATION_INVALID"),
    );
}

#[tokio::test]
async fn sensitive_writes_store_ciphertext_and_display_masks_without_plaintext_search() {
    let database_url = temp_database_url("variable-sensitive-storage-display");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let sensitive_value = sensitive_test_value("storage-display");

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({
            "key": "VSD_STORAGE_TOKEN",
            "value": sensitive_value,
            "description": "Sensitive storage token",
            "isSensitive": true,
            "valueMode": "REPLACE"
        })),
    )
    .await;
    assert_eq!(
        create_response.status(),
        StatusCode::CREATED,
        "AC-VSD-02 RED: sensitive platform variable create should be accepted"
    );
    let create_json = response_json(create_response).await;
    assert_sensitive_display_shape(&create_json["data"]);
    assert!(
        create_json["data"].get("valueNonce").is_none(),
        "AC-VSD-03 RED: display payload must not expose crypto metadata"
    );

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    let inherited = find_record(&list_json, "VSD_STORAGE_TOKEN");
    assert_sensitive_display_shape(inherited);

    let search_response = authenticated_request(
        &router,
        Method::GET,
        &format!(
            "/api/v1/variables/workspaces/TRON_CORE?search={}",
            sensitive_value
        ),
        None,
    )
    .await;
    assert_eq!(search_response.status(), StatusCode::OK);
    let search_json = response_json(search_response).await;
    assert!(
        !contains_record(&search_json, "VSD_STORAGE_TOKEN"),
        "AC-VSD-03 RED: sensitive plaintext must not be searchable"
    );

    let connection = Connection::open(&database_url).expect("variable database should reopen");
    let stored = connection
        .query_row(
            "SELECT variable_value, is_sensitive, value_nonce, value_tag, value_algorithm, value_key_version
             FROM platform_variables WHERE variable_key = 'VSD_STORAGE_TOKEN'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .expect("sensitive platform variable storage row should exist");
    assert!(
        stored.0 != sensitive_value,
        "AC-VSD-02 RED: stored sensitive payload must not equal submitted input"
    );
    assert_eq!(stored.1, 1);
    assert!(stored.2.is_some());
    assert!(stored.3.is_some());
    assert_eq!(stored.4.as_deref(), Some("SM4_GCM"));
    assert!(stored.5.is_some());
}

#[tokio::test]
async fn sensitive_edit_rules_preserve_replace_demote_and_inherited_override_safely() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("variable-sensitive-edit")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let sensitive_value = sensitive_test_value("edit");
    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({
            "key": "VSD_EDIT_TOKEN",
            "value": sensitive_value,
            "description": "Sensitive edit token",
            "isSensitive": true,
            "valueMode": "REPLACE"
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);

    let preserve_response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/admin/platform/variables/VSD_EDIT_TOKEN",
        Some(json!({
            "description": "Description-only update",
            "isSensitive": true,
            "valueMode": "PRESERVE"
        })),
    )
    .await;
    assert_eq!(
        preserve_response.status(),
        StatusCode::OK,
        "AC-VSD-04 RED: preserving a sensitive value should allow metadata-only edits"
    );
    let preserve_json = response_json(preserve_response).await;
    assert_sensitive_display_shape(&preserve_json["data"]);
    assert_eq!(
        preserve_json["data"]["description"],
        json!("Description-only update")
    );

    let replace_response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/admin/platform/variables/VSD_EDIT_TOKEN",
        Some(json!({
            "value": sensitive_test_value("edit-replacement"),
            "description": "Replacement update",
            "isSensitive": true,
            "valueMode": "REPLACE"
        })),
    )
    .await;
    assert_eq!(
        replace_response.status(),
        StatusCode::OK,
        "AC-VSD-04 RED: replacing a sensitive value should write a new encrypted payload"
    );

    let demote_without_value = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/admin/platform/variables/VSD_EDIT_TOKEN",
        Some(json!({
            "description": "Invalid demotion",
            "isSensitive": false,
            "valueMode": "PRESERVE"
        })),
    )
    .await;
    assert_eq!(demote_without_value.status(), StatusCode::BAD_REQUEST);
    let demote_error = response_json(demote_without_value).await;
    assert_eq!(
        demote_error["code"],
        json!("VARIABLE_SENSITIVE_DOWNGRADE_VALUE_REQUIRED")
    );

    let override_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/workspaces/TRON_CORE",
        Some(json!({
            "key": "VSD_EDIT_TOKEN",
            "value": "non-sensitive override",
            "description": "Unsafe override",
            "isSensitive": false,
            "valueMode": "REPLACE"
        })),
    )
    .await;
    assert_eq!(override_response.status(), StatusCode::BAD_REQUEST);
    let override_error = response_json(override_response).await;
    assert_eq!(
        override_error["code"],
        json!("VARIABLE_SENSITIVE_OVERRIDE_REQUIRED")
    );

    let non_sensitive_create = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({
            "key": "VSD_UPGRADE_TOKEN",
            "value": "plain-upgrade-candidate",
            "description": "Plain token",
            "isSensitive": false,
            "valueMode": "REPLACE"
        })),
    )
    .await;
    assert_eq!(non_sensitive_create.status(), StatusCode::CREATED);

    let upgrade_without_value = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/admin/platform/variables/VSD_UPGRADE_TOKEN",
        Some(json!({
            "description": "Invalid sensitive upgrade",
            "isSensitive": true,
            "valueMode": "PRESERVE"
        })),
    )
    .await;
    assert_eq!(upgrade_without_value.status(), StatusCode::BAD_REQUEST);
    let upgrade_error = response_json(upgrade_without_value).await;
    assert_eq!(
        upgrade_error["code"],
        json!("VARIABLE_SENSITIVE_UPGRADE_VALUE_REQUIRED"),
        "AC-VSD-04 RED: non-sensitive plaintext must not be copied into sensitive storage"
    );
}

#[tokio::test]
async fn active_profile_reveals_one_sensitive_value_without_creating_login_cookies() {
    let router = app();
    let secret = sensitive_test_value("profile-reveal");
    let created = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables",
        Some(json!({
            "key":"PROFILE_SECRET", "value":secret, "isSensitive":true, "valueMode":"REPLACE"
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables/reveal-platform-variable",
        Some(json!({"variableKey":"PROFILE_SECRET"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get(axum::http::header::SET_COOKIE)
            .is_none()
    );
    assert_eq!(
        response_json(response).await["data"],
        json!({"value":secret})
    );
    // Explicit reveal does not alter the masked list contract.
    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/platform/variables",
        None,
    )
    .await;
    assert!(!response_json(response).await.to_string().contains(&secret));
}

#[tokio::test]
async fn sensitive_reveal_uses_the_exact_platform_or_workspace_target() {
    let router = app();
    for (endpoint, value) in [
        ("/api/v1/admin/platform/variables", "platform-fixture"),
        (
            "/api/v1/variables/workspaces/TRON_CORE",
            "workspace-fixture",
        ),
    ] {
        let response = authenticated_request(
            &router,
            Method::POST,
            endpoint,
            Some(json!({"key":"SAME_KEY", "value":value,"isSensitive":true,"valueMode":"REPLACE"})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    let platform = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/variables/reveal-platform-variable",
        Some(json!({"variableKey":"SAME_KEY"})),
    )
    .await;
    assert_eq!(
        response_json(platform).await["data"]["value"],
        "platform-fixture"
    );
    let workspace = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/reveal-workspace-variable",
        Some(json!({"workspaceCode":"TRON_CORE","variableKey":"SAME_KEY"})),
    )
    .await;
    assert_eq!(
        response_json(workspace).await["data"]["value"],
        "workspace-fixture"
    );
    let missing = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/variables/reveal-workspace-variable",
        Some(json!({"workspaceCode":"TRON_CORE","variableKey":"MISSING_KEY"})),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn non_sensitive_variable_regression_remains_visible_and_searchable() {
    let router = app_with_config(AppConfig {
        database_url: temp_database_url("variable-non-sensitive-regression")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE?search=us-east-1",
        None,
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_json = response_json(list_response).await;
    let default_region = find_record(&list_json, "DEFAULT_REGION");
    assert_eq!(default_region["value"], json!("us-east-1"));
    assert_eq!(default_region["isSensitive"], json!(false));
    assert_eq!(default_region["valueMasked"], json!(false));
    assert_eq!(default_region["hasValue"], json!(true));

    let update_response = authenticated_request(
        &router,
        Method::PATCH,
        "/api/v1/variables/workspaces/TRON_CORE/ROUTER_TIMEOUT_MS",
        Some(json!({
            "value": "450",
            "description": "Non-sensitive regression update",
            "isSensitive": false,
            "valueMode": "REPLACE"
        })),
    )
    .await;
    assert_eq!(
        update_response.status(),
        StatusCode::OK,
        "AC-VSD-10 RED: existing non-sensitive mutations should keep working"
    );
    let update_json = response_json(update_response).await;
    assert_eq!(update_json["data"]["value"], json!("450"));
    assert_eq!(update_json["data"]["isSensitive"], json!(false));
}

fn sensitive_test_value(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    format!("vsd-{label}-{nanos}")
}

fn assert_sensitive_display_shape(record: &serde_json::Value) {
    assert_eq!(record["value"], json!("********"));
    assert_eq!(record["isSensitive"], json!(true));
    assert_eq!(record["valueMasked"], json!(true));
    assert_eq!(record["hasValue"], json!(true));
}

fn find_record<'a>(payload: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    payload["data"]
        .as_array()
        .expect("variable list should return records")
        .iter()
        .find(|item| item["key"] == key)
        .expect("expected variable record should be present")
}

fn contains_record(payload: &serde_json::Value, key: &str) -> bool {
    payload["data"]
        .as_array()
        .expect("variable list should return records")
        .iter()
        .any(|item| item["key"] == key)
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}
