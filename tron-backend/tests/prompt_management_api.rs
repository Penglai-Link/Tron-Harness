mod common;

use std::{
    collections::BTreeSet,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    http::{Method, StatusCode},
    routing::post,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tron_backend::config::AppConfig;

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
        json!([
            { "name": "release_name", "type": "STRING" },
            { "name": "audience", "type": "STRING" }
        ])
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
async fn workspace_update_creates_isolated_override_and_revert_restores_upstream() {
    let (router, _) = prompt_app("prompt-workspace-override-revert");
    let parent = create_workspace_prompt(
        &router,
        "PLATFORM",
        "Shared Writer",
        "SYSTEM",
        "Parent {{topic}}",
        true,
    )
    .await;
    let prompt_code = parent["data"]["promptCode"]
        .as_str()
        .expect("parent prompt should return promptCode")
        .to_string();

    let inherited = workspace_detail(&router, "TRON_CORE", &prompt_code).await;
    assert_eq!(inherited["data"]["sourceKind"], json!("INHERITED"));
    assert_eq!(inherited["data"]["sourceWorkspaceCode"], json!("PLATFORM"));
    let inherited_version = inherited["data"]["version"]
        .as_u64()
        .expect("inherited prompt should return version");

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
            "modelCode": null,
            "temperature": 0.4,
            "maxOutputTokens": 512,
            "enabled": true,
            "expectedVersion": inherited_version
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

    let unchanged_parent = workspace_detail(&router, "PLATFORM", &prompt_code).await;
    assert_eq!(unchanged_parent["data"]["sourceKind"], json!("LOCAL"));
    assert_eq!(
        unchanged_parent["data"]["promptBody"],
        json!("Parent {{topic}}")
    );

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
            "displayName": "Variable Sync",
            "promptBody": "Only {{third}} remains",
            "description": null,
            "modelCode": null,
            "temperature": null,
            "maxOutputTokens": null,
            "enabled": true,
            "expectedVersion": version
        })),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);

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
            "modelCode": null,
            "temperature": null,
            "maxOutputTokens": null,
            "enabled": true,
            "expectedVersion": version
        })),
    )
    .await;
    assert_eq!(forbidden_request.status(), StatusCode::BAD_REQUEST);
    let forbidden_payload = response_json(forbidden_request).await;
    assert_envelope_only(&forbidden_payload);

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
}

#[tokio::test]
async fn workspace_secondary_commands_follow_the_frozen_contract() {
    let (router, _) = prompt_app("prompt-workspace-secondary-commands");
    let created = create_workspace_prompt(
        &router,
        "TRON_CORE",
        "Workspace Commands",
        "SYSTEM",
        "Hello {{name}}",
        true,
    )
    .await;
    let prompt_code = created["data"]["promptCode"]
        .as_str()
        .expect("created prompt should return promptCode");

    let inheritance = post_json(
        &router,
        "/api/v1/prompts/workspace-set-allow-inheritance",
        json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "allowInheritance": false
        }),
    )
    .await;
    assert_eq!(inheritance["data"]["allowInheritance"], json!(false));

    let disabled = post_json(
        &router,
        "/api/v1/prompts/workspace-set-enabled",
        json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt_code,
            "enabled": false
        }),
    )
    .await;
    assert_eq!(disabled["data"]["enabled"], json!(false));

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
            "modelCode": null,
            "temperature": 0.3,
            "maxOutputTokens": 512,
            "enabled": true
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
            "modelCode": null,
            "temperature": null,
            "maxOutputTokens": null,
            "enabled": true
        }),
    )
    .await;
    assert_eq!(updated["data"]["category"], json!("PERSONA"));
    assert_eq!(
        updated["data"]["variables"],
        json!([{ "name": "topic", "type": "STRING" }])
    );

    let disabled = post_json(
        &router,
        "/api/v1/admin/platform/prompts/set-enabled",
        json!({ "promptCode": prompt_code, "enabled": false }),
    )
    .await;
    assert_eq!(disabled["data"]["enabled"], json!(false));

    let duplicated = post_json(
        &router,
        "/api/v1/admin/platform/prompts/duplicate",
        json!({ "promptCode": prompt_code, "targetCategory": "TEMPLATE" }),
    )
    .await;
    assert_eq!(duplicated["data"]["category"], json!("TEMPLATE"));

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
    let (router, _) = prompt_app("prompt-supporting-contracts");
    create_workspace_prompt(
        &router,
        "TRON_CORE",
        "Exported Prompt",
        "TEMPLATE",
        "Export {{item}}",
        true,
    )
    .await;

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
        "/api/v1/prompts/test-preview",
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
async fn test_preview_renders_variables_and_calls_the_selected_model() {
    let preview_server = preview_mock_server().await;
    let (router, _) = prompt_app("prompt-test-preview");
    let model_response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/models",
        Some(json!({
            "displayName": "Prompt Preview Model",
            "vendor": "OpenAI",
            "protocol": "OPENAI_COMPATIBLE",
            "modelType": "TEXT",
            "modelCapabilities": ["TEXT_GENERATION"],
            "baseUrl": preview_server.base_url,
            "apiPath": "/chat/completions",
            "apiKeyRef": "sk-prompt-preview",
            "modelName": "prompt-preview-model",
            "enabled": true,
            "description": "prompt preview fixture",
            "thinkingEnabled": false,
            "structuredOutputEnabled": false,
            "temperature": 0.2,
            "maxTokens": 512,
            "contextWindowTokens": 8192,
            "topP": 1.0,
            "maxRetries": 0
        })),
    )
    .await;
    assert_eq!(model_response.status(), StatusCode::CREATED);
    let model = response_json(model_response).await;
    let model_code = model["data"]["modelCode"]
        .as_str()
        .expect("preview model should return modelCode")
        .to_string();

    let prompt = post_json(
        &router,
        "/api/v1/prompts/workspace-create",
        json!({
            "workspaceCode": "TRON_CORE",
            "displayName": "Preview Prompt",
            "category": "TEMPLATE",
            "promptBody": "Hello {{name}}",
            "description": null,
            "modelCode": model_code,
            "temperature": 0.4,
            "maxOutputTokens": 256,
            "enabled": true,
            "allowInheritance": true
        }),
    )
    .await;
    let preview = post_json(
        &router,
        "/api/v1/prompts/test-preview",
        json!({
            "workspaceCode": "TRON_CORE",
            "promptCode": prompt["data"]["promptCode"],
            "variables": { "name": "Ada" }
        }),
    )
    .await;
    assert_eq!(preview["data"]["renderedPrompt"], json!("Hello Ada"));
    assert_eq!(preview["data"]["modelCode"], json!(model_code));
    assert_eq!(preview["data"]["output"], json!("Preview output"));

    let platform_preview = post_json(
        &router,
        "/api/v1/prompts/test-preview",
        json!({
            "promptBody": "Platform {{name}}",
            "variables": { "name": "Ada" }
        }),
    )
    .await;
    assert_eq!(
        platform_preview["data"]["renderedPrompt"],
        json!("Platform Ada")
    );
    assert_eq!(platform_preview["data"]["modelCode"], json!(model_code));
    assert_eq!(platform_preview["data"]["output"], json!("Preview output"));

    preview_server.task.abort();
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
            "modelCode": null,
            "temperature": 0.2,
            "maxOutputTokens": 256,
            "enabled": true,
            "allowInheritance": allow_inheritance
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
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
    (app_with_config(config), database_url)
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

struct PreviewMockServer {
    base_url: String,
    task: tokio::task::JoinHandle<()>,
}

async fn preview_mock_server() -> PreviewMockServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("preview mock server should bind");
    let address = listener
        .local_addr()
        .expect("preview mock server should expose an address");
    let app = Router::new().route("/chat/completions", post(preview_completion));
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("preview mock server should stay available");
    });
    PreviewMockServer {
        base_url: format!("http://{address}"),
        task,
    }
}

async fn preview_completion() -> Json<Value> {
    Json(json!({
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "Preview output" },
            "finish_reason": "stop"
        }]
    }))
}
