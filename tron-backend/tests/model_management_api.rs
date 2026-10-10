mod common;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    body::to_bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::IntoResponse,
    routing::post,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tron_backend::config::AppConfig;

use common::{app, app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn workspace_catalog_returns_local_and_inherited_models_without_non_inheritable_ancestors() {
    let database_url = temp_database_url("model-workspace-inherited-catalog");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let shared_parent_model_code = create_workspace_model(
        &router,
        "PLATFORM",
        "Platform Shared Writer",
        "gpt-4.1-mini",
        "RESPONSES",
        "TEXT",
    )
    .await;
    let hidden_parent_model_code = create_workspace_model(
        &router,
        "PLATFORM",
        "Platform Private Image",
        "gpt-image-1",
        "RESPONSES",
        "IMAGE",
    )
    .await;
    set_workspace_model_allow_inheritance(&router, "PLATFORM", &hidden_parent_model_code, false)
        .await;
    let local_model_code = create_workspace_model(
        &router,
        "TRON_CORE",
        "Core Fast",
        "gpt-4.1-mini",
        "OPENAI_COMPATIBLE",
        "TEXT",
    )
    .await;

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(
        response.status(),
        axum::http::StatusCode::OK,
        "AC-MODEL-01 RED: workspace model catalog route must exist",
    );

    let payload = response_json(response).await;
    let records = payload["data"]["records"]
        .as_array()
        .expect("workspace catalog should return record array");

    let inherited = records
        .iter()
        .find(|item| item["modelCode"] == json!(shared_parent_model_code))
        .expect("TRON_CORE should inherit allowInheritance workspace-local model");
    assert_eq!(inherited["sourceKind"], json!("INHERITED"));
    assert_eq!(inherited["sourceWorkspaceCode"], json!("PLATFORM"));
    assert_eq!(
        inherited["sourceWorkspacePath"],
        json!([
            { "workspaceCode": "ENTERPRISE", "name": "Enterprise" },
            { "workspaceCode": "PLATFORM", "name": "Platform" }
        ])
    );
    assert_eq!(inherited["sourceLabel"], json!("Inherited"));

    assert!(
        records
            .iter()
            .all(|item| item["modelCode"] != json!(hidden_parent_model_code)),
        "non-inheritable ancestor models must stay hidden from descendants",
    );

    let local = records
        .iter()
        .find(|item| item["modelCode"] == json!(local_model_code))
        .expect("workspace-local model should be visible in current workspace");
    assert_eq!(local["sourceKind"], json!("LOCAL"));
    assert_eq!(local["allowInheritance"], json!(true));
}

#[tokio::test]
async fn duplicate_display_name_is_rejected_per_scope_but_same_model_name_is_allowed() {
    let database_url = temp_database_url("model-duplicate-scope");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let platform_first = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(model_payload(
            "Platform Unique",
            "gpt-4.1",
            "RESPONSES",
            "TEXT",
        )),
    )
    .await;
    assert_eq!(platform_first.status(), axum::http::StatusCode::CREATED);

    let platform_duplicate = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(model_payload(
            "Platform Unique",
            "gpt-4.1-mini",
            "RESPONSES",
            "TEXT",
        )),
    )
    .await;
    assert_eq!(
        platform_duplicate.status(),
        axum::http::StatusCode::CONFLICT
    );
    let platform_duplicate_json = response_json(platform_duplicate).await;
    assert_eq!(
        platform_duplicate_json["message"],
        json!("displayName already exists in platform models"),
    );

    let workspace_first = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE",
        Some(model_payload(
            "Workspace Alpha",
            "shared-remote",
            "RESPONSES",
            "TEXT",
        )),
    )
    .await;
    assert_eq!(workspace_first.status(), axum::http::StatusCode::CREATED);
    let workspace_second = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE",
        Some(model_payload(
            "Workspace Beta",
            "shared-remote",
            "RESPONSES",
            "TEXT",
        )),
    )
    .await;
    assert_eq!(
        workspace_second.status(),
        axum::http::StatusCode::CREATED,
        "same modelName should be allowed when displayName differs",
    );

    let workspace_duplicate = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE",
        Some(model_payload(
            "Workspace Alpha",
            "other-remote",
            "RESPONSES",
            "TEXT",
        )),
    )
    .await;
    assert_eq!(
        workspace_duplicate.status(),
        axum::http::StatusCode::CONFLICT
    );
    let workspace_duplicate_json = response_json(workspace_duplicate).await;
    assert_eq!(
        workspace_duplicate_json["message"],
        json!("displayName already exists in workspace TRON_CORE"),
    );

    let compliance_same_name = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/COMPLIANCE",
        Some(model_payload(
            "Workspace Alpha",
            "other-remote",
            "RESPONSES",
            "TEXT",
        )),
    )
    .await;
    assert_eq!(
        compliance_same_name.status(),
        axum::http::StatusCode::CREATED,
        "same displayName in different workspaces should stay valid",
    );
}

#[tokio::test]
async fn model_capabilities_round_trip_as_multi_value_field() {
    let database_url = temp_database_url("model-capabilities-round-trip");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Reasoning Voice",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION", "DEEP_THINKING", "SPEECH_SYNTHESIS"],
            "baseUrl": "https://api.example.com",
            "apiKeyRef": "sk-test-model",
            "modelName": "gpt-4.1",
            "enabled": true,
            "thinkingEnabled": true,
            "structuredOutputEnabled": true,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_payload = response_json(create_response).await;
    assert_eq!(
        create_payload["data"]["modelCapabilities"],
        json!(["TEXT_GENERATION", "DEEP_THINKING", "SPEECH_SYNTHESIS"]),
    );

    let model_code = create_payload["data"]["modelCode"]
        .as_str()
        .expect("created model should return modelCode");

    let update_response = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/admin/platform/models/{model_code}"),
        Some(json!({
            "displayName": "Reasoning Voice",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION", "REALTIME_SPEECH_TRANSLATION"],
            "baseUrl": "https://api.example.com",
            "apiKeyRef": "*****",
            "modelName": "gpt-4.1",
            "enabled": true,
            "thinkingEnabled": true,
            "structuredOutputEnabled": true,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_payload = response_json(update_response).await;
    assert_eq!(
        update_payload["data"]["modelCapabilities"],
        json!(["TEXT_GENERATION", "REALTIME_SPEECH_TRANSLATION"]),
    );
}

#[tokio::test]
async fn reasoning_efforts_round_trip_per_model_and_reject_an_invalid_default() {
    let database_url = temp_database_url("model-reasoning-effort-round-trip");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let create_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Gateway Ultra Model",
            "vendor": "Custom Gateway",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION", "DEEP_THINKING"],
            "baseUrl": "https://gateway.example.com/v1",
            "apiKeyRef": "sk-test-model",
            "modelName": "gateway-ultra",
            "enabled": true,
            "thinkingEnabled": true,
            "supportedReasoningEfforts": ["Ultra", "Low", "None", "High", "Low"],
            "defaultReasoningEffort": "Ultra",
            "structuredOutputEnabled": true,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let create_payload = response_json(create_response).await;
    assert_eq!(
        create_payload["data"]["supportedReasoningEfforts"],
        json!(["None", "Low", "High", "Ultra"]),
    );
    assert_eq!(
        create_payload["data"]["defaultReasoningEffort"],
        json!("Ultra"),
    );

    let model_code = create_payload["data"]["modelCode"]
        .as_str()
        .expect("created model should return modelCode");
    let get_response = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/admin/platform/models/{model_code}"),
        None,
    )
    .await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_payload = response_json(get_response).await;
    assert_eq!(
        get_payload["data"]["supportedReasoningEfforts"],
        json!(["None", "Low", "High", "Ultra"]),
    );
    assert_eq!(
        get_payload["data"]["defaultReasoningEffort"],
        json!("Ultra")
    );

    let invalid_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Invalid Reasoning Default",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": "https://api.example.com/v1",
            "apiKeyRef": "sk-test-model",
            "modelName": "gpt-test",
            "enabled": true,
            "thinkingEnabled": true,
            "supportedReasoningEfforts": ["Low", "Medium"],
            "defaultReasoningEffort": "High",
            "structuredOutputEnabled": true,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);
    let invalid_payload = response_json(invalid_response).await;
    assert_eq!(
        invalid_payload["code"],
        json!("MODEL_DEFAULT_REASONING_EFFORT_UNSUPPORTED"),
    );
}

#[tokio::test]
async fn context_window_tokens_round_trip_for_platform_and_workspace_models() {
    let database_url = temp_database_url("model-context-window-round-trip");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let mut platform_payload =
        model_payload("Context Platform", "qwen-max", "OPENAI_COMPATIBLE", "TEXT");
    platform_payload["contextWindowTokens"] = json!(65536);
    let platform_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(platform_payload),
    )
    .await;
    assert_eq!(platform_response.status(), StatusCode::CREATED);
    let platform_body = response_json(platform_response).await;
    assert_eq!(platform_body["data"]["contextWindowTokens"], json!(65536));

    let mut workspace_payload = model_payload(
        "Context Workspace",
        "qwen-fast",
        "OPENAI_COMPATIBLE",
        "TEXT",
    );
    workspace_payload["contextWindowTokens"] = json!(32768);
    let workspace_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE",
        Some(workspace_payload),
    )
    .await;
    assert_eq!(workspace_response.status(), StatusCode::CREATED);
    let workspace_body = response_json(workspace_response).await;
    assert_eq!(workspace_body["data"]["contextWindowTokens"], json!(32768));
}

#[tokio::test]
async fn duplicate_platform_model_clones_config_with_incrementing_copy_names() {
    let server = verify_mock_server().await;
    let database_url = temp_database_url("model-platform-duplicate");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let source_model_code = create_platform_model_for_verify_with_secret(
        &router,
        &server.base_url,
        "Qwen3.7-max",
        "qwen3.7-max",
        "Alibaba Token Plan",
        "RESPONSES",
        "TEXT",
        "sk-platform-duplicate-secret",
    )
    .await;

    let first_duplicate_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{source_model_code}/duplicate"),
        None,
    )
    .await;
    assert_eq!(first_duplicate_response.status(), StatusCode::CREATED);
    let first_duplicate_payload = response_json(first_duplicate_response).await;
    let first_duplicate_code = first_duplicate_payload["data"]["modelCode"]
        .as_str()
        .expect("duplicate route should return modelCode")
        .to_string();
    assert_eq!(
        first_duplicate_payload["data"]["displayName"],
        json!("Qwen3.7-max-Copy")
    );
    assert_eq!(
        first_duplicate_payload["data"]["modelName"],
        json!("qwen3.7-max")
    );
    assert_eq!(first_duplicate_payload["data"]["apiKeyRef"], json!("*****"));
    assert_eq!(
        first_duplicate_payload["data"]["lastVerifyStatus"],
        Value::Null
    );

    let second_duplicate_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{source_model_code}/duplicate"),
        None,
    )
    .await;
    assert_eq!(second_duplicate_response.status(), StatusCode::CREATED);
    let second_duplicate_payload = response_json(second_duplicate_response).await;
    assert_eq!(
        second_duplicate_payload["data"]["displayName"],
        json!("Qwen3.7-max-Copy-2")
    );

    verify_platform_model_for_test(&router, &first_duplicate_code).await;

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex should stay available")
        .clone();
    let duplicate_capture = captures
        .iter()
        .find(|capture| capture.path == "/responses")
        .expect("duplicated model verify should call the responses endpoint");
    assert_eq!(
        duplicate_capture.authorization.as_deref(),
        Some("Bearer sk-platform-duplicate-secret")
    );

    server.task.abort();
}

#[tokio::test]
async fn duplicate_workspace_model_clones_visible_platform_source_into_local_workspace_copy() {
    let database_url = temp_database_url("model-workspace-duplicate");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let source_model_code =
        create_platform_model(&router, "GPT-4o Main", "gpt-4o", "RESPONSES", "TEXT").await;

    let first_duplicate_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{source_model_code}/duplicate"),
        None,
    )
    .await;
    assert_eq!(first_duplicate_response.status(), StatusCode::CREATED);
    let first_duplicate_payload = response_json(first_duplicate_response).await;
    let first_duplicate_code = first_duplicate_payload["data"]["modelCode"]
        .as_str()
        .expect("workspace duplicate should return modelCode")
        .to_string();
    assert_eq!(
        first_duplicate_payload["data"]["displayName"],
        json!("GPT-4o Main-Copy")
    );
    assert_eq!(
        first_duplicate_payload["data"]["sourceKind"],
        json!("LOCAL")
    );
    assert_eq!(
        first_duplicate_payload["data"]["workspaceCode"],
        json!("TRON_CORE")
    );
    assert_eq!(first_duplicate_payload["data"]["apiKeyRef"], json!("*****"));
    assert_eq!(
        first_duplicate_payload["data"]["allowInheritance"],
        json!(true)
    );
    assert_eq!(
        first_duplicate_payload["data"]["lastVerifyStatus"],
        Value::Null
    );

    let second_duplicate_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{source_model_code}/duplicate"),
        None,
    )
    .await;
    assert_eq!(second_duplicate_response.status(), StatusCode::CREATED);
    let second_duplicate_payload = response_json(second_duplicate_response).await;
    assert_eq!(
        second_duplicate_payload["data"]["displayName"],
        json!("GPT-4o Main-Copy-2")
    );

    let catalog_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_json = response_json(catalog_response).await;
    let catalog_records = catalog_json["data"]["records"]
        .as_array()
        .expect("workspace catalog should return records");
    let workspace_copy = catalog_records
        .iter()
        .find(|record| record["modelCode"] == json!(first_duplicate_code))
        .expect("workspace duplicate should appear in the current workspace catalog");
    assert_eq!(workspace_copy["sourceKind"], json!("LOCAL"));
    assert_eq!(workspace_copy["displayName"], json!("GPT-4o Main-Copy"));
    assert!(
        catalog_records
            .iter()
            .any(|record| record["modelCode"] == json!(source_model_code)),
        "duplicating a visible platform model must not remove the inherited platform row",
    );
}

#[tokio::test]
async fn first_created_platform_model_becomes_default_until_explicitly_changed() {
    let database_url = temp_database_url("model-platform-first-default");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let first_text_model_code = create_platform_model(
        &router,
        "Deepseek First",
        "deepseek-v4-pro",
        "OPENAI_COMPATIBLE",
        "TEXT",
    )
    .await;
    let second_text_model_code =
        create_platform_model(&router, "Qwen Second", "qwen3.7-max", "RESPONSES", "TEXT").await;
    let first_image_model_code =
        create_platform_model(&router, "Vision First", "gpt-image-1", "RESPONSES", "IMAGE").await;

    let connection =
        Connection::open(&database_url).expect("platform default test db should reopen");
    let explicit_defaults = {
        let mut statement = connection
            .prepare(
                "SELECT model_type, model_code
                 FROM platform_default_models
                 ORDER BY model_type ASC, model_code ASC",
            )
            .expect("platform default assignment query should prepare");
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .expect("platform default assignment query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("platform default assignments should read")
    };
    assert_eq!(
        explicit_defaults,
        vec![
            ("IMAGE".to_string(), first_image_model_code.clone()),
            ("TEXT".to_string(), first_text_model_code.clone()),
        ],
        "creating the first enabled platform model of each type should persist an explicit default assignment",
    );
    drop(connection);

    let platform_list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    assert_eq!(platform_list_response.status(), StatusCode::OK);
    let platform_list_json = response_json(platform_list_response).await;
    let platform_records = platform_list_json["data"]
        .as_array()
        .expect("platform catalog should return records");

    let first_text = platform_records
        .iter()
        .find(|item| item["modelCode"] == json!(first_text_model_code.clone()))
        .expect("first platform text model should exist");
    assert_eq!(first_text["isDefault"], json!(true));

    let second_text = platform_records
        .iter()
        .find(|item| item["modelCode"] == json!(second_text_model_code))
        .expect("second platform text model should exist");
    assert_eq!(second_text["isDefault"], json!(false));

    let first_image = platform_records
        .iter()
        .find(|item| item["modelCode"] == json!(first_image_model_code))
        .expect("first platform image model should exist");
    assert_eq!(first_image["isDefault"], json!(true));
}

#[tokio::test]
async fn inheritance_toggle_changes_descendant_visibility() {
    let database_url = temp_database_url("model-inheritance-toggle");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let shared_model_code = create_workspace_model(
        &router,
        "PLATFORM",
        "Toggle Shared Writer",
        "gpt-4.1-mini",
        "RESPONSES",
        "TEXT",
    )
    .await;

    set_workspace_model_allow_inheritance(&router, "PLATFORM", &shared_model_code, false).await;

    let list_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    let list_json = response_json(list_response).await;
    let records = list_json["data"]["records"]
        .as_array()
        .expect("workspace catalog should stay readable");
    assert!(
        records
            .iter()
            .all(|item| item["modelCode"] != json!(shared_model_code)),
        "descendants must stop seeing models after allowInheritance is disabled",
    );
}

#[tokio::test]
async fn workspace_catalog_uses_platform_model_codes_and_removed_restore_routes_stay_gone() {
    let database_url = temp_database_url("model-platform-reference-default");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let source_model_code =
        create_platform_model(&router, "GPT-4o Main", "gpt-4o", "RESPONSES", "TEXT").await;
    set_platform_default_for_test(&router, &source_model_code).await;

    let removed_candidates_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE/platform-candidates",
        None,
    )
    .await;
    assert!(
        removed_candidates_response.status() == StatusCode::NOT_FOUND,
        "deprecated restore-candidate route must be removed instead of kept for compatibility",
    );

    let removed_restore_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE/from-platform",
        Some(json!({ "sourceModelCode": source_model_code.clone() })),
    )
    .await;
    assert_eq!(
        removed_restore_response.status(),
        StatusCode::GONE,
        "deprecated explicit restore route should now fail through an explicit tombstone route",
    );

    let catalog_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    let catalog_json = response_json(catalog_response).await;
    let records = catalog_json["data"]["records"]
        .as_array()
        .expect("workspace catalog should expose records");
    let platform_record = records
        .iter()
        .find(|item| item["modelCode"] == json!(source_model_code.clone()))
        .expect("TRON_CORE should automatically see enabled platform models");
    assert_eq!(platform_record["sourceKind"], json!("INHERITED"));
    assert_eq!(
        platform_record["sourceModelCode"],
        json!(source_model_code.clone())
    );

    let set_default_json = set_platform_default_for_test(&router, &source_model_code).await;
    assert_eq!(
        set_default_json["data"]["modelCode"],
        json!(source_model_code)
    );

    let platform_list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    let platform_list_json = response_json(platform_list_response).await;
    let platform_default = platform_list_json["data"]
        .as_array()
        .expect("platform catalog should stay readable")
        .iter()
        .find(|item| item["modelCode"] == json!(source_model_code.clone()))
        .expect("platform list should expose the configured default");
    assert_eq!(platform_default["isDefault"], json!(true));

    let catalog_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    let catalog_json = response_json(catalog_response).await;
    let defaults = catalog_json["data"]["defaults"]
        .as_array()
        .expect("workspace catalog should expose defaults");
    let text_default = defaults
        .iter()
        .find(|item| item["modelType"] == "TEXT")
        .expect("TEXT default assignment should exist");
    assert_eq!(text_default["modelCode"], json!(source_model_code));
    assert_eq!(text_default["displayName"], json!("GPT-4o Main"));
    assert_eq!(text_default["sourceLabel"], json!("Inherited"));

    let options_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE/select-options",
        None,
    )
    .await;
    let options_json = response_json(options_response).await;
    let options = options_json["data"]
        .as_array()
        .expect("select-options should return an array");
    let option = options
        .iter()
        .find(|item| item["modelCode"] == json!(source_model_code))
        .expect("select-options should return visible platform modelCode values");
    assert_eq!(option["displayName"], json!("GPT-4o Main"));
    assert_eq!(option["modelName"], json!("gpt-4o"));
    assert_eq!(option["sourceLabel"], json!("Inherited"));
}

#[tokio::test]
async fn workspace_default_override_does_not_mutate_platform_default() {
    let database_url = temp_database_url("model-workspace-default-override");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let platform_model_code =
        create_platform_model(&router, "GPT-4o Main", "gpt-4o", "RESPONSES", "TEXT").await;
    let workspace_model_code = create_workspace_model(
        &router,
        "TRON_CORE",
        "Core Fast",
        "gpt-4.1-mini",
        "OPENAI_COMPATIBLE",
        "TEXT",
    )
    .await;

    set_platform_default_for_test(&router, &platform_model_code).await;

    let inherited_catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    let inherited_catalog_json = response_json(inherited_catalog).await;
    let inherited_text_default = inherited_catalog_json["data"]["defaults"]
        .as_array()
        .expect("workspace catalog should expose defaults")
        .iter()
        .find(|item| item["modelType"] == "TEXT")
        .expect("TEXT default should exist");
    assert_eq!(
        inherited_text_default["modelCode"],
        json!(platform_model_code)
    );

    let workspace_set_default_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{workspace_model_code}/set-default"),
        None,
    )
    .await;
    assert_eq!(workspace_set_default_response.status(), StatusCode::OK);

    let overridden_catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    let overridden_catalog_json = response_json(overridden_catalog).await;
    let overridden_text_default = overridden_catalog_json["data"]["defaults"]
        .as_array()
        .expect("workspace catalog should expose defaults")
        .iter()
        .find(|item| item["modelType"] == "TEXT")
        .expect("TEXT default should exist");
    assert_eq!(
        overridden_text_default["modelCode"],
        json!(workspace_model_code)
    );
    assert_eq!(overridden_text_default["sourceLabel"], json!("Local"));

    let platform_list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    let platform_list_json = response_json(platform_list_response).await;
    let platform_default = platform_list_json["data"]
        .as_array()
        .expect("platform catalog should stay readable")
        .iter()
        .find(|item| item["modelCode"] == json!(platform_model_code.clone()))
        .expect("platform default should remain unchanged");
    assert_eq!(platform_default["isDefault"], json!(true));
    assert!(
        platform_list_json["data"]
            .as_array()
            .expect("platform catalog should stay readable")
            .iter()
            .all(|item| item["modelCode"] != json!(workspace_model_code)),
        "workspace-local default overrides must not leak into platform catalog",
    );
}

#[tokio::test]
async fn inherited_platform_models_cannot_be_deleted_from_a_workspace() {
    let database_url = temp_database_url("model-workspace-platform-remove");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let source_model_code =
        create_platform_model(&router, "GPT-4o Main", "gpt-4o", "RESPONSES", "TEXT").await;
    set_platform_default_for_test(&router, &source_model_code).await;

    let remove_response = authenticated_request(
        &router,
        Method::DELETE,
        &format!("/api/v1/models/workspaces/TRON_CORE/{source_model_code}"),
        None,
    )
    .await;
    assert_eq!(remove_response.status(), StatusCode::BAD_REQUEST);

    let after_remove_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    let after_remove_json = response_json(after_remove_response).await;
    let after_remove_records = after_remove_json["data"]["records"]
        .as_array()
        .expect("workspace catalog should return records");
    assert!(
        after_remove_records
            .iter()
            .any(|item| item["modelCode"] == json!(source_model_code)),
        "inherited platform models must stay visible because delete is not a valid mutation path",
    );

    let options_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE/select-options",
        None,
    )
    .await;
    let options_json = response_json(options_response).await;
    let options = options_json["data"]
        .as_array()
        .expect("workspace select options should stay readable");
    assert!(
        options
            .iter()
            .any(|item| item["modelCode"] == json!(source_model_code)),
        "selectors must keep inherited models visible when delete is rejected",
    );

    let platform_list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    let platform_list_json = response_json(platform_list_response).await;
    let platform_records = platform_list_json["data"]
        .as_array()
        .expect("platform catalog should remain readable");
    assert!(
        platform_records
            .iter()
            .any(|item| item["modelCode"] == json!(source_model_code)),
        "removing from workspace must not delete the platform configuration",
    );
    let platform_default = platform_records
        .iter()
        .find(|item| item["modelCode"] == json!(source_model_code))
        .expect("platform model should still exist after workspace removal");
    assert_eq!(
        platform_default["isDefault"],
        json!(true),
        "workspace-level remove must not unset the platform default",
    );
}

#[tokio::test]
async fn workspace_can_disable_platform_models_without_mutating_platform_source() {
    let database_url = temp_database_url("model-workspace-platform-disable");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let source_model_code =
        create_platform_model(&router, "GPT-4o Main", "gpt-4o", "RESPONSES", "TEXT").await;
    set_platform_default_for_test(&router, &source_model_code).await;

    let disable_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{source_model_code}/disable"),
        None,
    )
    .await;
    assert_eq!(disable_response.status(), StatusCode::OK);
    let disable_payload = response_json(disable_response).await;
    assert_eq!(
        disable_payload["data"]["modelCode"],
        json!(source_model_code)
    );
    assert_eq!(disable_payload["data"]["sourceKind"], json!("INHERITED"));
    assert_eq!(disable_payload["data"]["enabled"], json!(false));

    let tron_core_catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(tron_core_catalog.status(), StatusCode::OK);
    let tron_core_catalog_json = response_json(tron_core_catalog).await;
    let tron_core_records = tron_core_catalog_json["data"]["records"]
        .as_array()
        .expect("workspace catalog should return records");
    let disabled_record = tron_core_records
        .iter()
        .find(|record| record["modelCode"] == json!(source_model_code))
        .expect("disabled platform model should remain visible in the current workspace catalog");
    assert_eq!(disabled_record["enabled"], json!(false));
    let tron_core_text_default = tron_core_catalog_json["data"]["defaults"]
        .as_array()
        .expect("workspace defaults should be readable")
        .iter()
        .find(|item| item["modelType"] == "TEXT")
        .expect("TEXT default should exist");
    assert_eq!(
        tron_core_text_default["modelCode"],
        Value::Null,
        "workspace-level disable should clear the inherited platform default inside the current workspace only",
    );

    let options_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE/select-options",
        None,
    )
    .await;
    assert_eq!(options_response.status(), StatusCode::OK);
    let options_json = response_json(options_response).await;
    let disabled_option = options_json["data"]
        .as_array()
        .expect("workspace select options should stay readable")
        .iter()
        .find(|item| item["modelCode"] == json!(source_model_code))
        .expect("disabled platform model should remain in current-workspace selector options");
    assert_eq!(disabled_option["enabled"], json!(false));

    let compliance_catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/COMPLIANCE",
        None,
    )
    .await;
    assert_eq!(compliance_catalog.status(), StatusCode::OK);
    let compliance_catalog_json = response_json(compliance_catalog).await;
    let compliance_record = compliance_catalog_json["data"]["records"]
        .as_array()
        .expect("compliance workspace catalog should return records")
        .iter()
        .find(|record| record["modelCode"] == json!(source_model_code))
        .expect("platform model should stay enabled in other workspaces");
    assert_eq!(compliance_record["enabled"], json!(true));
    let compliance_text_default = compliance_catalog_json["data"]["defaults"]
        .as_array()
        .expect("compliance defaults should be readable")
        .iter()
        .find(|item| item["modelType"] == "TEXT")
        .expect("TEXT default should exist");
    assert_eq!(
        compliance_text_default["modelCode"],
        json!(source_model_code)
    );

    let platform_list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    assert_eq!(platform_list_response.status(), StatusCode::OK);
    let platform_list_json = response_json(platform_list_response).await;
    let platform_model = platform_list_json["data"]
        .as_array()
        .expect("platform catalog should remain readable")
        .iter()
        .find(|item| item["modelCode"] == json!(source_model_code))
        .expect("platform configuration should still exist");
    assert_eq!(platform_model["enabled"], json!(true));
    assert_eq!(platform_model["isDefault"], json!(true));

    let enable_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{source_model_code}/enable"),
        None,
    )
    .await;
    assert_eq!(enable_response.status(), StatusCode::OK);

    let reenabled_catalog = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    let reenabled_catalog_json = response_json(reenabled_catalog).await;
    let reenabled_record = reenabled_catalog_json["data"]["records"]
        .as_array()
        .expect("workspace catalog should return records")
        .iter()
        .find(|record| record["modelCode"] == json!(source_model_code))
        .expect("re-enabled platform model should still be visible");
    assert_eq!(reenabled_record["enabled"], json!(true));
    let reenabled_text_default = reenabled_catalog_json["data"]["defaults"]
        .as_array()
        .expect("workspace defaults should be readable")
        .iter()
        .find(|item| item["modelType"] == "TEXT")
        .expect("TEXT default should exist");
    assert_eq!(
        reenabled_text_default["modelCode"],
        json!(source_model_code)
    );
}



#[tokio::test]
async fn workspace_platform_verify_uses_visible_platform_model_code() {
    let database_url = temp_database_url("model-platform-reference-verify");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let source_model_code = create_platform_model_for_verify(
        &router,
        &server.base_url,
        "Workspace Verify Source",
        "workspace-verify-source",
        "RESPONSES",
        "TEXT",
    )
    .await;

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{source_model_code}/verify"),
        None,
    )
    .await;
    assert_eq!(
        verify_response.status(),
        StatusCode::OK,
        "verifying a visible workspace platform model should succeed via the platform source",
    );
    let verify_payload = response_json(verify_response).await;
    assert_eq!(
        verify_payload["data"]["modelCode"],
        json!(source_model_code)
    );
    assert_eq!(verify_payload["data"]["sourceKind"], json!("INHERITED"));
    assert_eq!(
        verify_payload["data"]["sourceModelCode"],
        json!(source_model_code)
    );
    assert_eq!(
        verify_payload["data"]["lastVerifyStatus"],
        json!("SUCCEEDED")
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_uses_protocol_specific_request_shapes() {
    let database_url = temp_database_url("model-verify-shapes");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let text_model_code = create_platform_model_for_verify(
        &router,
        &server.base_url,
        "Verify Responses Text",
        "verify-text",
        "RESPONSES",
        "TEXT",
    )
    .await;
    verify_platform_model_for_test(&router, &text_model_code).await;

    let embedding_model_code = create_platform_model_for_verify(
        &router,
        &server.base_url,
        "Verify Embedding",
        "verify-embedding",
        "OPENAI_COMPATIBLE_EMBEDDING",
        "EMBEDDING",
    )
    .await;
    verify_platform_model_for_test(&router, &embedding_model_code).await;

    let chat_model_code = create_platform_model_for_verify(
        &router,
        &server.base_url,
        "Verify Chat",
        "verify-chat",
        "OPENAI_COMPATIBLE",
        "TEXT",
    )
    .await;
    verify_platform_model_for_test(&router, &chat_model_code).await;

    let image_model_code = create_platform_model_for_verify(
        &router,
        &server.base_url,
        "Verify Image",
        "verify-image",
        "RESPONSES",
        "IMAGE",
    )
    .await;
    verify_platform_model_for_test(&router, &image_model_code).await;

    let rerank_model_code = create_platform_model_for_verify(
        &router,
        &server.base_url,
        "Verify Rerank",
        "verify-rerank",
        "RERANK",
        "RERANK",
    )
    .await;
    verify_platform_model_for_test(&router, &rerank_model_code).await;

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    assert!(
        captures.iter().any(|capture| {
            capture.path == "/responses"
                && capture.body["input"] == json!("ping")
                && capture.body["model"] == json!("verify-text")
                && capture.body["max_output_tokens"] == json!(16)
        }),
        "RESPONSES + TEXT should POST /responses with plain input",
    );
    assert!(
        captures.iter().any(|capture| {
            capture.path == "/embeddings"
                && capture.body["input"] == json!("ping")
                && capture.body["model"] == json!("verify-embedding")
        }),
        "OPENAI_COMPATIBLE_EMBEDDING should POST /embeddings",
    );
    assert!(
        captures.iter().any(|capture| {
            capture.path == "/chat/completions"
                && capture.body["messages"].is_array()
                && capture.body["model"] == json!("verify-chat")
                && capture.body["max_tokens"] == json!(16)
        }),
        "OPENAI_COMPATIBLE + TEXT should POST /chat/completions",
    );
    assert!(
        captures.iter().any(|capture| {
            capture.path == "/responses"
                && capture.body["input"].is_array()
                && capture.body["model"] == json!("verify-image")
                && capture.body["max_output_tokens"] == json!(16)
        }),
        "RESPONSES + IMAGE should POST /responses with array input",
    );
    assert!(
        captures.iter().any(|capture| {
            capture.path == "/rerank"
                && capture.body["query"] == json!("ping")
                && capture.body["documents"].is_array()
                && capture.body["model"] == json!("verify-rerank")
        }),
        "RERANK + RERANK should POST /rerank",
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_preserves_version_prefix_in_base_url() {
    let database_url = temp_database_url("model-verify-version-prefix");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;
    let versioned_base = format!("{}/v1", server.base_url);

    let embedding_model_code = create_platform_model_for_verify(
        &router,
        &versioned_base,
        "Verify Embedding Versioned",
        "verify-embedding-versioned",
        "OPENAI_COMPATIBLE_EMBEDDING",
        "EMBEDDING",
    )
    .await;
    verify_platform_model_for_test(&router, &embedding_model_code).await;

    let rerank_model_code = create_platform_model_for_verify(
        &router,
        &versioned_base,
        "Verify Rerank Versioned",
        "verify-rerank-versioned",
        "RERANK",
        "RERANK",
    )
    .await;
    verify_platform_model_for_test(&router, &rerank_model_code).await;

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    assert!(
        captures
            .iter()
            .any(|capture| capture.path == "/v1/embeddings"),
        "OPENAI-compatible verification must preserve the version prefix in baseUrl",
    );
    assert!(
        captures.iter().any(|capture| capture.path == "/v1/rerank"),
        "rerank verification must preserve the version prefix in baseUrl",
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_uses_workspace_plaintext_api_key_when_sending_bearer_token() {
    let database_url = temp_database_url("model-verify-workspace-plaintext-api-key");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let model_code = create_workspace_model_for_verify(
        &router,
        "TRON_CORE",
        &server.base_url,
        "Workspace Verify Secret",
        "verify-workspace-secret",
        "OpenAI-Compatible",
        "OPENAI_COMPATIBLE",
        "TEXT",
        "sk-workspace-verify-demo",
    )
    .await;

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{model_code}/verify"),
        None,
    )
    .await;
    assert_eq!(verify_response.status(), StatusCode::OK);

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("verify-workspace-secret"))
        .expect("workspace verify should hit the mock provider");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-workspace-verify-demo"),
        "workspace verify should send the saved plaintext apiKey directly",
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_uses_platform_plaintext_api_key_when_sending_bearer_token() {
    let database_url = temp_database_url("model-verify-platform-plaintext-api-key");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);

    let server = verify_mock_server().await;
    let model_code = create_platform_model_for_verify_with_secret(
        &router,
        &format!("{}/v1", server.base_url),
        "Platform Verify Secret",
        "verify-platform-secret",
        "OpenAI",
        "RESPONSES",
        "TEXT",
        "sk-platform-verify-demo",
    )
    .await;
    verify_platform_model_for_test(&router, &model_code).await;

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("verify-platform-secret"))
        .expect("platform verify should hit the mock provider");
    assert_eq!(capture.path, "/v1/responses");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-platform-verify-demo"),
        "platform verify should send the saved plaintext apiKey directly",
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_platform_draft_uses_unsaved_payload_without_persisting() {
    let database_url = temp_database_url("model-verify-platform-draft-preview");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let model_code = create_platform_model_for_verify_with_secret(
        &router,
        &format!("{}/v1", server.base_url),
        "Platform Draft Saved",
        "platform-draft-saved",
        "OpenAI",
        "RESPONSES",
        "TEXT",
        "sk-platform-draft-preview",
    )
    .await;

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{model_code}/verify"),
        Some(json!({
            "displayName": "Platform Draft Verify",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": format!("{}/v1", server.base_url),
            "apiPath": "/responses",
            "apiKeyRef": "",
            "modelName": "platform-draft-verify",
            "enabled": true,
            "description": "Draft verify only",
            "thinkingEnabled": true,
            "structuredOutputEnabled": true,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_payload = response_json(verify_response).await;
    assert_eq!(
        verify_payload["data"]["displayName"],
        json!("Platform Draft Verify")
    );
    assert_eq!(
        verify_payload["data"]["modelName"],
        json!("platform-draft-verify")
    );
    assert_eq!(
        verify_payload["data"]["lastVerifyStatus"],
        json!("SUCCEEDED")
    );

    let list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_payload = response_json(list_response).await;
    let listed_record = list_payload["data"]
        .as_array()
        .expect("platform list should return an array")
        .iter()
        .find(|record| record["modelCode"] == json!(model_code))
        .expect("verified platform record should remain in the list");
    assert_eq!(listed_record["displayName"], json!("Platform Draft Saved"));
    assert_eq!(listed_record["modelName"], json!("platform-draft-saved"));

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("platform-draft-verify"))
        .expect("platform draft verify should send the unsaved model payload");
    assert_eq!(capture.path, "/v1/responses");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-platform-draft-preview"),
        "draft verify should preserve the saved api key when the draft leaves it blank",
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_platform_create_draft_uses_unsaved_payload_without_persisting() {
    let database_url = temp_database_url("model-verify-platform-create-draft-preview");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models/verify",
        Some(json!({
            "displayName": "Platform Create Draft Verify",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": format!("{}/v1", server.base_url),
            "apiPath": "/responses",
            "apiKeyRef": "sk-platform-create-draft-preview",
            "modelName": "platform-create-draft-verify",
            "enabled": true,
            "description": "Create draft verify only",
            "thinkingEnabled": true,
            "structuredOutputEnabled": true,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_payload = response_json(verify_response).await;
    assert_eq!(
        verify_payload["data"]["displayName"],
        json!("Platform Create Draft Verify")
    );
    assert_eq!(
        verify_payload["data"]["modelName"],
        json!("platform-create-draft-verify")
    );
    assert_eq!(
        verify_payload["data"]["lastVerifyStatus"],
        json!("SUCCEEDED")
    );

    let list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_payload = response_json(list_response).await;
    let contains_draft = list_payload["data"]
        .as_array()
        .expect("platform list should return an array")
        .iter()
        .any(|record| record["displayName"] == json!("Platform Create Draft Verify"));
    assert!(
        !contains_draft,
        "create-draft verify must not persist a platform model"
    );

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("platform-create-draft-verify"))
        .expect("platform create draft verify should send the unsaved model payload");
    assert_eq!(capture.path, "/v1/responses");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-platform-create-draft-preview"),
        "create-draft verify should send the current draft api key",
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_workspace_draft_uses_unsaved_payload_without_persisting() {
    let database_url = temp_database_url("model-verify-workspace-draft-preview");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let model_code = create_platform_model_for_verify_with_secret(
        &router,
        &format!("{}/v1", server.base_url),
        "Workspace Inherited Saved",
        "workspace-inherited-saved",
        "OpenAI",
        "RESPONSES",
        "TEXT",
        "sk-workspace-draft-preview",
    )
    .await;

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{model_code}/verify"),
        Some(json!({
            "displayName": "Workspace Draft Verify",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": format!("{}/v1", server.base_url),
            "apiPath": "/responses",
            "apiKeyRef": "",
            "modelName": "workspace-draft-verify",
            "enabled": true,
            "description": "Draft verify only",
            "thinkingEnabled": true,
            "structuredOutputEnabled": true,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_payload = response_json(verify_response).await;
    assert_eq!(verify_payload["data"]["sourceKind"], json!("INHERITED"));
    assert_eq!(
        verify_payload["data"]["displayName"],
        json!("Workspace Draft Verify")
    );
    assert_eq!(
        verify_payload["data"]["modelName"],
        json!("workspace-draft-verify")
    );
    assert_eq!(
        verify_payload["data"]["lastVerifyStatus"],
        json!("SUCCEEDED")
    );

    let catalog_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_payload = response_json(catalog_response).await;
    let listed_record = catalog_payload["data"]["records"]
        .as_array()
        .expect("workspace catalog should return records")
        .iter()
        .find(|record| record["modelCode"] == json!(model_code))
        .expect("inherited platform record should remain visible in the catalog");
    assert_eq!(
        listed_record["displayName"],
        json!("Workspace Inherited Saved")
    );
    assert_eq!(
        listed_record["modelName"],
        json!("workspace-inherited-saved")
    );

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("workspace-draft-verify"))
        .expect("workspace draft verify should send the unsaved model payload");
    assert_eq!(capture.path, "/v1/responses");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-workspace-draft-preview"),
        "workspace draft verify should preserve the saved upstream api key when the draft leaves it blank",
    );

    server.task.abort();
}

#[tokio::test]
async fn verify_workspace_create_draft_uses_unsaved_payload_without_persisting() {
    let database_url = temp_database_url("model-verify-workspace-create-draft-preview");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/models/workspaces/TRON_CORE/verify",
        Some(json!({
            "displayName": "Workspace Create Draft Verify",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": format!("{}/v1", server.base_url),
            "apiPath": "/responses",
            "apiKeyRef": "sk-workspace-create-draft-preview",
            "modelName": "workspace-create-draft-verify",
            "enabled": true,
            "description": "Workspace create draft verify only",
            "thinkingEnabled": true,
            "structuredOutputEnabled": true,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "maxRetries": 0,
            "allowInheritance": true
        })),
    )
    .await;
    assert_eq!(verify_response.status(), StatusCode::OK);
    let verify_payload = response_json(verify_response).await;
    assert_eq!(verify_payload["data"]["sourceKind"], json!("LOCAL"));
    assert_eq!(
        verify_payload["data"]["displayName"],
        json!("Workspace Create Draft Verify")
    );
    assert_eq!(
        verify_payload["data"]["modelName"],
        json!("workspace-create-draft-verify")
    );
    assert_eq!(
        verify_payload["data"]["lastVerifyStatus"],
        json!("SUCCEEDED")
    );

    let catalog_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_payload = response_json(catalog_response).await;
    let contains_draft = catalog_payload["data"]["records"]
        .as_array()
        .expect("workspace catalog should return records")
        .iter()
        .any(|record| record["displayName"] == json!("Workspace Create Draft Verify"));
    assert!(
        !contains_draft,
        "create-draft verify must not persist a workspace model"
    );

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("workspace-create-draft-verify"))
        .expect("workspace create draft verify should send the unsaved model payload");
    assert_eq!(capture.path, "/v1/responses");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-workspace-create-draft-preview"),
        "workspace create-draft verify should send the current draft api key",
    );

    server.task.abort();
}

#[tokio::test]
async fn platform_list_masks_api_key_and_blank_update_preserves_saved_secret() {
    let database_url = temp_database_url("model-platform-mask-and-preserve");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let model_code = create_platform_model_for_verify_with_secret(
        &router,
        &format!("{}/v1", server.base_url),
        "Masked Platform Secret",
        "masked-platform-secret",
        "OpenAI",
        "RESPONSES",
        "TEXT",
        "sk-platform-preserve-demo",
    )
    .await;

    let list_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_payload = response_json(list_response).await;
    let list_records = list_payload["data"]
        .as_array()
        .expect("platform list should return an array");
    let listed_record = list_records
        .iter()
        .find(|record| record["modelCode"] == json!(model_code))
        .expect("created platform model should be present in the list");
    assert_eq!(listed_record["apiKeyRef"], json!("*****"));

    let update_response = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/admin/platform/models/{model_code}"),
        Some(json!({
            "displayName": "Masked Platform Secret Updated",
            "vendor": "OpenAI",
            "protocol": "RESPONSES",
            "modelType": "TEXT",
            "baseUrl": format!("{}/v1", server.base_url),
            "apiPath": "/responses",
            "apiKeyRef": "",
            "modelName": "masked-platform-secret",
            "enabled": true,
            "description": "Updated without replacing the key",
            "thinkingEnabled": true,
            "structuredOutputEnabled": true,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_payload = response_json(update_response).await;
    assert_eq!(update_payload["data"]["apiKeyRef"], json!("*****"));

    verify_platform_model_for_test(&router, &model_code).await;

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("masked-platform-secret"))
        .expect("verify should still hit the provider after blank update");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-platform-preserve-demo"),
        "blank platform apiKey updates should preserve the stored secret",
    );

    server.task.abort();
}

#[tokio::test]
async fn workspace_catalog_masks_api_key_and_blank_update_preserves_saved_secret() {
    let database_url = temp_database_url("model-workspace-mask-and-preserve");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let model_code = create_workspace_model_for_verify(
        &router,
        "TRON_CORE",
        &server.base_url,
        "Masked Workspace Secret",
        "masked-workspace-secret",
        "OpenAI-Compatible",
        "OPENAI_COMPATIBLE",
        "TEXT",
        "sk-workspace-preserve-demo",
    )
    .await;

    let catalog_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/TRON_CORE",
        None,
    )
    .await;
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_payload = response_json(catalog_response).await;
    let catalog_records = catalog_payload["data"]["records"]
        .as_array()
        .expect("workspace catalog should return records");
    let listed_record = catalog_records
        .iter()
        .find(|record| record["modelCode"] == json!(model_code))
        .expect("created workspace model should be present in the catalog");
    assert_eq!(listed_record["apiKeyRef"], json!("*****"));

    let update_response = authenticated_request(
        &router,
        Method::PATCH,
        &format!("/api/v1/models/workspaces/TRON_CORE/{model_code}"),
        Some(json!({
            "displayName": "Masked Workspace Secret Updated",
            "vendor": "OpenAI-Compatible",
            "protocol": "OPENAI_COMPATIBLE",
            "modelType": "TEXT",
            "baseUrl": server.base_url,
            "apiPath": "/chat/completions",
            "apiKeyRef": "",
            "modelName": "masked-workspace-secret",
            "enabled": true,
            "description": "Updated without replacing the key",
            "thinkingEnabled": true,
            "structuredOutputEnabled": false,
            "temperature": 0.2,
            "maxTokens": 1024,
            "topP": 1.0,
            "maxRetries": 0,
            "allowInheritance": true
        })),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_payload = response_json(update_response).await;
    assert_eq!(update_payload["data"]["apiKeyRef"], json!("*****"));

    let verify_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/models/workspaces/TRON_CORE/{model_code}/verify"),
        None,
    )
    .await;
    assert_eq!(verify_response.status(), StatusCode::OK);

    let captures = server
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .clone();
    let capture = captures
        .iter()
        .find(|capture| capture.body["model"] == json!("masked-workspace-secret"))
        .expect("workspace verify should still hit the provider after blank update");
    assert_eq!(
        capture.authorization.as_deref(),
        Some("Bearer sk-workspace-preserve-demo"),
        "blank workspace apiKey updates should preserve the stored secret",
    );

    server.task.abort();
}

#[tokio::test]
async fn created_openai_responses_models_use_versioned_base_urls() {
    let database_url = temp_database_url("model-created-versioned-openai-responses");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let platform_model_code = create_platform_model_with_payload(
        &router,
        model_payload_with_base_url(
            "GPT-4o Main",
            "gpt-4o",
            "RESPONSES",
            "TEXT",
            "https://api.openai.com/v1",
            "/responses",
        ),
    )
    .await;
    let platform_image_model_code = create_workspace_model_with_payload(
        &router,
        "PLATFORM",
        model_payload_with_base_url(
            "Private Image Lab",
            "gpt-image-1",
            "RESPONSES",
            "IMAGE",
            "https://api.openai.com/v1",
            "/responses",
        ),
    )
    .await;

    let platform_response =
        authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None).await;
    assert_eq!(platform_response.status(), StatusCode::OK);
    let platform_payload = response_json(platform_response).await;
    let platform_records = platform_payload["data"]
        .as_array()
        .expect("platform model list should return an array");
    let gpt4o = platform_records
        .iter()
        .find(|record| record["modelCode"] == json!(platform_model_code.clone()))
        .expect("created GPT-4o Main should exist");
    assert_eq!(gpt4o["baseUrl"], json!("https://api.openai.com/v1"));

    let platform_workspace_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/PLATFORM",
        None,
    )
    .await;
    assert_eq!(platform_workspace_response.status(), StatusCode::OK);
    let platform_workspace_payload = response_json(platform_workspace_response).await;
    let platform_workspace_records = platform_workspace_payload["data"]["records"]
        .as_array()
        .expect("platform workspace catalog should return record array");
    let workspace_platform_record = platform_workspace_records
        .iter()
        .find(|record| record["modelCode"] == json!(platform_model_code.clone()))
        .expect("platform workspace catalog should include explicit platform models");
    assert_eq!(
        workspace_platform_record["baseUrl"],
        json!("https://api.openai.com/v1")
    );
    let image_lab = platform_workspace_records
        .iter()
        .find(|record| record["modelCode"] == json!(platform_image_model_code))
        .expect("platform workspace local image model should exist");
    assert_eq!(image_lab["baseUrl"], json!("https://api.openai.com/v1"));

    let compliance_response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/models/workspaces/COMPLIANCE",
        None,
    )
    .await;
    assert_eq!(compliance_response.status(), StatusCode::OK);
    let compliance_payload = response_json(compliance_response).await;
    let compliance_records = compliance_payload["data"]["records"]
        .as_array()
        .expect("compliance workspace catalog should return record array");
    let compliance_platform_record = compliance_records
        .iter()
        .find(|record| record["modelCode"] == json!(platform_model_code))
        .expect("compliance workspace catalog should inherit explicit platform models");
    assert_eq!(
        compliance_platform_record["baseUrl"],
        json!("https://api.openai.com/v1")
    );
}

#[tokio::test]
async fn failed_verify_attempt_persists_failed_status_and_handles_utf8_provider_messages() {
    let database_url = temp_database_url("model-verify-failure-persist");
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    let server = verify_mock_server().await;

    let model_code = create_platform_model_for_verify(
        &router,
        &server.base_url,
        "Verify Failure Persist",
        "verify-failure-persist",
        "RESPONSES",
        "TEXT",
    )
    .await;
    verify_platform_model_for_test(&router, &model_code).await;

    *server
        .response_mode
        .lock()
        .expect("verify response mode mutex poisoned") = VerifyResponseMode::Error {
        status: StatusCode::BAD_GATEWAY,
        body: "验证失败".repeat(90),
    };

    let failed_response = authenticated_request(
        &router,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{model_code}/verify"),
        None,
    )
    .await;
    assert_eq!(failed_response.status(), StatusCode::BAD_GATEWAY);
    let failed_payload = response_json(failed_response).await;
    assert_eq!(failed_payload["code"], json!("MODEL_VERIFY_FAILED"));
    let failed_message = failed_payload["message"]
        .as_str()
        .expect("failed verify response should expose a message");
    assert!(
        failed_message.contains("returned 502 Bad Gateway"),
        "provider failure should be mapped into a structured verify error",
    );
    assert!(
        failed_message.ends_with("..."),
        "long provider messages should be truncated safely instead of panicking",
    );

    let get_response = authenticated_request(
        &router,
        Method::GET,
        &format!("/api/v1/admin/platform/models/{model_code}"),
        None,
    )
    .await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let get_payload = response_json(get_response).await;
    assert_eq!(get_payload["data"]["lastVerifyStatus"], json!("FAILED"));
    assert_eq!(
        get_payload["data"]["lastVerifyMessage"],
        json!(failed_message)
    );

    server.task.abort();
}

#[tokio::test]
async fn missing_platform_route_model_returns_not_found() {
    let router = app();

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/admin/platform/models/MODEL-MISSING",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], json!("MODEL_NOT_FOUND"));
    assert_eq!(payload["message"], json!("model was not found"));
}

async fn create_platform_model_for_verify(
    router: &Router,
    base_url: &str,
    display_name: &str,
    model_name: &str,
    protocol: &str,
    model_type: &str,
) -> String {
    create_model_for_verify(
        router,
        "/api/v1/admin/platform/models",
        display_name,
        "VerifyVendor",
        base_url,
        "sk-platform-verify-default",
        model_name,
        protocol,
        model_type,
    )
    .await
}

async fn create_platform_model_for_verify_with_secret(
    router: &Router,
    base_url: &str,
    display_name: &str,
    model_name: &str,
    vendor: &str,
    protocol: &str,
    model_type: &str,
    api_key_ref: &str,
) -> String {
    create_model_for_verify(
        router,
        "/api/v1/admin/platform/models",
        display_name,
        vendor,
        base_url,
        api_key_ref,
        model_name,
        protocol,
        model_type,
    )
    .await
}

async fn create_workspace_model_for_verify(
    router: &Router,
    workspace_code: &str,
    base_url: &str,
    display_name: &str,
    model_name: &str,
    vendor: &str,
    protocol: &str,
    model_type: &str,
    api_key_ref: &str,
) -> String {
    create_model_for_verify(
        router,
        &format!("/api/v1/models/workspaces/{workspace_code}"),
        display_name,
        vendor,
        base_url,
        api_key_ref,
        model_name,
        protocol,
        model_type,
    )
    .await
}

async fn create_model_for_verify(
    router: &Router,
    uri: &str,
    display_name: &str,
    vendor: &str,
    base_url: &str,
    api_key_ref: &str,
    model_name: &str,
    protocol: &str,
    model_type: &str,
) -> String {
    let model_capabilities = model_capabilities_payload(model_type);
    let response = authenticated_request(
        router,
        Method::POST,
        uri,
        Some(json!({
            "displayName": display_name,
            "vendor": vendor,
            "protocol": protocol,
            "modelType": model_type,
            "modelCapabilities": model_capabilities,
            "baseUrl": base_url,
            "apiKeyRef": api_key_ref,
            "modelName": model_name,
            "enabled": true,
            "thinkingEnabled": model_type == "TEXT",
            "structuredOutputEnabled": protocol == "RESPONSES",
            "maxRetries": 0
        })),
    )
    .await;
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("create response body should be readable");
    let payload: Value = serde_json::from_slice(&body).unwrap_or_else(|_| {
        panic!(
            "create response body should be valid JSON. status={status} body={}",
            String::from_utf8_lossy(&body)
        )
    });
    assert_eq!(
        status,
        axum::http::StatusCode::CREATED,
        "verify fixture create failed with payload: {payload}",
    );
    payload["data"]["modelCode"]
        .as_str()
        .expect("verify fixture create should return modelCode")
        .to_string()
}

async fn verify_platform_model_for_test(router: &Router, model_code: &str) {
    let response = authenticated_request(
        router,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{model_code}/verify"),
        None,
    )
    .await;
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("verify response body should be readable");
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "verify request failed with body: {}",
        String::from_utf8_lossy(&body),
    );
    let payload: Value =
        serde_json::from_slice(&body).expect("verify response body should be valid JSON");
    assert_eq!(payload["data"]["lastVerifyStatus"], json!("SUCCEEDED"));
}

fn model_payload(display_name: &str, model_name: &str, protocol: &str, model_type: &str) -> Value {
    let model_capabilities = model_capabilities_payload(model_type);
    let temperature = if model_type == "TEXT" {
        json!(0.2)
    } else {
        Value::Null
    };
    let max_tokens = if model_type == "TEXT" {
        json!(1024)
    } else {
        Value::Null
    };
    let context_window_tokens = if model_type == "TEXT" {
        json!(128000)
    } else {
        Value::Null
    };
    let top_p = if model_type == "TEXT" {
        json!(1.0)
    } else {
        Value::Null
    };
    json!({
        "displayName": display_name,
        "vendor": "OpenAI",
        "protocol": protocol,
        "modelType": model_type,
        "modelCapabilities": model_capabilities,
        "baseUrl": "https://api.example.com",
        "apiKeyRef": "sk-test-model",
        "modelName": model_name,
        "enabled": true,
        "description": "Test model",
        "thinkingEnabled": model_type == "TEXT",
        "structuredOutputEnabled": protocol == "RESPONSES",
        "temperature": temperature,
        "maxTokens": max_tokens,
        "contextWindowTokens": context_window_tokens,
        "topP": top_p,
        "maxRetries": 0
    })
}

fn model_payload_with_base_url(
    display_name: &str,
    model_name: &str,
    protocol: &str,
    model_type: &str,
    base_url: &str,
    api_path: &str,
) -> Value {
    let mut payload = model_payload(display_name, model_name, protocol, model_type);
    payload["baseUrl"] = json!(base_url);
    payload["apiPath"] = json!(api_path);
    payload
}

async fn create_platform_model(
    router: &Router,
    display_name: &str,
    model_name: &str,
    protocol: &str,
    model_type: &str,
) -> String {
    let response = authenticated_request(
        router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(model_payload(
            display_name,
            model_name,
            protocol,
            model_type,
        )),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    payload["data"]["modelCode"]
        .as_str()
        .expect("created platform model should expose modelCode")
        .to_owned()
}

async fn create_platform_model_with_payload(router: &Router, payload: Value) -> String {
    let response = authenticated_request(
        router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(payload),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_payload = response_json(response).await;
    response_payload["data"]["modelCode"]
        .as_str()
        .expect("created platform model should expose modelCode")
        .to_owned()
}

async fn create_workspace_model(
    router: &Router,
    workspace_code: &str,
    display_name: &str,
    model_name: &str,
    protocol: &str,
    model_type: &str,
) -> String {
    let response = authenticated_request(
        router,
        Method::POST,
        &format!("/api/v1/models/workspaces/{workspace_code}"),
        Some(model_payload(
            display_name,
            model_name,
            protocol,
            model_type,
        )),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    payload["data"]["modelCode"]
        .as_str()
        .expect("created workspace model should expose modelCode")
        .to_owned()
}

async fn create_workspace_model_with_payload(
    router: &Router,
    workspace_code: &str,
    payload: Value,
) -> String {
    let response = authenticated_request(
        router,
        Method::POST,
        &format!("/api/v1/models/workspaces/{workspace_code}"),
        Some(payload),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_payload = response_json(response).await;
    response_payload["data"]["modelCode"]
        .as_str()
        .expect("created workspace model should expose modelCode")
        .to_owned()
}

async fn set_platform_default_for_test(router: &Router, model_code: &str) -> Value {
    let response = authenticated_request(
        router,
        Method::POST,
        &format!("/api/v1/admin/platform/models/{model_code}/set-default"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

async fn set_workspace_model_allow_inheritance(
    router: &Router,
    workspace_code: &str,
    model_code: &str,
    allow_inheritance: bool,
) {
    let response = authenticated_request(
        router,
        Method::POST,
        &format!("/api/v1/models/workspaces/{workspace_code}/{model_code}/allow-inheritance"),
        Some(json!({ "allowInheritance": allow_inheritance })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

fn model_capabilities_payload(model_type: &str) -> Value {
    let values = match model_type {
        "EMBEDDING" => vec!["TEXT_VECTOR"],
        "RERANK" => vec!["RERANK"],
        "IMAGE" => vec!["IMAGE_GENERATION"],
        _ => vec!["TEXT_GENERATION"],
    };
    json!(values)
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tron-{test_name}-{nanos}.sqlite3"))
}

#[derive(Clone)]
struct RequestCapture {
    body: Value,
    path: String,
    authorization: Option<String>,
}

#[derive(Clone)]
struct VerifyServerState {
    captures: Arc<Mutex<Vec<RequestCapture>>>,
    response_mode: Arc<Mutex<VerifyResponseMode>>,
}

struct VerifyMockServer {
    base_url: String,
    captures: Arc<Mutex<Vec<RequestCapture>>>,
    response_mode: Arc<Mutex<VerifyResponseMode>>,
    task: JoinHandle<()>,
}

#[derive(Clone)]
enum VerifyResponseMode {
    Success,
    Error { status: StatusCode, body: String },
}

async fn verify_mock_server() -> VerifyMockServer {
    let captures = Arc::new(Mutex::new(Vec::<RequestCapture>::new()));
    let response_mode = Arc::new(Mutex::new(VerifyResponseMode::Success));
    let state = VerifyServerState {
        captures: captures.clone(),
        response_mode: response_mode.clone(),
    };

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("verify mock server should bind");
    let address = listener
        .local_addr()
        .expect("verify mock server should expose a local address");
    let app = Router::new()
        .route("/chat/completions", post(capture_verify_request))
        .route("/v1/chat/completions", post(capture_verify_request))
        .route("/embeddings", post(capture_verify_request))
        .route("/v1/embeddings", post(capture_verify_request))
        .route("/responses", post(capture_verify_request))
        .route("/v1/responses", post(capture_verify_request))
        .route("/rerank", post(capture_verify_request))
        .route("/v1/rerank", post(capture_verify_request))
        .with_state(state);
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("verify mock server should stay available during the test");
    });

    VerifyMockServer {
        base_url: format!("http://{address}"),
        captures,
        response_mode,
        task,
    }
}

async fn capture_verify_request(
    State(state): State<VerifyServerState>,
    headers: HeaderMap,
    uri: Uri,
    Json(body): Json<Value>,
) -> axum::response::Response {
    state
        .captures
        .lock()
        .expect("verify captures mutex poisoned")
        .push(RequestCapture {
            body,
            path: uri.path().to_string(),
            authorization: headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
        });

    match state
        .response_mode
        .lock()
        .expect("verify response mode mutex poisoned")
        .clone()
    {
        VerifyResponseMode::Success => Json(json!({ "ok": true })).into_response(),
        VerifyResponseMode::Error { status, body } => (
            status,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            body,
        )
            .into_response(),
    }
}

#[tokio::test]
async fn sdk_probe_preview_client_errors_are_single_attempt_and_do_not_persist() {
    let database_url = temp_database_url("sdk-probe-client-errors");
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..Default::default()
    });
    let server = verify_mock_server().await;
    for status in [StatusCode::BAD_REQUEST, StatusCode::UNAUTHORIZED] {
        *server.response_mode.lock().unwrap() = VerifyResponseMode::Error {
            status,
            body: "invalid credential or request".into(),
        };
        server.captures.lock().unwrap().clear();
        let response=authenticated_request(&router,Method::POST,"/api/v1/admin/platform/models/verify",Some(json!({"displayName":"Transient Draft","vendor":"OpenAI","protocol":"OPENAI_COMPATIBLE","modelType":"TEXT","modelCapabilities":["TEXT_GENERATION"],"baseUrl":server.base_url,"apiPath":"/chat/completions","apiKeyRef":"test-sdk-probe","modelName":"probe","enabled":true,"maxRetries":3}))).await;
        assert!(!response.status().is_success());
        assert_eq!(
            server.captures.lock().unwrap().len(),
            1,
            "client errors must not be blindly retried"
        );
        let list =
            authenticated_request(&router, Method::GET, "/api/v1/admin/platform/models", None)
                .await;
        let payload = response_json(list).await;
        assert!(
            !payload["data"]
                .as_array()
                .unwrap()
                .iter()
                .any(|model| model["displayName"] == json!("Transient Draft"))
        );
    }
    server.task.abort();
}
