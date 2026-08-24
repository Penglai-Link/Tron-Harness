mod common;

use std::{
    collections::BTreeSet,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::SqliteVariableRepository,
    application::variable::VariableRepository,
    config::AppConfig,
    domain::{prompt::analyze_template, variable::{PlatformVariable, Variable}},
    shared::id::SnowflakeId,
};

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn workspace_create_page_and_detail_return_consensus_envelopes() {
    let (router, _) = prompt_app("prompt-workspace-create-page-detail");

    let created = create_workspace_prompt(
        &router,
        "TRON_CORE",
        "Release Summary",
        "TEMPLATE",
        "Summarize {{release_name}} for {{audience}}.",
        true,
    )
    .await;
    let prompt_code = created["data"]["promptCode"]
        .as_str()
        .expect("created prompt should return promptCode")
        .to_string();
    assert!(prompt_code.starts_with("PROMPT-"));
    assert_eq!(created["code"], json!("0"));
    assert_eq!(created["message"], json!("success"));
    assert_eq!(created["data"]["category"], json!("TEMPLATE"));
    assert_eq!(created["data"]["sourceKind"], json!("LOCAL"));
    assert_eq!(created["data"]["allowInheritance"], json!(true));
    assert_eq!(
        created["data"]["variables"],
        variable_metadata("Summarize {{release_name}} for {{audience}}.")
    );
    assert_envelope_only(&created);

    let page_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-page",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "current": 1,
            "size": 20
        })),
    )
    .await;
    assert_eq!(page_response.status(), StatusCode::OK);
    let page = response_json(page_response).await;
    assert_eq!(page["code"], json!("0"));
    assert_eq!(page["data"]["current"], json!(1));
    assert_eq!(page["data"]["size"], json!(20));
    assert_eq!(page["data"]["total"], json!(1));
    assert_eq!(page["data"]["records"][0]["promptCode"], json!(prompt_code));
    assert_envelope_only(&page);

    let detail_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-detail",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code
        })),
    )
    .await;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail = response_json(detail_response).await;
    assert_eq!(
        detail["data"]["promptBody"],
        json!("Summarize {{release_name}} for {{audience}}.")
    );
    assert_envelope_only(&detail);
}

#[tokio::test]
async fn prompt_variables_require_matching_descriptions_and_supported_types() {
    let (router, _) = prompt_app("prompt-variable-metadata-contract");
    let typed_variables = Value::Array(vec![
        managed_variable("task", "STRING", "Task description"),
        managed_variable("attempts", "NUMBER", "Retry count"),
        managed_variable("options", "JSON", "Runtime options object"),
    ]);
    let valid_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-create",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "displayName": "Typed Template",
            "category": "TEMPLATE",
            "promptBody": "Retry {{task}} {{attempts}} times with {{options}}",
            "description": null,
            "enabled": true,
            "allowInheritance": true,
            "variables": typed_variables
        })),
    )
    .await;
    assert_eq!(valid_response.status(), StatusCode::OK);
    let valid = response_json(valid_response).await;
    assert_eq!(
        valid["data"]["variables"],
        typed_variables
    );

    for (name, variables) in [
        (
            "Missing variable description",
            Value::Array(vec![managed_variable("name", "STRING", "")]),
        ),
        (
            "Mismatched variable name",
            Value::Array(vec![managed_variable("other", "STRING", "Other value")]),
        ),
    ] {
        let response = authenticated_request(
            &router,
            Method::POST,
            "/api/v1/prompts/workspace-create",
            Some(json!({
                "workspaceCode": "TRON_CORE",
                "displayName": name,
                "category": "TEMPLATE",
                "promptBody": "Hello {{name}}",
                "description": null,
                "enabled": true,
                "allowInheritance": true,
                "variables": variables
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{name}");
        let body = response_json(response).await;
        assert_eq!(body["code"], json!("PROMPT_INPUT_INVALID"), "{name}");
    }
}

#[tokio::test]
async fn runtime_prompt_variables_must_match_the_global_runtime_catalog() {
    let (router, _) = prompt_app("prompt-runtime-variable-contract");
    let valid = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-create",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "displayName": "Runtime variable prompt",
            "category": "SYSTEM",
            "promptBody": "Visible domains: {{visible_domains}}",
            "description": null,
            "enabled": true,
            "allowInheritance": true,
            "variables": [{
                "name": "visible_domains",
                "type": "JSON",
                "description": "Visible tool domains",
                "sourceKind": "RUNTIME",
                "runtimeProvider": "AGENT_VISIBLE_DOMAINS"
            }]
        })),
    )
    .await;
    assert_eq!(valid.status(), StatusCode::OK);
    let valid = response_json(valid).await;
    assert_eq!(valid["data"]["variables"][0]["sourceKind"], json!("RUNTIME"));
    assert_eq!(
        valid["data"]["variables"][0]["runtimeProvider"],
        json!("AGENT_VISIBLE_DOMAINS")
    );

    for (name, variable) in [
        (
            "Mismatched runtime provider",
            json!({
                "name": "visible_domains",
                "type": "JSON",
                "description": "Visible tool domains",
                "sourceKind": "RUNTIME",
                "runtimeProvider": "TASK_CONTEXT"
            }),
        ),
        (
            "Mismatched runtime type",
            json!({
                "name": "visible_domains",
                "type": "STRING",
                "description": "Visible tool domains",
                "sourceKind": "RUNTIME",
                "runtimeProvider": "AGENT_VISIBLE_DOMAINS"
            }),
        ),
    ] {
        let response = authenticated_request(
            &router,
            Method::POST,
            "/api/v1/prompts/workspace-create",
            Some(json!({
                "workspaceCode": "TRON_CORE",
                "displayName": name,
                "category": "SYSTEM",
                "promptBody": "Visible domains: {{visible_domains}}",
                "description": null,
                "enabled": true,
                "allowInheritance": true,
                "variables": [variable]
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{name}");
        assert_eq!(response_json(response).await["code"], json!("PROMPT_INPUT_INVALID"));
    }
}

#[tokio::test]
async fn workspace_update_creates_isolated_override_and_revert_restores_upstream() {
    let (router, _) = prompt_app("prompt-workspace-override-revert");
    let parent_variables = Value::Array(vec![managed_variable(
        "topic",
        "NUMBER",
        "Parent topic number",
    )]);
    let parent = post_json(
        &router,
        "/api/v1/prompts/workspace-create",
        json!({
            "workspaceCode": "PLATFORM",
            "displayName": "Shared Writer",
            "category": "SYSTEM",
            "promptBody": "Parent {{topic}}",
            "description": null,
            "enabled": true,
            "allowInheritance": true,
            "variables": parent_variables.clone()
        }),
    )
    .await;
    let prompt_code = parent["data"]["promptCode"]
        .as_str()
        .expect("parent prompt should return promptCode")
        .to_string();

    let inherited = workspace_detail(&router, "TRON_CORE", &prompt_code).await;
    assert_eq!(inherited["data"]["sourceKind"], json!("INHERITED"));
    assert_eq!(inherited["data"]["sourceWorkspaceCode"], json!("PLATFORM"));
    assert_eq!(inherited["data"]["variables"], parent_variables);
    let inherited_version = inherited["data"]["version"]
        .as_u64()
        .expect("inherited prompt should return version");

    let override_variables = Value::Array(vec![
        managed_variable("topic", "JSON", "Override topic payload"),
        managed_variable("audience", "BOOLEAN", "Audience feature flag"),
    ]);
    let update_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-update",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "displayName": "Shared Writer Override",
            "promptBody": "Child {{topic}} for {{audience}}",
            "description": "child-only override",
            "enabled": true,
            "expectedVersion": inherited_version,
            "variables": override_variables.clone()
        })),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let overridden = response_json(update_response).await;
    assert_eq!(overridden["data"]["promptCode"], json!(prompt_code));
    assert_eq!(overridden["data"]["category"], json!("SYSTEM"));
    assert_eq!(overridden["data"]["sourceKind"], json!("OVERRIDE"));
    assert_eq!(
        overridden["data"]["promptBody"],
        json!("Child {{topic}} for {{audience}}")
    );
    assert_eq!(overridden["data"]["variables"], override_variables);

    let unchanged_parent = workspace_detail(&router, "PLATFORM", &prompt_code).await;
    assert_eq!(unchanged_parent["data"]["sourceKind"], json!("LOCAL"));
    assert_eq!(
        unchanged_parent["data"]["promptBody"],
        json!("Parent {{topic}}")
    );
    assert_eq!(unchanged_parent["data"]["variables"], parent_variables);

    let revert_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-revert",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code
        })),
    )
    .await;
    assert_eq!(revert_response.status(), StatusCode::OK);
    let reverted = response_json(revert_response).await;
    assert_eq!(reverted["data"]["sourceKind"], json!("INHERITED"));
    assert_eq!(reverted["data"]["promptBody"], json!("Parent {{topic}}"));
    assert_eq!(reverted["data"]["variables"], parent_variables);
}

#[tokio::test]
async fn workspace_update_syncs_variables_transactionally() {
    let (router, database_url) = prompt_app("prompt-variable-sync");
    let created = create_workspace_prompt(
        &router,
        "TRON_CORE",
        "Variable Sync",
        "TOOL",
        "{{first}} {{second}}",
        true,
    )
    .await;
    let prompt_code = created["data"]["promptCode"]
        .as_str()
        .expect("created prompt should return promptCode")
        .to_string();
    let version = created["data"]["version"]
        .as_u64()
        .expect("created prompt should return version");

    let update_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-update",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "allowInheritance": false,
            "displayName": "Variable Sync",
            "promptBody": "Only {{third}} remains",
            "description": null,
            "enabled": true,
            "expectedVersion": version,
            "variables": variable_metadata("Only {{third}} remains")
        })),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated = response_json(update_response).await;
    assert_eq!(updated["data"]["allowInheritance"], json!(false));

    let connection = Connection::open(database_url).expect("prompt database should open");
    let names = connection
        .prepare(
            "SELECT variable_name FROM prompt_variables WHERE prompt_code = ?1 ORDER BY position",
        )
        .expect("prompt variable query should prepare")
        .query_map(params![prompt_code], |row| row.get::<_, String>(0))
        .expect("prompt variables should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("prompt variables should decode");
    assert_eq!(names, vec!["third"]);
}

#[tokio::test]
async fn database_and_http_prevent_prompt_code_and_category_updates() {
    let (router, database_url) = prompt_app("prompt-immutable-identity");
    let created = create_workspace_prompt(
        &router,
        "TRON_CORE",
        "Immutable Prompt",
        "PERSONA",
        "Act as {{role}}",
        true,
    )
    .await;
    let prompt_code = created["data"]["promptCode"]
        .as_str()
        .expect("created prompt should return promptCode")
        .to_string();
    let version = created["data"]["version"]
        .as_u64()
        .expect("created prompt should return version");

    let forbidden_request = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-update",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "category": "SYSTEM",
            "displayName": "Immutable Prompt",
            "promptBody": "Act as {{role}}",
            "description": null,
            "enabled": true,
            "expectedVersion": version
        })),
    )
    .await;
    assert_eq!(forbidden_request.status(), StatusCode::BAD_REQUEST);
    let forbidden_payload = response_json(forbidden_request).await;
    assert_envelope_only(&forbidden_payload);

    let missing_version = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-update",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "displayName": "Immutable Prompt",
            "promptBody": "Act as {{role}}",
            "description": null,
            "enabled": true
        })),
    )
    .await;
    assert_eq!(missing_version.status(), StatusCode::BAD_REQUEST);
    assert_envelope_only(&response_json(missing_version).await);

    let invalid_model = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-create",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "displayName": "Invalid Model Prompt",
            "category": "SYSTEM",
            "promptBody": "Hello",
            "description": null,
            "modelCode": "MODEL-NOT-VISIBLE",
            "temperature": null,
            "maxOutputTokens": null,
            "enabled": true,
            "allowInheritance": true
        })),
    )
    .await;
    assert_eq!(invalid_model.status(), StatusCode::BAD_REQUEST);
    assert_envelope_only(&response_json(invalid_model).await);

    let connection = Connection::open(database_url).expect("prompt database should open");
    let category_update = connection.execute(
        "UPDATE prompts SET category = 'SYSTEM' WHERE prompt_code = ?1",
        params![prompt_code],
    );
    assert!(
        category_update.is_err(),
        "category trigger must reject updates"
    );
    let code_update = connection.execute(
        "UPDATE prompts SET prompt_code = 'PROMPT-999' WHERE prompt_code = ?1",
        params![prompt_code],
    );
    assert!(
        code_update.is_err(),
        "promptCode trigger must reject updates"
    );
    let invalid_scope_source = connection.execute(
        "INSERT INTO prompts (
            id, scope_kind, workspace_code, prompt_code, display_name, category, prompt_body,
            description, enabled, source_kind,
            source_prompt_code, allow_inheritance, version, created_at, updated_at
         ) VALUES (
            9000001, 'WORKSPACE', 'TRON_CORE', 'PROMPT-9000001', 'Invalid Scope', 'SYSTEM',
            'Invalid', NULL, 1, 'PLATFORM', NULL, NULL, 1, 1, 1
         )",
        [],
    );
    assert!(
        invalid_scope_source.is_err(),
        "workspace records must not use the platform source kind"
    );
}

#[tokio::test]
async fn workspace_secondary_commands_follow_the_frozen_contract() {
    let (router, _) = prompt_app("prompt-workspace-secondary-commands");
    let variables = Value::Array(vec![
        managed_variable("name", "STRING", "Recipient name"),
        managed_variable("attempts", "NUMBER", "Retry attempts"),
    ]);
    let created = post_json(
        &router,
        "/api/v1/prompts/workspace-create",
        json!({
            "workspaceCode": "TRON_CORE",
            "displayName": "Workspace Commands",
            "category": "SYSTEM",
            "promptBody": "Hello {{name}} after {{attempts}} attempts",
            "description": null,
            "enabled": true,
            "allowInheritance": true,
            "variables": variables.clone()
        }),
    )
    .await;
    assert_eq!(created["data"]["variables"], variables);
    let prompt_code = created["data"]["promptCode"]
        .as_str()
        .expect("created prompt should return promptCode");
    let created_version = created["data"]["version"]
        .as_u64()
        .expect("created prompt should return version");

    let inheritance = post_json(
        &router,
        "/api/v1/prompts/workspace-set-allow-inheritance",
        json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "allowInheritance": false,
            "expectedVersion": created_version
        }),
    )
    .await;
    assert_eq!(inheritance["data"]["allowInheritance"], json!(false));
    assert_eq!(inheritance["data"]["variables"], variables);
    let inherited_version = inheritance["data"]["version"]
        .as_u64()
        .expect("inheritance update should return version");

    let disabled = post_json(
        &router,
        "/api/v1/prompts/workspace-set-enabled",
        json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "enabled": false,
            "expectedVersion": inherited_version
        }),
    )
    .await;
    assert_eq!(disabled["data"]["enabled"], json!(false));
    assert_eq!(disabled["data"]["variables"], variables);

    let duplicated = post_json(
        &router,
        "/api/v1/prompts/workspace-duplicate",
        json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "targetCategory": "TOOL"
        }),
    )
    .await;
    assert_eq!(duplicated["data"]["category"], json!("TOOL"));
    assert_ne!(duplicated["data"]["promptCode"], json!(prompt_code));
    assert_eq!(duplicated["data"]["variables"], variables);

    let deleted = post_json(
        &router,
        "/api/v1/prompts/workspace-delete",
        json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": duplicated["data"]["promptCode"]
        }),
    )
    .await;
    assert_eq!(deleted["data"]["workspaceCode"], json!("TRON_CORE"));
}

#[tokio::test]
async fn platform_prompt_lifecycle_uses_prompt_local_envelopes() {
    let (router, _) = prompt_app("prompt-platform-lifecycle");
    let created = post_json(
        &router,
        "/api/v1/admin/platform/prompts/create",
        json!({
            "displayName": "Platform Writer",
            "category": "PERSONA",
            "promptBody": "Write for {{audience}}",
            "description": "platform prompt",
            "enabled": true,
            "variables": variable_metadata("Write for {{audience}}")
        }),
    )
    .await;
    let prompt_code = created["data"]["promptCode"]
        .as_str()
        .expect("platform create should return promptCode")
        .to_string();
    assert_eq!(created["data"]["scopeKind"], json!("PLATFORM"));
    assert_eq!(created["data"]["sourceKind"], json!("PLATFORM"));
    assert_envelope_only(&created);

    let page = post_json(
        &router,
        "/api/v1/admin/platform/prompts/page",
        json!({ "current": 1, "size": 20 }),
    )
    .await;
    assert_eq!(page["data"]["total"], json!(1));

    let detail = post_json(
        &router,
        "/api/v1/admin/platform/prompts/detail",
        json!({ "promptCode": prompt_code }),
    )
    .await;
    let version = detail["data"]["version"]
        .as_u64()
        .expect("platform detail should return version");

    let updated = post_json(
        &router,
        "/api/v1/admin/platform/prompts/update",
        json!({
            "promptCode": prompt_code,
            "expectedVersion": version,
            "displayName": "Platform Writer Updated",
            "promptBody": "Updated {{topic}}",
            "description": null,
            "enabled": true,
            "variables": [managed_variable("topic", "JSON", "Updated topic payload")]
        }),
    )
    .await;
    assert_eq!(updated["data"]["category"], json!("PERSONA"));
    assert_eq!(
        updated["data"]["variables"],
        Value::Array(vec![managed_variable("topic", "JSON", "Updated topic payload")])
    );

    let disabled = post_json(
        &router,
        "/api/v1/admin/platform/prompts/set-enabled",
        json!({
            "promptCode": prompt_code,
            "enabled": false,
            "expectedVersion": updated["data"]["version"]
        }),
    )
    .await;
    assert_eq!(disabled["data"]["enabled"], json!(false));
    assert_eq!(disabled["data"]["variables"], updated["data"]["variables"]);

    let duplicated = post_json(
        &router,
        "/api/v1/admin/platform/prompts/duplicate",
        json!({ "promptCode": prompt_code, "targetCategory": "TEMPLATE" }),
    )
    .await;
    assert_eq!(duplicated["data"]["category"], json!("TEMPLATE"));
    assert_eq!(
        duplicated["data"]["variables"],
        updated["data"]["variables"]
    );

    let deleted = post_json(
        &router,
        "/api/v1/admin/platform/prompts/delete",
        json!({ "promptCode": duplicated["data"]["promptCode"] }),
    )
    .await;
    assert_eq!(deleted["data"]["workspaceCode"], Value::Null);
}

#[tokio::test]
async fn analyze_export_and_route_table_match_the_frozen_post_contract() {
    let (router, database_url) = prompt_app("prompt-supporting-contracts");
    create_workspace_prompt(
        &router,
        "TRON_CORE",
        "Exported Prompt",
        "TEMPLATE",
        "Export {{item}}",
        true,
    )
    .await;

    let mut connection = Connection::open(database_url).expect("prompt database should open");
    let transaction = connection
        .transaction()
        .expect("export fixtures should start a transaction");
    for index in 0..205_i64 {
        transaction
            .execute(
                "INSERT INTO prompts (
                    id, scope_kind, workspace_code, prompt_code, display_name, category,
                    prompt_body, description, enabled, source_kind, source_prompt_code,
                    allow_inheritance, version,
                    created_at, updated_at
                 ) VALUES (
                    ?1, 'WORKSPACE', 'TRON_CORE', ?2, ?3, 'TEMPLATE', 'Bulk export', NULL,
                    1, 'WORKSPACE_LOCAL', NULL, 1, 1, 1, 1
                 )",
                params![
                    9_100_000_i64 + index,
                    format!("PROMPT-BULK-{index}"),
                    format!("Bulk Export {index:03}")
                ],
            )
            .expect("bulk export fixture should insert");
    }
    transaction
        .commit()
        .expect("bulk export fixtures should commit");

    let analysis = post_json(
        &router,
        "/api/v1/prompts/analyze-template",
        json!({ "promptBody": "Hello {{name}}" }),
    )
    .await;
    assert_eq!(
        analysis["data"],
        json!({
            "variables": [{ "name": "name", "type": "STRING" }],
            "invalidVariables": []
        })
    );

    let exported = post_json(
        &router,
        "/api/v1/prompts/workspace-export",
        json!({
            "workspaceCode": "TRON_CORE",
            "current": 1,
            "size": 100
        }),
    )
    .await;
    assert_eq!(
        exported["data"]["fileName"],
        json!("tron_core-prompts.json")
    );
    let content = exported["data"]["content"]
        .as_str()
        .expect("workspace export should return JSON content");
    assert!(content.contains("Exported Prompt"));
    let exported_records: Vec<Value> =
        serde_json::from_str(content).expect("workspace export should contain JSON records");
    assert_eq!(exported_records.len(), 206);

    for path in [
        "/api/v1/prompts/workspace-page",
        "/api/v1/prompts/workspace-detail",
        "/api/v1/prompts/workspace-create",
        "/api/v1/prompts/workspace-update",
        "/api/v1/prompts/workspace-duplicate",
        "/api/v1/prompts/workspace-delete",
        "/api/v1/prompts/workspace-revert",
        "/api/v1/prompts/workspace-set-enabled",
        "/api/v1/prompts/workspace-set-allow-inheritance",
        "/api/v1/prompts/analyze-template",
        "/api/v1/prompts/workspace-export",
        "/api/v1/admin/platform/prompts/page",
        "/api/v1/admin/platform/prompts/detail",
        "/api/v1/admin/platform/prompts/create",
        "/api/v1/admin/platform/prompts/update",
        "/api/v1/admin/platform/prompts/duplicate",
        "/api/v1/admin/platform/prompts/delete",
        "/api/v1/admin/platform/prompts/set-enabled",
    ] {
        let response = authenticated_request(&router, Method::GET, path, None).await;
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{path} must exist and reject non-POST requests"
        );
    }
}

#[tokio::test]
async fn prompt_contract_excludes_model_configuration() {
    let (router, database_url) = prompt_app("prompt-without-model-configuration");
    let created = post_json(
        &router,
        "/api/v1/prompts/workspace-create",
        json!({
            "workspaceCode": "TRON_CORE",
            "displayName": "Pure Template",
            "category": "TEMPLATE",
            "promptBody": "Hello {{name}}",
            "description": null,
            "enabled": true,
            "allowInheritance": true,
            "variables": variable_metadata("Hello {{name}}")
        }),
    )
    .await;
    let data = created["data"]
        .as_object()
        .expect("created prompt data should be an object");
    assert!(!data.contains_key("modelCode"));
    assert!(!data.contains_key("temperature"));
    assert!(!data.contains_key("maxOutputTokens"));

    let rejected = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/workspace-create",
        Some(json!({
            "workspaceCode": "TRON_CORE",
            "displayName": "Prompt With Model",
            "category": "TEMPLATE",
            "promptBody": "Hello",
            "description": null,
            "modelCode": null,
            "enabled": true,
            "allowInheritance": true
        })),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);

    let connection = Connection::open(database_url).expect("prompt database should open");
    let mut statement = connection
        .prepare("PRAGMA table_info(prompts)")
        .expect("prompt schema should be readable");
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("prompt schema should be queryable")
        .collect::<Result<BTreeSet<_>, _>>()
        .expect("prompt columns should decode");
    assert!(!columns.contains("model_code"));
    assert!(!columns.contains("temperature"));
    assert!(!columns.contains("max_output_tokens"));
}

#[tokio::test]
async fn prompt_management_does_not_expose_template_rendering() {
    let (router, _) = prompt_app("prompt-without-render-template");
    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/render-template",
        Some(json!({
            "promptBody": "Hello {{name}}",
            "variables": { "name": "Ada" }
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn create_workspace_prompt(
    router: &axum::Router,
    workspace_code: &str,
    display_name: &str,
    category: &str,
    body: &str,
    allow_inheritance: bool,
) -> Value {
    let response = authenticated_request(
        router,
        Method::POST,
        "/api/v1/prompts/workspace-create",
        Some(json!({
            "workspaceCode": workspace_code,
            "displayName": display_name,
            "category": category,
            "promptBody": body,
            "description": null,
            "enabled": true,
            "allowInheritance": allow_inheritance,
            "variables": variable_metadata(body)
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

fn variable_metadata(body: &str) -> Value {
    Value::Array(
        analyze_template(body)
            .expect("test prompt body should contain valid variables")
            .into_iter()
            .map(|name| managed_variable(&name, "STRING", &format!("Variable {name}")))
            .collect(),
    )
}

fn managed_variable(name: &str, variable_type: &str, description: &str) -> Value {
    let managed_variable_key = match variable_type {
        "STRING" => "PROMPT_TEST_STRING",
        "NUMBER" => "PROMPT_TEST_NUMBER",
        "BOOLEAN" => "PROMPT_TEST_BOOLEAN",
        "JSON" => "PROMPT_TEST_JSON",
        _ => panic!("unsupported prompt test variable type: {variable_type}"),
    };
    json!({
        "description": description,
        "managedVariableKey": managed_variable_key,
        "name": name,
        "sourceKind": "MANAGED",
        "type": variable_type
    })
}

async fn workspace_detail(router: &axum::Router, workspace_code: &str, prompt_code: &str) -> Value {
    let response = authenticated_request(
        router,
        Method::POST,
        "/api/v1/prompts/workspace-detail",
        Some(json!({
            "workspaceCode": workspace_code,
            "promptCode": prompt_code
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

async fn post_json(router: &axum::Router, path: &str, body: Value) -> Value {
    let response = authenticated_request(router, Method::POST, path, Some(body)).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "POST {path} should succeed"
    );
    let payload = response_json(response).await;
    assert_eq!(payload["code"], json!("0"));
    assert_eq!(payload["message"], json!("success"));
    assert_envelope_only(&payload);
    payload
}

fn prompt_app(test_name: &str) -> (axum::Router, PathBuf) {
    let database_url = temp_database_url(test_name);
    let config = AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config);
    seed_prompt_managed_variables(&database_url);
    (router, database_url)
}

fn seed_prompt_managed_variables(database_url: &PathBuf) {
    let values = [
        ("PROMPT_TEST_STRING", "prompt test value"),
        ("PROMPT_TEST_NUMBER", "3"),
        ("PROMPT_TEST_BOOLEAN", "true"),
        ("PROMPT_TEST_JSON", r#"{"mode":"test"}"#),
    ];
    let mut repository = SqliteVariableRepository::connect(&database_url.to_string_lossy())
        .expect("prompt test variables should initialize");
    for (index, (key, value)) in values.into_iter().enumerate() {
        let id = 734_500_000_000_001_000_i64 + index as i64;
        let description = Some(format!("Prompt test {key}"));
        repository
            .create_platform_variable(&PlatformVariable {
                id: SnowflakeId::new(id),
                key: key.into(),
                value: value.into(),
                description: description.clone(),
                is_sensitive: false,
                value_nonce: None,
                value_tag: None,
                value_algorithm: None,
                value_key_version: None,
                created_at: "2026-08-24T00:00:00Z".into(),
                updated_at: "2026-08-24T00:00:00Z".into(),
            })
            .expect("prompt platform test variable should create");
        repository
            .create_variable(&Variable {
                id: SnowflakeId::new(id + 10),
                workspace_code: "TRON_CORE".into(),
                key: key.into(),
                value: value.into(),
                description,
                is_sensitive: false,
                value_nonce: None,
                value_tag: None,
                value_algorithm: None,
                value_key_version: None,
                created_at: "2026-08-24T00:00:00Z".into(),
                updated_at: "2026-08-24T00:00:00Z".into(),
            })
            .expect("prompt workspace test variable should create");
    }
}

fn temp_database_url(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "tron-{test_name}-{}-{nonce}.sqlite3",
        std::process::id()
    ))
}

fn assert_envelope_only(payload: &Value) {
    let keys = payload
        .as_object()
        .expect("response should be an object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from([
            "code".to_string(),
            "data".to_string(),
            "message".to_string()
        ])
    );
}
