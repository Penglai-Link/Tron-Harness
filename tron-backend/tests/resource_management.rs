mod common;

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tempfile::tempfile;
use tron_backend::{
    app::AppState,
    application::{
        agent::{BusinessToolCatalog, tool_record_to_effective_tool},
        resource::{
            ResourceBinaryAgentCommand, ResourceCommitBinary, ResourceCreateCommand,
            ResourcePageQuery, ResourceUpdateCommand,
        },
    },
    config::{AppConfig, ResourceConfig},
    domain::{
        agent::{
            AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
            AgentRunContext, AgentRunLimits, EffectiveSkill, ToolRequest,
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
fn resource_catalog_exposes_only_fixed_type_tools() {
    let _state = resource_state("catalog");
    let tool_codes = BusinessToolCatalog::list_tools(Some("resource"), None, "en-US")
        .into_iter()
        .map(|tool| tool.tool_code)
        .collect::<Vec<_>>();
    assert_eq!(tool_codes, vec!["resource.query", "resource.binary"]);
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
                kind: Some(ResourceKind::File),
                search: None,
                current: 1,
                size: 50,
            },
        )
        .expect("workspace query should project platform resources");
    assert_eq!(workspace_page.total, 1);
    assert_eq!(workspace_page.records[0].source_kind, ResourceSourceKind::Inherited);
    assert_eq!(
        workspace_page.records[0]
            .file
            .as_ref()
            .expect("file summary should be present")
            .byte_size,
        13
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
        tool_name: "resource.query".to_string(),
        input: json!({ "workspaceCode": WORKSPACE }),
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
    assert!(inaccessible_workspace.to_string().contains("not authorized"));

    let mut viewer_context = context.clone();
    viewer_context.user_code = Some(VIEWER.to_string());
    let viewer_denied = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource.binary".to_string(),
                input: json!({
                    "action": "upload",
                    "resourceCode": "viewer-file",
                    "sourcePath": "payload.txt"
                }),
            },
            &viewer_context,
        )
        .expect_err("binary upload requires workspace write permission");
    assert!(viewer_denied.to_string().contains("not authorized"));

    let invalid_binary = state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "resource.binary".to_string(),
                input: json!({
                    "action": "upload",
                    "resourceCode": "test-document"
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
                    "targetPath": "download.txt",
                    "base64": "cmVzb3VyY2UgcGF5bG9hZA=="
                }),
            },
            &context,
        )
        .expect_err("binary tools must reject undeclared raw payload fields");
    assert!(raw_binary_payload.to_string().contains("unknown field"));
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
