mod common;

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tron_backend::{
    adapters::outbound::persistence::SqliteResourceRepository,
    app::AppState,
    application::{
        agent::{BusinessToolCatalog, tool_record_to_effective_tool},
        resource::{
            ResourceBinaryAction, ResourceBinaryCommand, ResourceDefinitionDraftAction,
            ResourceDefinitionDraftCommand, ResourceDefinitionPublishCommand,
            ResourceDefinitionQuery, ResourceDraftAction, ResourceDraftCommand,
            ResourcePublishCommand, ResourceQuery, ResourceRepository,
        },
    },
    config::{AppConfig, ResourceConfig},
    domain::{
        agent::{
            AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
            AgentRunContext, AgentRunLimits, EffectiveSkill, ToolRequest,
        },
        resource::{
            Resource, ResourceActivationResult, ResourceDefinitionKey, ResourceDefinitionStatus,
            ResourceError, ResourceKey, ResourceLifecycleStatus, ResourceScopeKind,
            ResourceSourceKind, ResourceValidationStatus,
        },
    },
    shared::time::current_timestamp,
};

use common::prepared_config;

const ACTOR: &str = "USR-AUTH-0001";
const WORKSPACE: &str = "TRON_CORE";

#[test]
fn resource_startup_seeds_definitions_backfills_permissions_and_catalog() {
    let state = resource_state("seed-and-catalog");
    let page = state
        .resources
        .query_definitions(
            ACTOR,
            ResourceDefinitionQuery {
                scope_kind: ResourceScopeKind::Platform,
                workspace_code: None,
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
        .expect("platform definition query should succeed");
    let codes = page
        .records
        .iter()
        .map(|record| record.definition_code.as_str())
        .collect::<Vec<_>>();
    for code in [
        "FILE",
        "IMAGE",
        "PDF",
        "MYSQL_CONNECTION",
        "POSTGRESQL_CONNECTION",
        "REDIS_CONNECTION",
        "KAFKA_CONNECTION",
    ] {
        assert!(codes.contains(&code), "seed {code} should be present");
    }

    assert!(
        state
            .roles
            .list_permission_domains()
            .contains(&"Resources".to_string())
    );
    let roles = state.roles.list_roles().expect("roles should load");
    let admin = roles
        .iter()
        .find(|role| role.code == "ADMIN")
        .expect("ADMIN should exist");
    assert_eq!(
        admin.permissions.get("Resources"),
        Some(&vec!["R".into(), "W".into(), "D".into(), "A".into()])
    );

    let tool_codes = BusinessToolCatalog::list_tools(Some("resource"), None, "en-US")
        .into_iter()
        .map(|tool| tool.tool_code)
        .collect::<Vec<_>>();
    assert_eq!(
        tool_codes,
        vec![
            "resource_definition.query",
            "resource_definition.draft",
            "resource_definition.publish",
            "resource.query",
            "resource.draft",
            "resource.publish",
            "resource.binary",
        ]
    );
}

#[test]
fn resources_support_invalid_drafts_binary_mapping_and_atomic_publication() {
    let state = resource_state("lifecycle-and-binary");
    let definition = state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Create,
                definition_code: "TEST_DOCUMENT".to_string(),
                definition_version: None,
                expected_draft_revision: None,
                name: Some("Test document".to_string()),
                description: Some("managed only by tests".to_string()),
                definition_status: Some(ResourceDefinitionStatus::Enabled),
                allow_inheritance: Some(true),
                attributes_schema: Some(json!({
                    "type": "object",
                    "properties": { "title": { "type": "string" } },
                    "required": ["title"],
                    "additionalProperties": false
                })),
                binary_schema: Some(json!({
                    "minCount": 0,
                    "maxCount": 1,
                    "maxSizeBytes": 16,
                    "namePattern": "^[a-z]+\\.txt$"
                })),
                binary_storage_profile: None,
            },
        )
        .expect("definition draft should be stored");
    assert_eq!(
        definition.validation_status,
        ResourceValidationStatus::Valid
    );
    let published_definition = state
        .resources
        .publish_definition(
            ACTOR,
            ResourceDefinitionPublishCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                definition_code: "TEST_DOCUMENT".to_string(),
                definition_version: definition.definition_version.clone(),
                expected_draft_revision: Some(definition.draft_revision),
                expected_active_definition_version: None,
            },
        )
        .expect("definition draft should publish");
    assert_eq!(
        published_definition.lifecycle_status,
        ResourceLifecycleStatus::Active
    );

    let definition_detail = state
        .resources
        .query_definitions(
            ACTOR,
            ResourceDefinitionQuery {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                search: None,
                definition_code: Some("TEST_DOCUMENT".to_string()),
                definition_version: None,
                definition_status: None,
                lifecycle_status: None,
                include_history: false,
                current: 1,
                size: 20,
            },
        )
        .expect("one definition query should return the selected schema");
    assert_eq!(definition_detail.result_kind, "DEFINITION");
    assert_eq!(definition_detail.records.len(), 1);
    assert!(definition_detail.records[0].attributes_schema.is_some());

    let resource = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Create,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: "test-document".to_string(),
                resource_version: None,
                expected_draft_revision: None,
                definition_code: Some("TEST_DOCUMENT".to_string()),
                name: Some("Release notes".to_string()),
                description: None,
                attributes: Some(json!({ "title": "Release notes" })),
                allow_inheritance: Some(true),
            },
        )
        .expect("resource draft should be stored");
    assert_eq!(resource.validation_status, ResourceValidationStatus::Valid);

    let work_dir = temp_directory("binary-workdir");
    let source = work_dir.join("payload.txt");
    fs::write(&source, b"resource payload").expect("source file should be writable");
    fs::write(work_dir.join("oversized.txt"), b"resource payload+")
        .expect("oversized source file should be writable");
    let invalid_name = state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Upload,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: Some(resource.resource_version.clone()),
                expected_draft_revision: Some(resource.draft_revision),
                binary_name: "payload.bin".to_string(),
                source_path: Some("payload.txt".to_string()),
                target_path: None,
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect_err("binary name patterns must be enforced");
    assert_eq!(invalid_name, ResourceError::InvalidInput);
    let oversized = state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Upload,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: Some(resource.resource_version.clone()),
                expected_draft_revision: Some(resource.draft_revision),
                binary_name: "oversized.txt".to_string(),
                source_path: Some("oversized.txt".to_string()),
                target_path: None,
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect_err("per-file binary limits must be enforced");
    assert_eq!(oversized, ResourceError::InvalidInput);
    let upload = state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Upload,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: Some(resource.resource_version.clone()),
                expected_draft_revision: Some(resource.draft_revision),
                binary_name: "payload.txt".to_string(),
                source_path: Some("payload.txt".to_string()),
                target_path: None,
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect("binary upload should succeed");
    assert_eq!(upload.byte_size, Some(16));
    assert!(
        !serde_json::to_string(&upload)
            .expect("result should serialize")
            .contains("local://")
    );

    state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Download,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: Some(resource.resource_version.clone()),
                expected_draft_revision: None,
                binary_name: "payload.txt".to_string(),
                source_path: None,
                target_path: Some("draft-download.txt".to_string()),
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect("local draft binary should be readable before publication");
    assert_eq!(
        fs::read(work_dir.join("draft-download.txt")).expect("draft download should exist"),
        b"resource payload"
    );
    let traversal = state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Upload,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: Some(resource.resource_version.clone()),
                expected_draft_revision: upload.draft_revision,
                binary_name: "other.txt".to_string(),
                source_path: Some("../payload.txt".to_string()),
                target_path: None,
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect_err("path traversal must be rejected");
    assert_eq!(traversal, ResourceError::PathDenied);

    let published_resource = state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: resource.resource_version.clone(),
                expected_draft_revision: upload.draft_revision,
                expected_active_resource_version: None,
            },
        )
        .expect("resource should publish after complete revalidation");
    assert_eq!(
        published_resource.lifecycle_status,
        ResourceLifecycleStatus::Active
    );

    let managed_root = state
        .resources
        .storage_roots()
        .into_iter()
        .next()
        .expect("local storage root should exist");
    let protected_target = state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Download,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: Some(resource.resource_version.clone()),
                expected_draft_revision: None,
                binary_name: "payload.txt".to_string(),
                source_path: None,
                target_path: Some(managed_root.join("leak.txt").display().to_string()),
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect_err("managed storage root must not be a download target");
    assert_eq!(protected_target, ResourceError::PathDenied);

    let download = state
        .resources
        .handle_resource_binary(
            ACTOR,
            ResourceBinaryCommand {
                action: ResourceBinaryAction::Download,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: resource.resource_code.clone(),
                resource_version: Some(resource.resource_version.clone()),
                expected_draft_revision: None,
                binary_name: "payload.txt".to_string(),
                source_path: None,
                target_path: Some("download.txt".to_string()),
                overwrite: false,
                agent_local_directory: Some(work_dir.display().to_string()),
            },
        )
        .expect("binary download should succeed");
    assert_eq!(download.byte_size, Some(16));
    assert_eq!(
        fs::read(work_dir.join("download.txt")).expect("download should exist"),
        b"resource payload"
    );

    let invalid = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Create,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: "invalid-file".to_string(),
                resource_version: None,
                expected_draft_revision: None,
                definition_code: Some("FILE".to_string()),
                name: Some("Incomplete file".to_string()),
                description: None,
                attributes: Some(json!({})),
                allow_inheritance: Some(true),
            },
        )
        .expect("invalid draft should still be stored");
    assert_eq!(invalid.validation_status, ResourceValidationStatus::Invalid);
}

#[test]
fn resource_definition_guards_require_names_and_reject_external_references() {
    let state = resource_state("definition-guards");
    let missing_name = state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Create,
                definition_code: "MISSING_NAME".to_string(),
                definition_version: None,
                expected_draft_revision: None,
                name: None,
                description: None,
                definition_status: None,
                allow_inheritance: None,
                attributes_schema: None,
                binary_schema: None,
                binary_storage_profile: None,
            },
        )
        .expect_err("a first definition draft requires a name");
    assert_eq!(missing_name, ResourceError::InvalidInput);

    let external_reference = state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Create,
                definition_code: "EXTERNAL_REFERENCE".to_string(),
                definition_version: None,
                expected_draft_revision: None,
                name: Some("External reference".to_string()),
                description: None,
                definition_status: None,
                allow_inheritance: None,
                attributes_schema: Some(json!({
                    "$ref": "https://schemas.example.invalid/resource.json"
                })),
                binary_schema: Some(json!({ "minCount": 0, "maxCount": 0 })),
                binary_storage_profile: None,
            },
        )
        .expect_err("remote schema references are a persistence-time security failure");
    assert_eq!(external_reference, ResourceError::InvalidSchema);
}

#[test]
fn resource_drafts_rebind_current_definition_and_survive_later_disablement() {
    let state = resource_state("definition-rebind-and-disable");
    let first_definition = create_test_definition(
        &state,
        "REBIND_DOCUMENT",
        json!({
            "type": "object",
            "required": ["title"],
            "properties": { "title": { "type": "string" } },
            "additionalProperties": false
        }),
    );
    publish_test_definition(&state, "REBIND_DOCUMENT", &first_definition);

    let first_resource = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Create,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: "rebind-document".to_string(),
                resource_version: None,
                expected_draft_revision: None,
                definition_code: Some("REBIND_DOCUMENT".to_string()),
                name: Some("Rebind document".to_string()),
                description: None,
                attributes: Some(json!({ "title": "Initial" })),
                allow_inheritance: Some(true),
            },
        )
        .expect("first resource draft should save");
    state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: first_resource.resource_code.clone(),
                resource_version: first_resource.resource_version.clone(),
                expected_draft_revision: Some(first_resource.draft_revision),
                expected_active_resource_version: None,
            },
        )
        .expect("first resource should publish");

    let second_definition = state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Update,
                definition_code: "REBIND_DOCUMENT".to_string(),
                definition_version: Some(first_definition.definition_version.clone()),
                expected_draft_revision: None,
                name: None,
                description: None,
                definition_status: None,
                allow_inheritance: None,
                attributes_schema: Some(json!({
                    "type": "object",
                    "required": ["revision"],
                    "properties": { "revision": { "type": "string" } },
                    "additionalProperties": false
                })),
                binary_schema: None,
                binary_storage_profile: None,
            },
        )
        .expect("an active definition should create an update draft");
    publish_test_definition(&state, "REBIND_DOCUMENT", &second_definition);

    let rebound = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Update,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: first_resource.resource_code.clone(),
                resource_version: Some(first_resource.resource_version.clone()),
                expected_draft_revision: None,
                definition_code: None,
                name: None,
                description: None,
                attributes: None,
                allow_inheritance: None,
            },
        )
        .expect("an active resource should create a re-bound draft");
    assert_eq!(rebound.draft_result, "DRAFT_CREATED");
    assert_eq!(
        rebound.definition_version,
        second_definition.definition_version
    );
    assert_eq!(rebound.validation_status, ResourceValidationStatus::Invalid);

    let duplicate = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Update,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: first_resource.resource_code.clone(),
                resource_version: Some(first_resource.resource_version.clone()),
                expected_draft_revision: None,
                definition_code: None,
                name: None,
                description: None,
                attributes: None,
                allow_inheritance: None,
            },
        )
        .expect_err("a logical resource may have only one local draft");
    assert_eq!(duplicate, ResourceError::Conflict);

    let stale_update = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Update,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: rebound.resource_code.clone(),
                resource_version: Some(rebound.resource_version.clone()),
                expected_draft_revision: Some(rebound.draft_revision + 1),
                definition_code: None,
                name: None,
                description: None,
                attributes: Some(json!({ "revision": "stale" })),
                allow_inheritance: None,
            },
        )
        .expect_err("a stale draft revision must not overwrite the draft");
    assert_eq!(stale_update, ResourceError::DraftRevisionConflict);

    let repaired = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Update,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: rebound.resource_code.clone(),
                resource_version: Some(rebound.resource_version.clone()),
                expected_draft_revision: Some(rebound.draft_revision),
                definition_code: None,
                name: None,
                description: None,
                attributes: Some(json!({ "revision": "2" })),
                allow_inheritance: None,
            },
        )
        .expect("the invalid draft should remain editable");
    assert_eq!(repaired.validation_status, ResourceValidationStatus::Valid);

    let disabled_definition = state
        .resources
        .draft_definition(
            ACTOR,
            ResourceDefinitionDraftCommand {
                scope_kind: ResourceScopeKind::Workspace,
                workspace_code: Some(WORKSPACE.to_string()),
                action: ResourceDefinitionDraftAction::Update,
                definition_code: "REBIND_DOCUMENT".to_string(),
                definition_version: Some(second_definition.definition_version.clone()),
                expected_draft_revision: None,
                name: None,
                description: None,
                definition_status: Some(ResourceDefinitionStatus::Disabled),
                allow_inheritance: None,
                attributes_schema: None,
                binary_schema: None,
                binary_storage_profile: None,
            },
        )
        .expect("definition disablement should create a later definition draft");
    publish_test_definition(&state, "REBIND_DOCUMENT", &disabled_definition);

    let published = state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: repaired.resource_code.clone(),
                resource_version: repaired.resource_version.clone(),
                expected_draft_revision: Some(repaired.draft_revision),
                expected_active_resource_version: None,
            },
        )
        .expect("a valid existing draft remains publishable after disablement");
    assert_eq!(published.lifecycle_status, ResourceLifecycleStatus::Active);

    let reactivated = state
        .resources
        .publish_resource(
            ACTOR,
            ResourcePublishCommand {
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: first_resource.resource_code,
                resource_version: first_resource.resource_version,
                expected_draft_revision: None,
                expected_active_resource_version: Some(published.resource_version),
            },
        )
        .expect("a historical resource should reactivate atomically");
    assert_eq!(
        reactivated.activation_result,
        ResourceActivationResult::HistoricalReactivated
    );
}

#[test]
fn resource_drafts_mark_missing_variable_references_invalid_and_resolve_platform_resources() {
    let state = resource_state("variable-and-platform-resolution");
    let definition = create_test_definition(
        &state,
        "VARIABLE_REFERENCE",
        json!({
            "type": "object",
            "required": ["secret"],
            "properties": {
                "secret": { "type": "string", "format": "tron-variable-ref" }
            },
            "additionalProperties": false
        }),
    );
    publish_test_definition(&state, "VARIABLE_REFERENCE", &definition);
    let invalid = state
        .resources
        .draft_resource(
            ACTOR,
            ResourceDraftCommand {
                action: ResourceDraftAction::Create,
                workspace_code: Some(WORKSPACE.to_string()),
                resource_code: "missing-variable".to_string(),
                resource_version: None,
                expected_draft_revision: None,
                definition_code: Some("VARIABLE_REFERENCE".to_string()),
                name: Some("Missing Variable".to_string()),
                description: None,
                attributes: Some(json!({ "secret": "MISSING_RESOURCE_SECRET" })),
                allow_inheritance: None,
            },
        )
        .expect("a missing Variable reference should remain a repairable draft error");
    assert_eq!(invalid.validation_status, ResourceValidationStatus::Invalid);
    assert!(
        invalid
            .validation_errors
            .iter()
            .all(|issue| !issue.message.contains("MISSING_RESOURCE_SECRET"))
    );

    let now = current_timestamp();
    let mut repository = SqliteResourceRepository::connect(&state.config.database_url)
        .expect("resource repository should reopen the test database");
    repository
        .insert_resource(
            &Resource {
                key: ResourceKey {
                    scope_kind: ResourceScopeKind::Platform,
                    scope_code: "PLATFORM".to_string(),
                    resource_code: "platform-file".to_string(),
                    resource_version: "v1-Z9a".to_string(),
                },
                definition_key: ResourceDefinitionKey {
                    scope_kind: ResourceScopeKind::Platform,
                    scope_code: "PLATFORM".to_string(),
                    definition_code: "FILE".to_string(),
                    definition_version: "v1-000".to_string(),
                },
                based_on_resource_version: None,
                name: "Platform file".to_string(),
                description: String::new(),
                attributes: json!({ "fileName": "platform.txt", "sizeBytes": 0 }),
                allow_inheritance: true,
                draft_revision: 0,
                validation_status: ResourceValidationStatus::Valid,
                lifecycle_status: ResourceLifecycleStatus::Active,
                created_at: now.clone(),
                updated_at: now.clone(),
                published_at: Some(now),
            },
            &[],
        )
        .expect("a platform resource snapshot should be insertable for resolution coverage");

    let page = state
        .resources
        .query_resources(
            ACTOR,
            ResourceQuery {
                workspace_code: Some(WORKSPACE.to_string()),
                search: None,
                definition_code: None,
                resource_code: Some("platform-file".to_string()),
                resource_version: None,
                include_history: false,
                current: 1,
                size: 20,
            },
        )
        .expect("effective workspace query should include active platform resources");
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].source_kind, ResourceSourceKind::Platform);
}

#[test]
fn resource_tools_require_actor_and_enforce_scope_and_action_shapes() {
    let state = resource_state("tool-authorization");
    let tools = BusinessToolCatalog::list_tools(Some("resource"), None, "en-US")
        .into_iter()
        .map(tool_record_to_effective_tool)
        .collect::<Vec<_>>();
    let context = resource_tool_context(tools);
    let request = ToolRequest {
        tool_name: "resource_definition.query".to_string(),
        input: json!({ "scopeKind": "PLATFORM" }),
    };
    let output = state
        .agent_tool_registry
        .execute_allowed(request.clone(), &context)
        .expect("assigned resource tool should execute for an authenticated admin");
    let json: Value = serde_json::from_str(&output.observation.output).expect("tool output JSON");
    assert!(json["records"].as_array().is_some());

    let mut missing_actor = context.clone();
    missing_actor.user_code = None;
    let error = state
        .agent_tool_registry
        .execute_allowed(request, &missing_actor)
        .expect_err("missing userCode must be rejected");
    assert!(error.to_string().contains("authenticated user"));

    let invalid_scope = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_definition.query".to_string(),
                input: json!({ "scopeKind": "PLATFORM", "workspaceCode": WORKSPACE }),
            },
            &context,
        )
        .expect_err("platform scope must reject workspaceCode");
    assert!(
        invalid_scope
            .to_string()
            .contains("workspaceCode is not allowed")
    );

    let mut viewer_context = context.clone();
    viewer_context.user_code = Some("USR-0007".to_string());
    state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_definition.query".to_string(),
                input: json!({ "scopeKind": "PLATFORM" }),
            },
            &viewer_context,
        )
        .expect("a viewer may read platform definition metadata");
    let platform_write = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_definition.draft".to_string(),
                input: json!({
                    "scopeKind": "PLATFORM",
                    "action": "create",
                    "definitionCode": "VIEWER_DENIED",
                    "name": "Viewer denied"
                }),
            },
            &viewer_context,
        )
        .expect_err("platform definition mutation requires Resources:A");
    assert!(platform_write.to_string().contains("not authorized"));

    let inaccessible_workspace = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource.query".to_string(),
                input: json!({ "workspaceCode": "ENTERPRISE" }),
            },
            &context,
        )
        .expect_err("Resources permission does not bypass workspace access");
    assert!(
        inaccessible_workspace
            .to_string()
            .contains("not authorized")
    );

    let invalid_binary = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource.binary".to_string(),
                input: json!({
                    "action": "upload",
                    "resourceCode": "test-document",
                    "resourceVersion": "v1-A1B",
                    "expectedDraftRevision": 1,
                    "binaryName": "payload.txt"
                }),
            },
            &context,
        )
        .expect_err("upload without sourcePath must be rejected before file access");
    assert!(invalid_binary.to_string().contains("sourcePath"));

    let raw_binary_payload = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource.binary".to_string(),
                input: json!({
                    "action": "download",
                    "resourceCode": "test-document",
                    "binaryName": "payload.txt",
                    "targetPath": "download.txt",
                    "base64": "cmVzb3VyY2UgcGF5bG9hZA=="
                }),
            },
            &context,
        )
        .expect_err("binary tools must reject undeclared raw payload fields");
    assert!(raw_binary_payload.to_string().contains("unknown field"));
}

fn create_test_definition(
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
        .expect("test definition draft should save")
}

fn publish_test_definition(
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
        .expect("test definition draft should publish");
}

fn resource_state(test_name: &str) -> AppState {
    let base = temp_directory(test_name);
    let mut config = AppConfig::default();
    config.database_url = base.join("tron.sqlite3").display().to_string();
    config.resource = ResourceConfig::with_default_root(base.join("objects").display().to_string());
    AppState::new(prepared_config(config)).expect("resource test app should initialize")
}

fn resource_tool_context(
    tools: Vec<tron_backend::domain::agent::EffectiveTool>,
) -> AgentRunContext {
    AgentRunContext {
        conversation_code: Some("CONV-RESOURCE".to_string()),
        run_code: "RUN-RESOURCE".to_string(),
        workspace_code: WORKSPACE.to_string(),
        user_code: Some(ACTOR.to_string()),
        locale: "en-US".to_string(),
        request_id: None,
        stream_key: None,
        agent_kind: AgentKind::React,
        system_agent_session_id: None,
        model: AgentModelConfig {
            model_code: "MODEL-TEST".to_string(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api_path: None,
            api_key: "test".to_string(),
            model_name: "test".to_string(),
            thinking_enabled: false,
            responses_stateful_enabled: false,
            temperature: None,
            max_tokens: Some(128),
            context_window_tokens: Some(8_192),
            top_p: None,
            max_retries: Some(0),
        },
        variables: Vec::new(),
        skills: vec![EffectiveSkill {
            skill_name: "resource-test".to_string(),
            description: "resource tool test".to_string(),
            instruction: None,
            tool_names: tools.iter().map(|tool| tool.tool_name.clone()).collect(),
        }],
        tools,
        effective_tools: Vec::new(),
        tool_exposure: Default::default(),
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("manage resources")],
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-07-16T00:00:00Z".to_string(),
    }
}

fn temp_directory(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("tron-resource-{test_name}-{nanos}"));
    fs::create_dir_all(&path).expect("temporary directory should be created");
    path
}
