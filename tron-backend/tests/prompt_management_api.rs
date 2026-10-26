mod common;

use std::{
    collections::BTreeSet,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::{Method, StatusCode};
use rusqlite::Connection;
use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::SqliteVariableRepository,
    application::variable::VariableRepository,
    config::AppConfig,
    domain::{prompt::analyze_template, variable::PlatformVariable},
    shared::id::SnowflakeId,
};

use common::{app_with_config, authenticated_request, response_json};

#[tokio::test]
async fn developer_prompts_are_managed_separately_from_system_prompts() {
    let database_url = temp_database_url("developer-prompt-category");
    let database_url = database_url.with_file_name(format!(
        "seeded-{}", database_url.file_name().unwrap().to_string_lossy()
    ));
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    for prompt_code in [
        "SYS_CHAT_ASK_USER",
        "SYS_AGENT_DOMAIN_DISCOVERY",
        "DEV_AGENT_GOAL_JUDGE",
        "DEV_MEMORY_CONTEXT_SUMMARY",
        "DEV_SKILLS_GUIDANCE",
        "DEV_A2UI_GUIDANCE",
        "DEV_AGENT_DELEGATED_SYNC",
        "DEV_AGENT_DELEGATED_BACKGROUND",
        "DEV_MODEL_CHAT_RERANK",
        "DEV_DUDE_IDENTITY_GUIDANCE",
        "DEV_DUDE_SOUL_GUIDANCE",
        "DEV_DUDE_ONBOARDING_LANGUAGE",
        "DEV_DUDE_INITIAL_SETUP",
        "DEV_AGENT_DIRECTORY_GUIDANCE",
    ] {
        let response = authenticated_request(
            &router,
            Method::POST,
            "/api/v1/admin/platform/prompts/detail",
            Some(json!({ "promptCode": prompt_code })),
        ).await;
        assert_eq!(response.status(), StatusCode::OK, "{prompt_code}");
        let detail = response_json(response).await;
        assert_eq!(detail["data"]["category"], json!("DEVELOPER"), "{prompt_code}");
    }
    let terminal = post_json(
        &router,
        "/api/v1/admin/platform/prompts/detail",
        json!({ "promptCode": "SYS_TERMINAL_COMMAND_EXACT" }),
    ).await;
    assert_eq!(terminal["data"]["category"], json!("TEMPLATE"));
    let custom = post_json(
        &router,
        "/api/v1/admin/platform/prompts/create",
        json!({
            "displayName": "Custom developer rule",
            "category": "DEVELOPER",
            "promptBody": "Follow the workspace rule.",
            "description": null,
            "enabled": true,
            "variables": []
        }),
    ).await;
    assert_eq!(custom["data"]["category"], json!("DEVELOPER"));

    Connection::open(&database_url).unwrap().execute(
        "UPDATE prompts SET category = 'SYSTEM', version = 100, prompt_body = 'Locally edited rule' WHERE prompt_code = 'SYS_CHAT_ASK_USER'",
        [],
    ).unwrap();
    let restarted = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let migrated = post_json(
        &restarted,
        "/api/v1/admin/platform/prompts/detail",
        json!({ "promptCode": "SYS_CHAT_ASK_USER" }),
    ).await;
    assert_eq!(migrated["data"]["category"], json!("DEVELOPER"));
    assert_eq!(migrated["data"]["promptBody"], json!("Locally edited rule"));
}

#[tokio::test]
async fn platform_prompt_lifecycle_uses_global_records_and_prompt_local_envelopes() {
    let (router, database_url) = prompt_app("platform-prompt-lifecycle");
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
    for field in [
        "scopeKind",
        "workspaceCode",
        "sourceKind",
        "allowInheritance",
    ] {
        assert!(
            !created["data"].as_object().unwrap().contains_key(field),
            "{field}"
        );
    }
    assert_envelope_only(&created);

    let page = post_json(
        &router,
        "/api/v1/admin/platform/prompts/page",
        json!({ "current": 1, "size": 20, "search": "Platform Writer" }),
    )
    .await;
    assert_eq!(page["data"]["total"], json!(1));
    assert_eq!(page["data"]["records"][0]["promptCode"], json!(prompt_code));

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
    assert_eq!(updated["data"]["variables"][0]["name"], json!("topic"));

    let duplicated = post_json(
        &router,
        "/api/v1/admin/platform/prompts/duplicate",
        json!({ "promptCode": prompt_code, "targetCategory": "TEMPLATE" }),
    )
    .await;
    assert_eq!(duplicated["data"]["category"], json!("TEMPLATE"));
    assert_eq!(duplicated["data"]["variables"][0]["name"], json!("topic"));

    let deleted = post_json(
        &router,
        "/api/v1/admin/platform/prompts/delete",
        json!({ "promptCode": duplicated["data"]["promptCode"] }),
    )
    .await;
    assert_eq!(
        deleted["data"],
        json!({ "promptCode": duplicated["data"]["promptCode"] })
    );

    let connection = Connection::open(database_url).expect("prompt database should open");
    let columns = table_columns(&connection, "prompts");
    for removed_column in [
        "scope_kind",
        "workspace_code",
        "source_kind",
        "source_prompt_code",
        "allow_inheritance",
    ] {
        assert!(
            !columns.contains(removed_column),
            "database still has {removed_column}"
        );
    }
}

#[tokio::test]
async fn workspace_prompt_routes_are_removed_and_platform_routes_remain_post_only() {
    let (router, _) = prompt_app("workspace-prompt-routes-removed");
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
        "/api/v1/prompts/workspace-export",
    ] {
        let response = authenticated_request(&router, Method::POST, path, Some(json!({}))).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }

    let response = authenticated_request(
        &router,
        Method::GET,
        "/api/v1/variables/workspaces/TRON_CORE/prompt-options",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    for path in [
        "/api/v1/admin/platform/prompts/page",
        "/api/v1/admin/platform/prompts/detail",
        "/api/v1/admin/platform/prompts/create",
        "/api/v1/admin/platform/prompts/update",
        "/api/v1/admin/platform/prompts/duplicate",
        "/api/v1/admin/platform/prompts/delete",
        "/api/v1/admin/platform/prompts/set-enabled",
    ] {
        let response = authenticated_request(&router, Method::GET, path, None).await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
    }
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
        "/api/v1/admin/platform/prompts/create",
        Some(json!({
            "displayName": "Typed Template",
            "category": "TEMPLATE",
            "promptBody": "Retry {{task}} {{attempts}} times with {{options}}",
            "description": null,
            "enabled": true,
            "variables": typed_variables
        })),
    )
    .await;
    assert_eq!(valid_response.status(), StatusCode::OK);
    let valid = response_json(valid_response).await;
    assert_eq!(valid["data"]["variables"], typed_variables);

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
            "/api/v1/admin/platform/prompts/create",
            Some(json!({
                "displayName": name,
                "category": "TEMPLATE",
                "promptBody": "Hello {{name}}",
                "description": null,
                "enabled": true,
                "variables": variables
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{name}");
        assert_eq!(
            response_json(response).await["code"],
            json!("PROMPT_INPUT_INVALID")
        );
    }
}

#[tokio::test]
async fn runtime_prompt_variables_must_match_the_global_runtime_catalog() {
    let (router, _) = prompt_app("prompt-runtime-variable-contract");
    let valid = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/prompts/create",
        Some(json!({
            "displayName": "Runtime variable prompt",
            "category": "SYSTEM",
            "promptBody": "Latest message: {{user_message}}",
            "description": null,
            "enabled": true,
            "variables": [{
                "name": "user_message",
                "type": "STRING",
                "description": "Chat user message",
                "sourceKind": "RUNTIME",
                "runtimeProvider": "CHAT_USER_MESSAGE"
            }]
        })),
    )
    .await;
    assert_eq!(valid.status(), StatusCode::OK);
    let valid = response_json(valid).await;
    assert_eq!(
        valid["data"]["variables"][0]["sourceKind"],
        json!("RUNTIME")
    );

    let invalid = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/prompts/create",
        Some(json!({
            "displayName": "Invalid runtime prompt",
            "category": "SYSTEM",
            "promptBody": "Persistence: {{persistence_mode}}",
            "description": null,
            "enabled": true,
            "variables": [{
                "name": "persistence_mode",
                "type": "STRING",
                "description": "How expanded domain tools are persisted",
                "sourceKind": "RUNTIME",
                "runtimeProvider": "AGENT_DOMAIN_PERSISTENCE_MODE"
            }]
        })),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    // The retired discovery persistence provider no longer deserializes, so the
    // request body itself is rejected before variable validation runs.
    assert_eq!(
        response_json(invalid).await["code"],
        json!("PROMPT_REQUEST_INVALID")
    );
}

#[tokio::test]
async fn database_and_http_prevent_prompt_code_and_category_updates() {
    let (router, database_url) = prompt_app("prompt-immutable-identity");
    let created = post_json(
        &router,
        "/api/v1/admin/platform/prompts/create",
        json!({
            "displayName": "Immutable Prompt",
            "category": "PERSONA",
            "promptBody": "Act as {{role}}",
            "description": null,
            "enabled": true,
            "variables": variable_metadata("Act as {{role}}")
        }),
    )
    .await;
    let prompt_code = created["data"]["promptCode"].as_str().unwrap().to_string();

    let forbidden_request = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/prompts/update",
        Some(json!({
            "promptCode": prompt_code,
            "expectedVersion": created["data"]["version"],
            "category": "SYSTEM",
            "displayName": "Immutable Prompt",
            "promptBody": "Act as {{role}}",
            "description": null,
            "enabled": true,
            "variables": variable_metadata("Act as {{role}}")
        })),
    )
    .await;
    assert_eq!(forbidden_request.status(), StatusCode::BAD_REQUEST);
    assert_envelope_only(&response_json(forbidden_request).await);

    let connection = Connection::open(database_url).expect("prompt database should open");
    assert!(
        connection
            .execute(
                "UPDATE prompts SET category = 'SYSTEM' WHERE prompt_code = ?1",
                [&prompt_code]
            )
            .is_err()
    );
    assert!(
        connection
            .execute(
                "UPDATE prompts SET prompt_code = 'PROMPT-999' WHERE prompt_code = ?1",
                [&prompt_code]
            )
            .is_err()
    );
}

#[tokio::test]
async fn domain_discovery_keeps_inventory_out_of_fixed_instructions() {
    // Builtin prompts are seeded only for databases whose file name does not
    // start with `tron-` (see `should_seed_builtin_prompts`), so rename the
    // temp database exactly like the neighbouring developer-prompt test.
    let database_url = temp_database_url("domain-discovery-runtime-contract");
    let database_url = database_url.with_file_name(format!(
        "seeded-{}",
        database_url.file_name().unwrap().to_string_lossy()
    ));
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let response = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/admin/platform/prompts/detail",
        Some(json!({ "promptCode": "SYS_AGENT_DOMAIN_DISCOVERY" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let detail = response_json(response).await;
    assert_eq!(detail["data"]["category"], json!("DEVELOPER"));
    assert_eq!(detail["data"]["runtimeRequired"], json!(true));
    assert_eq!(
        detail["data"]["catalogKey"],
        json!("agent.domain-discovery")
    );

    // Dynamic inventory belongs to its configured contributor, not this prompt.
    assert_eq!(detail["data"]["variables"], json!([]));
    assert_eq!(detail["data"]["version"], json!(8));
    let translations = detail["data"]["translations"]
        .as_array()
        .expect("builtin translations should be an array");
    for locale in ["en-US", "zh-CN"] {
        let translation = translations
            .iter()
            .find(|translation| translation["locale"] == json!(locale))
            .unwrap_or_else(|| panic!("missing {locale} translation: {translations:?}"));
        let body = translation["promptBody"]
            .as_str()
            .expect("prompt body should be text");
        assert!(body.contains("\"items\":[{\"tool\":"), "{locale}: missing batched exact-definition example");
        assert!(body.contains("description"), "{locale}: missing summary contract");
        assert!(body.contains("input_schema"), "{locale}: missing definition contract");
        assert!(body.contains("tool_domains") && !body.contains("{{visible_domains}}"), "{locale}: {body}");
        assert!(body.contains(eliteforge_agent_core::TOOL_SEARCH_CALL_NAME), "{locale}: {body}");
        assert!(body.contains(eliteforge_agent_core::TOOL_EXECUTE_CALL_NAME), "{locale}: {body}");
        assert!(
            !body.contains("discovery_expand_tool_domain"),
            "retired discovery tool leaked into {locale}: {body}"
        );
    }

    // Reopening a database with the previous gateway instructions must refresh
    // both the primary body and published translations, not just clean installs.
    Connection::open(&database_url).unwrap().execute(
        "UPDATE prompts SET version = 6, prompt_body = 'stale gateway contract' WHERE prompt_code = 'SYS_AGENT_DOMAIN_DISCOVERY'",
        [],
    ).unwrap();
    Connection::open(&database_url).unwrap().execute(
        "UPDATE prompt_translations SET source_version = 6, prompt_body = 'stale translation' WHERE prompt_code = 'SYS_AGENT_DOMAIN_DISCOVERY'",
        [],
    ).unwrap();
    drop(router);
    let restarted = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let refreshed = post_json(
        &restarted,
        "/api/v1/admin/platform/prompts/detail",
        json!({ "promptCode": "SYS_AGENT_DOMAIN_DISCOVERY" }),
    ).await;
    assert_eq!(refreshed["data"]["version"], json!(8));
    assert_eq!(refreshed["data"]["promptBody"], detail["data"]["promptBody"]);
    let refreshed_translations = refreshed["data"]["translations"].as_array().unwrap();
    for previous in translations {
        let current = refreshed_translations.iter()
            .find(|value| value["locale"] == previous["locale"]).unwrap();
        assert_eq!(current["promptBody"], previous["promptBody"]);
        assert_eq!(current["sourceVersion"], json!(8));
    }
}

#[tokio::test]
async fn removed_builtin_prompts_are_absent_from_the_global_catalog() {
    let (router, _) = prompt_app("retired-builtins-absent");
    for prompt_code in [
        "SYS_AGENT_FINAL_OUTPUT",
        "ROLE_MAIN_AGENT",
        "ROLE_EXPLORER",
        "ROLE_PROJECT_MANAGER",
        "ROLE_PRODUCT_MANAGER",
        "ROLE_FULL_STACK_DEVELOPER",
        "ROLE_TESTER",
        "SYS_AGENT_STEER_CONTINUITY",
        "SYS_MEMORY_SESSION_SUMMARY",
        "SYS_MEMORY_COMPACTION",
        "SYS_MEMORY_GOAL_JUDGE",
        "SYS_MEMORY_GOAL_JUDGE_RETRY",
        "SYS_RAG_TREE_SUMMARY",
        "SYS_TERMINAL_CHAT",
        "SYS_TERMINAL_EXPLAIN",
        "SYS_TERMINAL_COMMAND_GENERATE",
    ] {
        let response = authenticated_request(
            &router,
            Method::POST,
            "/api/v1/admin/platform/prompts/detail",
            Some(json!({ "promptCode": prompt_code })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{prompt_code}");
    }
}

#[tokio::test]
async fn retired_builtin_prompts_and_translations_are_deleted_from_existing_storage() {
    let database_url = temp_database_url("retired-builtins-existing");
    let retired = [
        ("SYS_AGENT_FINAL_OUTPUT", "agent.final-output", "SYSTEM"),
        ("ROLE_MAIN_AGENT", "agent.role.main", "AGENT_ROLE"),
        ("ROLE_EXPLORER", "agent.role.explorer", "AGENT_ROLE"),
        (
            "ROLE_PROJECT_MANAGER",
            "agent.role.project-manager",
            "AGENT_ROLE",
        ),
        (
            "ROLE_PRODUCT_MANAGER",
            "agent.role.product-manager",
            "AGENT_ROLE",
        ),
        (
            "ROLE_FULL_STACK_DEVELOPER",
            "agent.role.full-stack-developer",
            "AGENT_ROLE",
        ),
        ("ROLE_TESTER", "agent.role.tester", "AGENT_ROLE"),
    ];
    let config = common::prepared_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let router = common::app_with_state_setup(config, |_| {
        let connection = Connection::open(&database_url).unwrap();
        for (index, (code, key, category)) in retired.iter().enumerate() {
            connection.execute(
                "INSERT INTO prompts (id,prompt_code,display_name,category,prompt_body,enabled,version,created_at,updated_at,built_in,runtime_required,catalog_key)
                 VALUES (?1,?2,?2,?3,'Retired guidance',1,1,0,0,1,0,?4)",
                rusqlite::params![900000000000000001_i64 + index as i64, code, category, key],
            ).unwrap();
            for locale in ["zh-CN", "en-US"] {
                connection.execute(
                    "INSERT INTO prompt_translations (prompt_code,locale,display_name,prompt_body,updated_at)
                     VALUES (?1,?2,?1,'Retired guidance',0)",
                    [code, &locale],
                ).unwrap();
            }
        }
    });
    drop(router);
    // Reopening the application runs the same cleanup as a normal server restart.
    let router = app_with_config(AppConfig {
        database_url: database_url.to_string_lossy().into_owned(),
        ..AppConfig::default()
    });
    let connection = Connection::open(&database_url).unwrap();
    for (code, _, _) in retired {
        let response = authenticated_request(
            &router,
            Method::POST,
            "/api/v1/admin/platform/prompts/detail",
            Some(json!({ "promptCode": code })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{code}");
        let translations: i64 = connection
            .query_row(
                "SELECT count(*) FROM prompt_translations WHERE prompt_code=?1",
                [code],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(translations, 0, "{code}");
    }
}

#[tokio::test]
async fn template_analysis_remains_available_without_workspace_prompt_routes() {
    let (router, _) = prompt_app("prompt-template-analysis");
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

    let render = authenticated_request(
        &router,
        Method::POST,
        "/api/v1/prompts/render-template",
        Some(json!({ "promptBody": "Hello {{name}}", "variables": { "name": "Ada" } })),
    )
    .await;
    assert_eq!(render.status(), StatusCode::NOT_FOUND);
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
        repository
            .create_platform_variable(&PlatformVariable {
                id: SnowflakeId::new(734_500_000_000_001_000_i64 + index as i64),
                key: key.into(),
                value: value.into(),
                description: Some(format!("Prompt test {key}")),
                is_sensitive: false,
                value_nonce: None,
                value_tag: None,
                value_algorithm: None,
                value_key_version: None,
                created_at: "2026-08-24T00:00:00Z".into(),
                updated_at: "2026-08-24T00:00:00Z".into(),
            })
            .expect("prompt platform test variable should create");
    }
}

fn table_columns(connection: &Connection, table: &str) -> BTreeSet<String> {
    connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("database schema should be readable")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("database schema should be queryable")
        .collect::<Result<BTreeSet<_>, _>>()
        .expect("database columns should decode")
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
