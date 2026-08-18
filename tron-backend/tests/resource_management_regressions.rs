mod common;

use std::{
    fs, thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::SqliteResourceRepository,
    app::AppState,
    application::resource::{
        ResourceBinaryAction, ResourceBinaryCommand, ResourceDefinitionDraftAction,
        ResourceDefinitionDraftCommand, ResourceDefinitionPublishCommand, ResourceDefinitionQuery,
        ResourceDraftAction, ResourceDraftCommand, ResourcePublishCommand, ResourceQuery,
        ResourceRepository,
    },
    config::{AppConfig, ResourceConfig},
    domain::resource::{
        DEFAULT_BINARY_STORAGE_PROFILE, Resource, ResourceActivationResult, ResourceDefinition,
        ResourceDefinitionKey, ResourceDefinitionStatus, ResourceError, ResourceKey,
        ResourceLifecycleStatus, ResourceScopeKind, ResourceValidationStatus,
    },
    shared::time::current_timestamp,
};

use common::prepared_config;

const ACTOR: &str = "USR-AUTH-0001";
const WORKSPACE: &str = "TRON_CORE";

#[test]
fn definition_query_reads_exact_draft_and_historical_versions() {
    let state = resource_state("definition-query-regression");
    let active = create_definition(
        &state,
        "QUERY_VIEW",
        json!({
            "type": "object",
            "properties": { "title": { "type": "string" } },
            "additionalProperties": false
        }),
    );
    publish_definition(&state, "QUERY_VIEW", &active);
    let draft = state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Update,
                definition_code: "QUERY_VIEW".to_string(),
                definition_version: Some(active.definition_version.clone()),
                expected_draft_revision: None,
                name: None,
                description: None,
                definition_status: None,
                allow_inheritance: None,
                attributes_schema: Some(json!({
                    "type": "object",
                    "properties": { "revision": { "type": "integer" } },
                    "additionalProperties": false
                })),
                binary_schema: None,
                binary_storage_profile: None,
            },
        )
        .expect("active definition should create a draft");

    let logical = query_definition(&state, "QUERY_VIEW", None);
    let local_draft = logical.records[0]
        .local_draft
        .as_ref()
        .expect("logical query should project the workspace draft");
    assert_eq!(local_draft.definition_version, draft.definition_version);
    assert!(local_draft.attributes_schema.is_some());

    let exact_draft = query_definition(&state, "QUERY_VIEW", Some(&draft.definition_version));
    assert_eq!(exact_draft.result_kind, "VERSION");
    assert_eq!(
        exact_draft.records[0].lifecycle_status,
        ResourceLifecycleStatus::Draft
    );
    assert!(exact_draft.records[0].attributes_schema.is_some());

    publish_definition(&state, "QUERY_VIEW", &draft);
    let historical = query_definition(&state, "QUERY_VIEW", Some(&active.definition_version));
    assert_eq!(
        historical.records[0].lifecycle_status,
        ResourceLifecycleStatus::Historical
    );
    assert!(historical.records[0].attributes_schema.is_some());

    let draft_only = create_definition(
        &state,
        "DRAFT_ONLY",
        json!({ "type": "object", "additionalProperties": false }),
    );
    let page = state
        .resources
        .query_definitions(
            ACTOR,
            ResourceDefinitionQuery {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                search: None,
                definition_code: None,
                definition_version: None,
                definition_status: None,
                lifecycle_status: None,
                include_history: false,
                current: 1,
                size: 50,
            },
        )
        .expect("definition page should succeed");
    assert!(page.records.iter().any(|record| {
        record.definition_code == "DRAFT_ONLY"
            && record.definition_version == draft_only.definition_version
            && record.lifecycle_status == ResourceLifecycleStatus::Draft
    }));
    assert_eq!(page.total, 11, "pages count logical definition codes");
}

#[test]
fn exact_resource_query_expands_local_draft_and_returns_draft_only_resource() {
    let state = resource_state("resource-query-regression");
    let definition = create_definition(
        &state,
        "RESOURCE_VIEW",
        json!({
            "type": "object",
            "required": ["title"],
            "properties": { "title": { "type": "string" } },
            "additionalProperties": false
        }),
    );
    let definition = state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Update,
                definition_code: "RESOURCE_VIEW".to_string(),
                definition_version: Some(definition.definition_version.clone()),
                expected_draft_revision: Some(definition.draft_revision),
                name: None,
                description: None,
                definition_status: None,
                allow_inheritance: None,
                attributes_schema: None,
                binary_schema: Some(json!({ "minCount": 0, "maxCount": 1 })),
                binary_storage_profile: None,
            },
        )
        .expect("definition draft should allow a binary mapping");
    publish_definition(&state, "RESOURCE_VIEW", &definition);
    let active = create_resource(
        &state,
        "resource-view",
        "RESOURCE_VIEW",
        json!({ "title": "active" }),
    );
    let work_dir = temp_directory("resource-query-binary");
    fs::write(work_dir.join("draft.txt"), b"draft binary")
        .expect("draft binary source should be writable");
    let uploaded = state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Upload,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: active.resource_code.clone(),
                resource_version: Some(active.resource_version.clone()),
                expected_draft_revision: Some(active.draft_revision),
                binary_name: "draft.txt".to_string(),
                source_path: Some("draft.txt".to_string()),
                target_path: None,
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect("draft binary upload should succeed");
    state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: active.resource_code.clone(),
                resource_version: active.resource_version.clone(),
                expected_draft_revision: uploaded.draft_revision,
                expected_active_resource_version: None,
            },
        )
        .expect("resource with binary should publish");
    let draft = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Update,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: active.resource_code.clone(),
                resource_version: Some(active.resource_version.clone()),
                expected_draft_revision: None,
                definition_code: None,
                name: Some("Draft resource".to_string()),
                description: Some("draft description".to_string()),
                attributes: Some(json!({ "title": "draft" })),
                allow_inheritance: Some(false),
            },
        )
        .expect("active resource should create a draft");

    let page = query_resource(&state, &active.resource_code);
    let local_draft = page.records[0]
        .local_draft
        .as_ref()
        .expect("exact query should include local draft details");
    assert_eq!(local_draft.resource_version, draft.resource_version);
    assert_eq!(local_draft.attributes, Some(json!({ "title": "draft" })));
    assert_eq!(local_draft.name.as_deref(), Some("Draft resource"));
    assert_eq!(
        local_draft
            .binaries
            .as_ref()
            .expect("detail draft should include binary names")[0]
            .binary_name,
        "draft.txt"
    );

    let page = query_resource_page(&state);
    let summary = page
        .records
        .iter()
        .find(|record| record.resource_code == active.resource_code)
        .expect("list query should include the effective resource");
    assert!(summary.attributes.is_none());
    assert!(summary.binaries.is_none());
    assert!(
        summary
            .local_draft
            .as_ref()
            .expect("list query should retain the draft summary")
            .attributes
            .is_none()
    );

    let draft_only = create_resource(
        &state,
        "draft-only-resource",
        "RESOURCE_VIEW",
        json!({ "title": "draft only" }),
    );
    let page = query_resource(&state, &draft_only.resource_code);
    assert_eq!(page.records.len(), 1);
    assert_eq!(
        page.records[0].lifecycle_status,
        Some(ResourceLifecycleStatus::Draft)
    );
    assert_eq!(
        page.records[0].attributes,
        Some(json!({ "title": "draft only" }))
    );
}

#[test]
fn variable_references_follow_draft7_applicators_and_recursive_refs() {
    let state = resource_state("variable-evaluation-regression");
    let applicator_schema = json!({
        "type": "object",
        "properties": {
            "mode": { "type": "string", "enum": ["PLAIN", "SECRET"] },
            "conditional": { "type": "string" },
            "arrayValues": {
                "type": "array",
                "items": { "type": "string", "format": "tron-variable-ref" }
            },
            "choice": {
                "oneOf": [
                    { "type": "string", "format": "tron-variable-ref" },
                    { "type": "integer" }
                ]
            },
            "anyChoice": {
                "anyOf": [
                    { "type": "string", "format": "tron-variable-ref" },
                    { "type": "integer" }
                ]
            },
            "composite": {
                "oneOf": [
                    {
                        "type": "object",
                        "required": ["apiKey"],
                        "properties": {
                            "apiKey": { "type": "string", "format": "tron-variable-ref" }
                        },
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "required": ["literal"],
                        "properties": { "literal": { "type": "integer" } },
                        "additionalProperties": false
                    }
                ]
            },
            "dependentKey": { "type": "string" },
            "dependentVariable": { "type": "string" },
            "contained": {
                "type": "array",
                "minItems": 1,
                "contains": { "type": "string", "format": "tron-variable-ref" }
            }
        },
        "patternProperties": {
            "^secret_": { "type": "string", "format": "tron-variable-ref" }
        },
        "additionalProperties": { "type": "string", "format": "tron-variable-ref" },
        "dependencies": {
            "dependentKey": {
                "properties": {
                    "dependentVariable": { "type": "string", "format": "tron-variable-ref" }
                }
            }
        },
        "allOf": [{
            "if": {
                "required": ["mode"],
                "properties": { "mode": { "const": "SECRET" } }
            },
            "then": {
                "properties": {
                    "conditional": { "type": "string", "format": "tron-variable-ref" }
                }
            },
            "else": { "properties": { "conditional": { "type": "string" } } }
        }]
    });
    let applicator = create_definition(&state, "VARIABLE_APPLICATORS", applicator_schema);
    publish_definition(&state, "VARIABLE_APPLICATORS", &applicator);
    let valid_attributes = json!({
        "mode": "SECRET",
        "conditional": "OPENAI_API_KEY",
        "secret_key": "OPENAI_API_KEY",
        "dynamic": "OPENAI_API_KEY",
        "arrayValues": ["OPENAI_API_KEY"],
        "choice": "OPENAI_API_KEY",
        "anyChoice": "OPENAI_API_KEY",
        "composite": { "apiKey": "OPENAI_API_KEY" },
        "dependentKey": "enabled",
        "dependentVariable": "OPENAI_API_KEY",
        "contained": ["OPENAI_API_KEY"]
    });
    let valid = create_resource(
        &state,
        "variable-valid",
        "VARIABLE_APPLICATORS",
        valid_attributes,
    );
    assert_eq!(valid.validation_status, ResourceValidationStatus::Valid);
    publish_resource(&state, &valid);

    let non_variable_branch = create_resource(
        &state,
        "variable-non-variable-branch",
        "VARIABLE_APPLICATORS",
        json!({
            "mode": "PLAIN",
            "conditional": "plain text",
            "choice": 1,
            "anyChoice": 1
        }),
    );
    assert_eq!(
        non_variable_branch.validation_status,
        ResourceValidationStatus::Valid
    );
    publish_resource(&state, &non_variable_branch);

    for (resource_code, attributes) in [
        (
            "variable-missing-pattern",
            json!({ "secret_key": "MISSING_VARIABLE" }),
        ),
        (
            "variable-missing-additional",
            json!({ "dynamic": "MISSING_VARIABLE" }),
        ),
        (
            "variable-missing-conditional",
            json!({ "mode": "SECRET", "conditional": "MISSING_VARIABLE" }),
        ),
        (
            "variable-missing-array",
            json!({ "arrayValues": ["MISSING_VARIABLE"] }),
        ),
        (
            "variable-missing-composite",
            json!({ "composite": { "apiKey": "MISSING_VARIABLE" } }),
        ),
        (
            "variable-missing-dependency",
            json!({ "dependentKey": "enabled", "dependentVariable": "MISSING_VARIABLE" }),
        ),
        (
            "variable-missing-contains",
            json!({ "contained": ["MISSING_VARIABLE"] }),
        ),
    ] {
        assert_unknown_variable_is_not_publishable(
            &state,
            resource_code,
            "VARIABLE_APPLICATORS",
            attributes,
        );
    }

    let recursive = create_definition(
        &state,
        "VARIABLE_RECURSIVE",
        json!({
            "$ref": "#/definitions/node",
            "definitions": {
                "node": {
                    "type": "object",
                    "properties": {
                        "secret": { "type": "string", "format": "tron-variable-ref" },
                        "children": {
                            "type": "array",
                            "items": { "$ref": "#/definitions/node" }
                        }
                    },
                    "additionalProperties": false
                }
            }
        }),
    );
    publish_definition(&state, "VARIABLE_RECURSIVE", &recursive);
    let recursive_resource = create_resource(
        &state,
        "variable-recursive",
        "VARIABLE_RECURSIVE",
        json!({ "children": [{ "children": [{ "secret": "OPENAI_API_KEY" }] }] }),
    );
    assert_eq!(
        recursive_resource.validation_status,
        ResourceValidationStatus::Valid
    );
    publish_resource(&state, &recursive_resource);
    assert_unknown_variable_is_not_publishable(
        &state,
        "variable-missing-recursive",
        "VARIABLE_RECURSIVE",
        json!({ "children": [{ "secret": "MISSING_VARIABLE" }] }),
    );
}

#[test]
fn builtin_schemas_accept_documented_samples_and_reject_invalid_fields() {
    let state = resource_state("builtin-schema-regression");
    let samples = vec![
        (
            "FILE",
            json!({ "fileName": "report.csv", "fileExtension": "csv", "sizeBytes": 8 }),
            json!({ "fileName": "report.csv", "sizeBytes": 8, "mimeType": "text/csv" }),
        ),
        (
            "IMAGE",
            json!({
                "images": [{
                    "binaryName": "cover.png", "fileName": "cover.png", "imageFormat": "PNG",
                    "sizeBytes": 8, "width": 16, "height": 9
                }]
            }),
            json!({
                "images": [{
                    "binaryName": "cover.png", "fileName": "cover.png", "imageFormat": "PNG",
                    "sizeBytes": 8, "mimeType": "image/png"
                }]
            }),
        ),
        (
            "PDF",
            json!({ "fileName": "report.pdf", "sizeBytes": 8, "pageCount": 1, "pdfVersion": "1.7" }),
            json!({ "fileName": "report.pdf", "sizeBytes": 8, "pageCount": 0 }),
        ),
        (
            "MYSQL_CONNECTION",
            json!({
                "connectionMode": "SINGLE",
                "endpoints": [{ "host": "mysql.internal", "port": 3306 }],
                "database": "production",
                "credentials": { "username": "MYSQL_USERNAME", "password": "MYSQL_PASSWORD" },
                "tls": { "mode": "DISABLED" },
                "properties": { "connectTimeoutMs": 5000 }
            }),
            json!({
                "connectionMode": "SINGLE",
                "endpoints": [
                    { "host": "mysql-1.internal", "port": 3306 },
                    { "host": "mysql-2.internal", "port": 3306 }
                ],
                "database": "production",
                "credentials": { "username": "MYSQL_USERNAME", "password": "MYSQL_PASSWORD" },
                "tls": { "mode": "DISABLED" }
            }),
        ),
        (
            "POSTGRESQL_CONNECTION",
            json!({
                "endpoints": [{ "host": "postgres.internal", "port": 5432 }],
                "database": "production",
                "credentials": { "username": "POSTGRES_USERNAME", "password": "POSTGRES_PASSWORD" },
                "targetSession": "READ_WRITE",
                "loadBalance": "DISABLED",
                "tls": { "mode": "VERIFY_FULL", "caCertificate": "POSTGRES_CA_CERT" }
            }),
            json!({
                "endpoints": [{ "host": "postgres.internal", "port": 5432 }],
                "database": "production",
                "credentials": { "username": "POSTGRES_USERNAME", "password": "POSTGRES_PASSWORD" },
                "targetSession": "READ_WRITE",
                "loadBalance": "DISABLED",
                "tls": { "mode": "REQUIRED", "clientCertificate": "POSTGRES_CLIENT_CERT" }
            }),
        ),
        (
            "REDIS_CONNECTION",
            json!({
                "topology": "SENTINEL",
                "endpoints": [{ "host": "sentinel.internal", "port": 26379 }],
                "database": 0,
                "credentials": { "username": "REDIS_USERNAME", "password": "REDIS_PASSWORD" },
                "tls": { "mode": "VERIFY_FULL", "caCertificate": "REDIS_CA_CERT" },
                "sentinel": {
                    "masterName": "mymaster",
                    "tls": { "mode": "VERIFY_FULL", "caCertificate": "REDIS_SENTINEL_CA_CERT" }
                }
            }),
            json!({
                "topology": "CLUSTER",
                "endpoints": [{ "host": "redis.internal", "port": 6379 }],
                "database": 1,
                "tls": { "mode": "DISABLED" }
            }),
        ),
        (
            "KAFKA_CONNECTION",
            json!({
                "brokers": [{ "host": "kafka.internal", "port": 9093 }],
                "securityProtocol": "SASL_SSL",
                "sasl": {
                    "mechanism": "SCRAM_SHA_256",
                    "username": "KAFKA_USERNAME",
                    "password": "KAFKA_PASSWORD"
                },
                "tls": { "mode": "VERIFY_FULL", "caCertificate": "KAFKA_CA_CERT" }
            }),
            json!({
                "brokers": [{ "host": "kafka.internal", "port": 9092 }],
                "securityProtocol": "PLAINTEXT",
                "sasl": {
                    "mechanism": "PLAIN",
                    "username": "KAFKA_USERNAME",
                    "password": "KAFKA_PASSWORD"
                }
            }),
        ),
        (
            "SERVER_CONNECTION",
            json!({ "host": "server.internal", "port": 22, "protocol": "Ssh" }),
            json!({ "host": "server.internal", "port": 22, "protocol": "Telnet" }),
        ),
        (
            "SERVER_ACCOUNT",
            json!({
                "serverCode": "SERVER_001",
                "username": "deploy",
                "authenticationType": "Password",
                "passwordVariableKey": "SERVER_PASSWORD"
            }),
            json!({
                "serverCode": "SERVER_001",
                "username": "deploy",
                "authenticationType": "Password",
                "privateKeyVariableKey": "SERVER_PRIVATE_KEY"
            }),
        ),
    ];
    for (code, valid, invalid) in samples {
        let schema = active_platform_schema(&state, code);
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .with_format("tron-variable-ref", |_| true)
            .should_validate_formats(true)
            .build(&schema)
            .expect("built-in schema should compile");
        assert!(
            validator.is_valid(&valid),
            "{code} valid sample should pass"
        );
        assert!(
            !validator.is_valid(&invalid),
            "{code} invalid sample should fail"
        );
    }
}

#[test]
fn builtin_seed_repair_is_versioned_idempotent_and_preserves_custom_rows() {
    let config = resource_config("seed-repair");
    let now = current_timestamp();
    let legacy = legacy_file_definition("File", &now);
    let mut repository = SqliteResourceRepository::connect(&config.database_url)
        .expect("legacy database should open");
    repository
        .insert_definition(&legacy)
        .expect("legacy seed should insert");
    repository
        .insert_resource(
            &Resource {
                key: ResourceKey {
                    scope_kind: ResourceScopeKind::Workspace,
                    scope_code: WORKSPACE.to_string(),
                    resource_code: "legacy-file-resource".to_string(),
                    resource_version: "v1-000".to_string(),
                },
                definition_key: legacy.key.clone(),
                based_on_resource_version: None,
                name: "Legacy file resource".to_string(),
                description: String::new(),
                attributes: json!({ "fileName": "legacy.txt", "sizeBytes": 1 }),
                allow_inheritance: true,
                draft_revision: 0,
                validation_status: ResourceValidationStatus::Valid,
                lifecycle_status: ResourceLifecycleStatus::Active,
                created_at: now.clone(),
                updated_at: now.clone(),
                published_at: Some(now.clone()),
            },
            &[],
        )
        .expect("historical resource binding should insert");
    drop(repository);

    let state = AppState::new(prepared_config(config)).expect("startup should repair legacy seed");
    let mut repository = SqliteResourceRepository::connect(&state.config.database_url)
        .expect("resource repository should open");
    let versions = repository
        .list_definitions()
        .expect("definitions should load")
        .into_iter()
        .filter(|definition| definition.key.definition_code == "FILE")
        .collect::<Vec<_>>();
    assert_eq!(versions.len(), 2);
    assert!(versions.iter().any(|definition| {
        definition.key.definition_version == "v1-000"
            && definition.lifecycle_status == ResourceLifecycleStatus::Historical
    }));
    let active = versions
        .iter()
        .find(|definition| definition.lifecycle_status == ResourceLifecycleStatus::Active)
        .expect("corrected seed should be active");
    assert!(active.attributes_schema["properties"]["fileExtension"].is_object());
    let legacy_resource = repository
        .list_resources()
        .expect("resources should load")
        .into_iter()
        .find(|resource| resource.key.resource_code == "legacy-file-resource")
        .expect("legacy resource should remain");
    assert_eq!(legacy_resource.definition_key, legacy.key);
    drop(repository);
    state
        .resources
        .bootstrap_seed_definitions()
        .expect("repeated bootstrap should be idempotent");
    let mut repository = SqliteResourceRepository::connect(&state.config.database_url)
        .expect("resource repository should reopen");
    assert_eq!(
        repository
            .list_definitions()
            .expect("definitions should load")
            .iter()
            .filter(|definition| definition.key.definition_code == "FILE")
            .count(),
        2
    );

    let customized_config = resource_config("seed-customized");
    let now = current_timestamp();
    let customized = legacy_file_definition("Customized file", &now);
    let mut customized_repository =
        SqliteResourceRepository::connect(&customized_config.database_url)
            .expect("customized database should open");
    customized_repository
        .insert_definition(&customized)
        .expect("customized row should insert");
    drop(customized_repository);
    let customized_state = AppState::new(prepared_config(customized_config))
        .expect("startup should preserve custom row");
    let mut customized_repository =
        SqliteResourceRepository::connect(&customized_state.config.database_url)
            .expect("customized repository should open");
    let files = customized_repository
        .list_definitions()
        .expect("definitions should load")
        .into_iter()
        .filter(|definition| definition.key.definition_code == "FILE")
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "Customized file");
}

#[test]
fn active_publish_retries_do_not_write_or_emit_audit_events() {
    let state = resource_state("active-retry-regression");
    let definition = create_definition(
        &state,
        "IDEMPOTENT",
        json!({
            "type": "object",
            "required": ["title"],
            "properties": { "title": { "type": "string" } },
            "additionalProperties": false
        }),
    );
    publish_definition(&state, "IDEMPOTENT", &definition);
    let mut repository = SqliteResourceRepository::connect(&state.config.database_url)
        .expect("resource repository should open");
    let before_definition = repository
        .list_definitions()
        .expect("definitions should load")
        .into_iter()
        .find(|value| value.key.definition_version == definition.definition_version)
        .expect("published definition should exist");
    let events_before = state.event_bus.stats_snapshot().accepted;
    thread::sleep(Duration::from_millis(2));
    let retry = state
        .resources
        .publish_definition(
            ACTOR,
            ResourceDefinitionPublishCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                definition_code: "IDEMPOTENT".to_string(),
                definition_version: definition.definition_version.clone(),
                expected_draft_revision: None,
                expected_active_definition_version: None,
            },
        )
        .expect("active definition retry should succeed");
    assert_eq!(
        retry.activation_result,
        ResourceActivationResult::AlreadyActive
    );
    let after_definition = repository
        .list_definitions()
        .expect("definitions should load")
        .into_iter()
        .find(|value| value.key.definition_version == definition.definition_version)
        .expect("published definition should remain");
    assert_eq!(
        before_definition.published_at,
        after_definition.published_at
    );
    assert_eq!(before_definition.updated_at, after_definition.updated_at);
    assert_eq!(state.event_bus.stats_snapshot().accepted, events_before);

    let resource = create_resource(
        &state,
        "idempotent-resource",
        "IDEMPOTENT",
        json!({ "title": "stable" }),
    );
    publish_resource(&state, &resource);
    let before_resource = repository
        .list_resources()
        .expect("resources should load")
        .into_iter()
        .find(|value| value.key.resource_version == resource.resource_version)
        .expect("published resource should exist");
    let events_before = state.event_bus.stats_snapshot().accepted;
    thread::sleep(Duration::from_millis(2));
    let retry = state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: resource.resource_version.clone(),
                expected_draft_revision: None,
                expected_active_resource_version: None,
            },
        )
        .expect("active resource retry should succeed");
    assert_eq!(
        retry.activation_result,
        ResourceActivationResult::AlreadyActive
    );
    let after_resource = repository
        .list_resources()
        .expect("resources should load")
        .into_iter()
        .find(|value| value.key.resource_version == resource.resource_version)
        .expect("published resource should remain");
    assert_eq!(before_resource.published_at, after_resource.published_at);
    assert_eq!(before_resource.updated_at, after_resource.updated_at);
    assert_eq!(state.event_bus.stats_snapshot().accepted, events_before);
}

fn create_definition(
    state: &AppState,
    definition_code: &str,
    attributes_schema: Value,
) -> tron_backend::application::resource::ResourceDefinitionDraftResult {
    state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Create,
                definition_code: definition_code.to_string(),
                definition_version: None,
                expected_draft_revision: None,
                name: Some(definition_code.to_string()),
                description: None,
                definition_status: Some(ResourceDefinitionStatus::Enabled),
                allow_inheritance: Some(true),
                attributes_schema: Some(attributes_schema),
                binary_schema: Some(json!({ "minCount": 0, "maxCount": 0 })),
                binary_storage_profile: None,
            },
        )
        .expect("definition draft should save")
}

fn publish_definition(
    state: &AppState,
    definition_code: &str,
    draft: &tron_backend::application::resource::ResourceDefinitionDraftResult,
) {
    state
        .resources
        .publish_definition(
            ACTOR,
            ResourceDefinitionPublishCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                definition_code: definition_code.to_string(),
                definition_version: draft.definition_version.clone(),
                expected_draft_revision: Some(draft.draft_revision),
                expected_active_definition_version: None,
            },
        )
        .expect("definition should publish");
}

fn create_resource(
    state: &AppState,
    resource_code: &str,
    definition_code: &str,
    attributes: Value,
) -> tron_backend::application::resource::ResourceDraftResult {
    state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Create,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource_code.to_string(),
                resource_version: None,
                expected_draft_revision: None,
                definition_code: Some(definition_code.to_string()),
                name: Some(resource_code.to_string()),
                description: None,
                attributes: Some(attributes),
                allow_inheritance: Some(true),
            },
        )
        .unwrap_or_else(|error| panic!("resource draft {resource_code} should save: {error:?}"))
}

fn publish_resource(
    state: &AppState,
    draft: &tron_backend::application::resource::ResourceDraftResult,
) {
    state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: draft.resource_code.clone(),
                resource_version: draft.resource_version.clone(),
                expected_draft_revision: Some(draft.draft_revision),
                expected_active_resource_version: None,
            },
        )
        .expect("resource should publish");
}

fn assert_unknown_variable_is_not_publishable(
    state: &AppState,
    resource_code: &str,
    definition_code: &str,
    attributes: Value,
) {
    let draft = create_resource(state, resource_code, definition_code, attributes);
    assert_eq!(draft.validation_status, ResourceValidationStatus::Invalid);
    let error = state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: draft.resource_code,
                resource_version: draft.resource_version,
                expected_draft_revision: Some(draft.draft_revision),
                expected_active_resource_version: None,
            },
        )
        .expect_err("unknown Variable references must block publication");
    assert_eq!(error, ResourceError::ValidationFailed);
}

fn query_definition(
    state: &AppState,
    code: &str,
    version: Option<&str>,
) -> tron_backend::application::resource::ResourceDefinitionPage {
    state
        .resources
        .query_definitions(
            ACTOR,
            ResourceDefinitionQuery {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                search: None,
                definition_code: Some(code.to_string()),
                definition_version: version.map(str::to_string),
                definition_status: None,
                lifecycle_status: None,
                include_history: false,
                current: 1,
                size: 20,
            },
        )
        .expect("definition query should succeed")
}

fn query_resource(
    state: &AppState,
    code: &str,
) -> tron_backend::application::resource::ResourcePage {
    state
        .resources
        .query_resources(
            ACTOR,
            ResourceQuery {
                workspace_code: Some(WORKSPACE.to_string()),
                search: None,
                definition_code: None,
                resource_code: Some(code.to_string()),
                resource_version: None,
                include_history: false,
                current: 1,
                size: 20,
            },
        )
        .expect("resource query should succeed")
}

fn query_resource_page(state: &AppState) -> tron_backend::application::resource::ResourcePage {
    state
        .resources
        .query_resources(
            ACTOR,
            ResourceQuery {
                workspace_code: Some(WORKSPACE.to_string()),
                search: None,
                definition_code: None,
                resource_code: None,
                resource_version: None,
                include_history: false,
                current: 1,
                size: 50,
            },
        )
        .expect("resource list query should succeed")
}

fn active_platform_schema(state: &AppState, definition_code: &str) -> Value {
    let mut repository = SqliteResourceRepository::connect(&state.config.database_url)
        .expect("resource repository should open");
    repository
        .list_definitions()
        .expect("definitions should load")
        .into_iter()
        .find(|definition| {
            definition.key.scope_kind == ResourceScopeKind::Platform
                && definition.key.definition_code == definition_code
                && definition.lifecycle_status == ResourceLifecycleStatus::Active
        })
        .map(|definition| definition.attributes_schema)
        .expect("active built-in definition should exist")
}

fn legacy_file_definition(name: &str, timestamp: &str) -> ResourceDefinition {
    ResourceDefinition {
        key: ResourceDefinitionKey {
            scope_kind: ResourceScopeKind::Platform,
            scope_code: "PLATFORM".to_string(),
            definition_code: "FILE".to_string(),
            definition_version: "v1-000".to_string(),
        },
        name: name.to_string(),
        description: "A managed file resource.".to_string(),
        definition_status: ResourceDefinitionStatus::Enabled,
        allow_inheritance: true,
        attributes_schema: json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "fileName": { "type": "string", "maxLength": 255 },
                "sizeBytes": { "type": "integer", "minimum": 0 }
            },
            "required": ["fileName", "sizeBytes"],
            "additionalProperties": false
        }),
        binary_schema: json!({ "minCount": 1, "maxCount": 1 }),
        binary_storage_profile: DEFAULT_BINARY_STORAGE_PROFILE.to_string(),
        draft_revision: 0,
        validation_status: ResourceValidationStatus::Valid,
        lifecycle_status: ResourceLifecycleStatus::Active,
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
        published_at: Some(timestamp.to_string()),
    }
}

fn resource_state(test_name: &str) -> AppState {
    AppState::new(prepared_config(resource_config(test_name)))
        .expect("resource test app should initialize")
}

fn resource_config(test_name: &str) -> AppConfig {
    let base = temp_directory(test_name);
    let mut config = AppConfig::default();
    config.database_url = base.join("tron.sqlite3").display().to_string();
    config.resource = ResourceConfig::with_default_root(base.join("objects").display().to_string());
    config
}

fn temp_directory(test_name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("tron-resource-regression-{test_name}-{nanos}"));
    std::fs::create_dir_all(&path).expect("temporary directory should be created");
    path
}
