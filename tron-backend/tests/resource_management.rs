mod common;

use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tempfile::tempfile;
use tron_backend::{
    app::AppState,
    application::{
        agent::{BusinessToolCatalog, register_default_agent_tools, tool_record_to_effective_tool},
        resource::{
            ResourceBinaryAgentCommand, ResourceCommitBinary, ResourceConnectionTarget,
            ResourceConnectionVerificationFuture, ResourceConnectionVerifier,
            ResourceCreateCommand, ResourcePageQuery, ResourceUpdateCommand,
        },
    },
    config::{AppConfig, ResourceConfig},
    domain::{
        agent::{
            AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
            AgentRunContext, AgentRunLimits, EffectiveSkill, ToolRegistry, ToolRequest,
        },
        resource::{ResourceKind, ResourceSourceKind},
    },
};

use common::prepared_config;

const ACTOR: &str = "USR-AUTH-0001";
const VIEWER: &str = "USR-0007";
const WORKSPACE: &str = "TRON_CORE";
const PARENT_WORKSPACE: &str = "PLATFORM";
const PARENT_ACTOR: &str = "USR-LARGE-ID";

#[test]
fn resource_catalog_exposes_full_resource_lifecycle_tools() {
    let _state = resource_state("catalog");
    let tool_codes = BusinessToolCatalog::list_tools(Some("resource"), None, "en-US")
        .into_iter()
        .map(|tool| tool.tool_code)
        .collect::<Vec<_>>();
    assert_eq!(
        tool_codes,
        vec![
            "platform.resource.query",
            "platform.resource.get",
            "platform.resource.create",
            "platform.resource.update",
            "platform.resource.delete",
            "platform.resource.binary",
            "resource.query",
            "resource.get",
            "resource.create",
            "resource.update",
            "resource.delete",
            "resource.binary",
            "resource.data_source.create",
            "resource.data_source.update",
            "resource.server.create",
            "resource.server_account.create",
        ]
    );
}

#[test]
fn file_resources_support_direct_crud_and_platform_inheritance() {
    let state = resource_state("file-crud");

    let record = state
        .resources
        .create_file_resource(
            PARENT_ACTOR,
            ResourceCreateCommand {
                workspace_code: PARENT_WORKSPACE.to_string(),
                resource_code: "platform-template".to_string(),
                kind: ResourceKind::File,
                name: "Platform template".to_string(),
                description: "shared template".to_string(),
                allow_inheritance: true,
            },
            Some(ResourceCommitBinary {
                source: temp_file(b"template body"),
                file_name: "template.md".to_string(),
            }),
        )
        .expect("platform file should be created");
    assert_eq!(record.kind, ResourceKind::File);

    let workspace_page = state
        .resources
        .query_resources(
            ACTOR,
            ResourcePageQuery {
                workspace_code: WORKSPACE.to_string(),
                kinds: vec![ResourceKind::File],
                source_kind: None,
                search: None,
                current: 1,
                size: 50,
            },
        )
        .expect("workspace query should project platform resources");
    assert_eq!(workspace_page.total, 1);
    assert_eq!(
        workspace_page.records[0].source_kind,
        ResourceSourceKind::Inherited
    );
    assert_eq!(
        workspace_page.records[0]
            .file
            .as_ref()
            .expect("file summary should be present")
            .byte_size,
        13
    );

    let agent_directory = temp_directory("inherited-agent-download");
    let downloaded = state
        .resources
        .handle_agent_binary_download(
            ACTOR,
            WORKSPACE,
            ResourceBinaryAgentCommand {
                resource_code: "platform-template".to_string(),
                source_path: None,
                target_path: Some("platform-template.md".to_string()),
                overwrite: false,
                agent_local_directory: Some(agent_directory.display().to_string()),
            },
        )
        .expect("agent should download an inherited file");
    assert_eq!(downloaded.action, "DOWNLOADED");
    assert_eq!(
        fs::read_to_string(agent_directory.join("platform-template.md"))
            .expect("downloaded inherited file should be readable"),
        "template body",
    );

    state
        .resources
        .create_file_resource(
            ACTOR,
            ResourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "local-note".to_string(),
                kind: ResourceKind::File,
                name: "Local note".to_string(),
                description: String::new(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temp_file(b"local"),
                file_name: "note.txt".to_string(),
            }),
        )
        .expect("workspace file should be created");

    let updated = state
        .resources
        .update_resource(
            ACTOR,
            ResourceUpdateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "local-note".to_string(),
                name: Some("Local note v2".to_string()),
                description: Some("updated".to_string()),
                allow_inheritance: None,
            },
        )
        .expect("workspace file should be updated");
    assert_eq!(updated.resource_code, "local-note");

    let detail = state
        .resources
        .get_resource(
            ACTOR,
            tron_backend::application::resource::ResourceGetQuery {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "local-note".to_string(),
            },
        )
        .expect("detail should resolve");
    assert_eq!(detail.name, "Local note v2");

    let deleted = state
        .resources
        .delete_resource(
            ACTOR,
            tron_backend::application::resource::ResourceDeleteCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "local-note".to_string(),
            },
        )
        .expect("workspace file should be deleted");
    assert!(deleted.deleted);
}

#[test]
fn duplicate_resource_codes_are_rejected_within_a_scope() {
    let state = resource_state("duplicate");
    let command = || ResourceCreateCommand {
        workspace_code: WORKSPACE.to_string(),
        resource_code: "duplicate-code".to_string(),
        kind: ResourceKind::File,
        name: "Duplicate".to_string(),
        description: String::new(),
        allow_inheritance: false,
    };
    state
        .resources
        .create_file_resource(
            ACTOR,
            command(),
            Some(ResourceCommitBinary {
                source: temp_file(b"one"),
                file_name: "one.txt".to_string(),
            }),
        )
        .expect("first create should succeed");
    let second = state.resources.create_file_resource(
        ACTOR,
        command(),
        Some(ResourceCommitBinary {
            source: temp_file(b"two"),
            file_name: "two.txt".to_string(),
        }),
    );
    assert_eq!(
        second.err(),
        Some(tron_backend::domain::resource::ResourceError::Conflict)
    );
}

#[test]
fn agent_binary_tools_upload_and_download_workspace_files() {
    let state = resource_state("agent-binary");
    let base = temp_directory("agent-binary-payload");
    let source_path = base.join("payload.txt");
    fs::write(&source_path, "resource payload").expect("payload should be written");

    state
        .resources
        .create_file_resource(
            ACTOR,
            ResourceCreateCommand {
                workspace_code: WORKSPACE.to_string(),
                resource_code: "agent-file".to_string(),
                kind: ResourceKind::File,
                name: "Agent file".to_string(),
                description: String::new(),
                allow_inheritance: false,
            },
            Some(ResourceCommitBinary {
                source: temp_file(b"initial"),
                file_name: "initial.txt".to_string(),
            }),
        )
        .expect("workspace file should exist for the agent");

    let uploaded = state
        .resources
        .handle_agent_binary_upload(
            ACTOR,
            WORKSPACE,
            ResourceBinaryAgentCommand {
                resource_code: "agent-file".to_string(),
                source_path: Some(source_path.display().to_string()),
                target_path: None,
                overwrite: false,
                agent_local_directory: Some(base.display().to_string()),
            },
        )
        .expect("agent upload should succeed");
    assert_eq!(uploaded.action, "UPLOADED");
    assert_eq!(uploaded.byte_size, Some(16));

    let target_path = base.join("downloaded.txt");
    let downloaded = state
        .resources
        .handle_agent_binary_download(
            ACTOR,
            WORKSPACE,
            ResourceBinaryAgentCommand {
                resource_code: "agent-file".to_string(),
                source_path: None,
                target_path: Some("downloaded.txt".to_string()),
                overwrite: false,
                agent_local_directory: Some(base.display().to_string()),
            },
        )
        .expect("agent download should succeed");
    assert_eq!(downloaded.action, "DOWNLOADED");
    assert_eq!(
        fs::read_to_string(&target_path).expect("downloaded file should be readable"),
        "resource payload"
    );
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
        tool_name: "resource_query".to_string(),
        input: json!({ "workspaceCode": WORKSPACE }),
        invocation_id: None,
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

    // The active local profile can access every active workspace. Removed
    // legacy bearer-role names must not reintroduce the old ACL model.
    let mut profile_context = context.clone();
    profile_context.user_code = Some(VIEWER.to_string());
    state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_query".into(),
                input: json!({"workspaceCode":"ENTERPRISE"}),
                invocation_id: None,
            },
            &profile_context,
        )
        .expect("local profile may query active workspaces");
    let invalid_workspace = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_query".into(),
                input: json!({"workspaceCode":"MISSING-WORKSPACE"}),
                invocation_id: None,
            },
            &profile_context,
        )
        .unwrap_err();
    assert!(invalid_workspace.to_string().contains("workspace"));

    let invalid_binary = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_binary".to_string(),
                input: json!({
                    "action": "upload",
                    "resourceCode": "test-document"
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect_err("upload without sourcePath must be rejected before file access");
    assert!(invalid_binary.to_string().contains("sourcePath"));

    let raw_binary_payload = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_binary".to_string(),
                input: json!({
                    "action": "download",
                    "resourceCode": "test-document",
                    "targetPath": "download.txt",
                    "base64": "cmVzb3VyY2UgcGF5bG9hZA=="
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect_err("binary tools must reject undeclared raw payload fields");
    assert!(raw_binary_payload.to_string().contains("unknown field"));
}

#[test]
fn agent_resource_tools_create_all_supported_resource_families() {
    let state = resource_state("agent-resource-lifecycle");
    let base = temp_directory("agent-resource-payload");
    fs::write(base.join("release-notes.md"), "release notes")
        .expect("agent source file should be created");
    let tools = BusinessToolCatalog::list_tools(Some("resource"), None, "en-US")
        .into_iter()
        .map(tool_record_to_effective_tool)
        .collect::<Vec<_>>();
    let mut context = resource_tool_context(tools);
    context.execution_scope.local_directory = Some(base.display().to_string());

    let created_file = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_create".to_string(),
                input: json!({
                    "resourceCode": "agent-release-notes",
                    "kind": "File",
                    "name": "Agent release notes",
                    "allowInheritance": false,
                    "sourcePath": "release-notes.md"
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should create a file resource");
    let created_file: Value = serde_json::from_str(&created_file.observation.output)
        .expect("file create result should be JSON");
    assert_eq!(created_file["kind"], "File");

    let file_detail = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_get".to_string(),
                input: json!({ "resourceCode": "agent-release-notes" }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should read a file resource");
    let file_detail: Value =
        serde_json::from_str(&file_detail.observation.output).expect("file detail should be JSON");
    assert_eq!(file_detail["file"]["fileName"], "release-notes.md");

    state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_update".to_string(),
                input: json!({
                    "resourceCode": "agent-release-notes",
                    "name": "Agent release notes v2"
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should update file metadata");

    let data_source = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_data_source_create".to_string(),
                input: json!({
                    "resourceCode": "agent-mysql",
                    "kind": "MysqlConnection",
                    "name": "Agent MySQL",
                    "attributes": {
                        "connectionMode": "SINGLE",
                        "endpoints": [{ "host": "mysql.internal", "port": 3306 }],
                        "database": "tron",
                        "credentials": {
                            "username": "tron_user",
                            "password": { "source": "DIRECT", "value": "database-password" }
                        },
                        "tls": { "mode": "DISABLED" }
                    },
                    "allowInheritance": false
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should create a data source");
    let data_source: Value = serde_json::from_str(&data_source.observation.output)
        .expect("data-source create result should be JSON");
    assert_eq!(data_source["kind"], "MysqlConnection");
    assert!(!data_source.to_string().contains("database-password"));

    state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_data_source_update".to_string(),
                input: json!({
                    "resourceCode": "agent-mysql",
                    "description": "Agent-owned data source"
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should update a data source");

    let created_server = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_server_create".to_string(),
                input: json!({
                    "serverCode": "agent-bastion",
                    "name": "Agent bastion",
                    "host": "bastion.internal",
                    "port": 22,
                    "allowInheritance": false,
                    "initialAccount": {
                        "accountCode": "agent-bastion-ops",
                        "name": "Agent operations",
                        "username": "ops",
                        "authenticationType": "Password",
                        "passwordVariableKey": "OPENAI_API_KEY",
                        "allowInheritance": false
                    }
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should create a server and initial account");
    let created_server: Value = serde_json::from_str(&created_server.observation.output)
        .expect("server create result should be JSON");
    assert_eq!(created_server["server"]["kind"], "ServerConnection");
    assert_eq!(created_server["initialAccount"]["kind"], "ServerAccount");

    let created_account = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_server_account_create".to_string(),
                input: json!({
                    "serverCode": "agent-bastion",
                    "account": {
                        "accountCode": "agent-bastion-deploy",
                        "name": "Agent deploy",
                        "username": "deploy",
                        "authenticationType": "PrivateKey",
                        "privateKeyVariableKey": "OPENAI_API_KEY",
                        "allowInheritance": false
                    }
                }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should create a separate server account");
    let created_account: Value = serde_json::from_str(&created_account.observation.output)
        .expect("server-account create result should be JSON");
    assert_eq!(created_account["kind"], "ServerAccount");

    let data_source_detail = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_get".to_string(),
                input: json!({ "resourceCode": "agent-mysql" }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should read a data source");
    let data_source_detail: Value = serde_json::from_str(&data_source_detail.observation.output)
        .expect("data-source detail should be JSON");
    assert_eq!(
        data_source_detail["attributes"]["credentials"]["password"]["value"],
        Value::Null
    );

    let deleted = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource_delete".to_string(),
                input: json!({ "resourceCode": "agent-bastion-deploy" }),
                invocation_id: None,
            },
            &context,
        )
        .expect("agent should delete a local server account");
    let deleted: Value =
        serde_json::from_str(&deleted.observation.output).expect("delete result should be JSON");
    assert_eq!(deleted["deleted"], true);
}

#[derive(Clone)]
struct SuccessfulConnectionVerifier;

impl ResourceConnectionVerifier for SuccessfulConnectionVerifier {
    fn verify(
        &self,
        _target: ResourceConnectionTarget,
    ) -> ResourceConnectionVerificationFuture<'_> {
        Box::pin(async move { Ok(()) })
    }
}

fn resource_state(test_name: &str) -> AppState {
    let base = temp_directory(test_name);
    let mut config = AppConfig::default();
    config.database_url = base.join("tron.sqlite3").display().to_string();
    config.resource = ResourceConfig::with_default_root(base.join("objects").display().to_string());
    let mut state =
        AppState::new(prepared_config(config)).expect("resource test app should initialize");
    state.resources = state
        .resources
        .clone()
        .with_connection_verifier(Arc::new(SuccessfulConnectionVerifier));
    let agent_tool_registry = Arc::new(ToolRegistry::default());
    register_default_agent_tools(
        agent_tool_registry.as_ref(),
        state.workspaces.clone(),
        state.apps.clone(),
        state.automations.clone(),
        state.tasks.clone(),
        state.models.clone(),
        state.skills.clone(),
        state.variables.clone(),
        state.resources.clone(),
        state.prompts.clone(),
        state.mcp.clone(),
        state.memory.clone(),
        state.short_term_memory.clone(),
    )
    .expect("resource test tools should register");
    state.agent_tool_registry = agent_tool_registry;
    state
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
            reasoning_effort: None,
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
        auto_model_route: None,
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

fn temp_file(payload: &[u8]) -> std::fs::File {
    let mut file = tempfile().expect("tempfile should create");
    std::io::Write::write_all(&mut file, payload).expect("payload should write");
    std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(0)).expect("file should rewind");
    file
}
