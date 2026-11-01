mod common;

use serde_json::{Value, json};
use tron_backend::{
    app::AppState,
    application::agent::{BusinessToolCatalog, tool_record_to_effective_tool},
    config::{AppConfig, ResourceConfig},
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, EffectiveSkill, ToolRequest,
    },
};

const WORKSPACE: &str = "TRON_CORE";
const ACTOR: &str = "USR-AUTH-0001";

struct Fixture {
    state: AppState,
    context: AgentRunContext,
    _directory: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        config.database_url = directory.path().join("tron.sqlite3").display().to_string();
        config.graph_storage_path = directory
            .path()
            .join("tron.graph.grafeo")
            .display()
            .to_string();
        config.resource = ResourceConfig::with_default_root(
            directory.path().join("resources").display().to_string(),
        );
        let skill_root = directory.path().join("skills");
        let skill = skill_root.join(".agents/skills/resource-test");
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), "---\nname: resource-test\ndescription: resource tool test\n---\nResource test instructions.").unwrap();
        config.skills_registry_root = skill_root.display().to_string();
        let state = AppState::new(common::prepared_config(config)).unwrap();
        state.knowledge_bases.stop_ingestion_workers();
        let tools = BusinessToolCatalog::list_tools(None, None, "en-US")
            .into_iter()
            .map(tool_record_to_effective_tool)
            .collect();
        Self {
            state,
            context: tool_context(tools),
            _directory: directory,
        }
    }
    fn call(&self, name: &str, input: Value) -> Value {
        let out = self
            .state
            .agent_tool_registry
            .execute_allowed(
                ToolRequest {
                    tool_name: name.into(),
                    input,
                    invocation_id: None,
                },
                &self.context,
            )
            .unwrap_or_else(|error| panic!("{name} failed: {error}"));
        serde_json::from_str(&out.observation.output).unwrap()
    }
}

#[test]
fn skill_tools_only_show_assigned_skills() {
    let f = Fixture::new();
    f.state
        .skills
        .replace_agent_skills("main-agent", &["resource-test".into()])
        .unwrap();
    assert!(
        f.call("skill_page", json!({"search":"resource-test"}))
            .to_string()
            .contains("resource-test")
    );
    assert!(
        f.call("skill_view", json!({"skillName":"resource-test"}))
            .to_string()
            .contains("resource-test")
    );
    f.state
        .skills
        .replace_agent_skills("main-agent", &[])
        .unwrap();
    assert!(
        !f.call("skill_page", json!({}))
            .to_string()
            .contains("resource-test")
    );
    let denied = f.state.agent_tool_registry.execute_allowed(
        ToolRequest {
            tool_name: "skill_view".into(),
            input: json!({"skillName":"resource-test"}),
            invocation_id: None,
        },
        &f.context,
    );
    assert!(denied.is_err());
}

#[test]
fn workspace_tools_reject_missing_foreign_and_dude_scopes() {
    use tron_backend::domain::agent::AgentCollaborationOwner;
    let f = Fixture::new();
    let mut dude = f.context.clone();
    dude.agent_code = Some("main-dude".into());
    dude.collaboration_owner = Some(AgentCollaborationOwner {
        kind: "DUDE".into(),
        code: "main-dude".into(),
        origin_session_code: dude.conversation_code.clone(),
        task_code: None,
    });
    let mut checked = 0;
    for tool in &f.context.tools {
        if tool
            .input_schema
            .pointer("/properties/workspaceCode")
            .is_none()
        {
            continue;
        }
        for (context, input) in [
            (&f.context, json!({})),
            (&f.context, json!({"workspaceCode":"OTHER"})),
            (&dude, json!({"workspaceCode":WORKSPACE})),
        ] {
            let request = ToolRequest {
                tool_name: tool.tool_name.clone(),
                input,
                invocation_id: None,
            };
            let error = f
                .state
                .agent_tool_registry
                .execute_allowed(request, context)
                .expect_err(&tool.tool_name);
            assert!(
                error.to_string().contains("workspaceCode"),
                "{}: {error}",
                tool.tool_name
            );
        }
        checked += 1;
    }
    assert!(checked > 10);

    let error = f.state.agent_tool_registry.execute_allowed(
        ToolRequest { tool_name: "dude_todo_update".into(), input: json!({"todos":[]}), invocation_id: None },
        &f.context,
    ).expect_err("ordinary agents must not use Dude todos");
    assert!(error.to_string().contains("Dude"), "{error}");
}

fn tool_context(tools: Vec<tron_backend::domain::agent::EffectiveTool>) -> AgentRunContext {
    AgentRunContext {
        system_agent_fast_mode: false,
        collaboration_owner: None,
        agent_code: None,
        conversation_code: Some("CONV-TOOLS".to_string()),
        run_code: "RUN-TOOLS".to_string(),
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
            request_headers: Default::default(),
            model_name: "test".to_string(),
            thinking_enabled: false,
            supports_developer_role: false,
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
        tool_assignment: None,
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("manage resources")].into(),
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-07-16T00:00:00Z".to_string(),
        auto_model_route: None,
    }
}

#[test]
fn workspace_and_platform_variable_tools_round_trip() {
    let f = Fixture::new();
    for (prefix, key) in [
        ("variable", "WORKSPACE_TEST_KEY"),
        ("platform_variable", "PLATFORM_TEST_KEY"),
    ] {
        let workspace = prefix == "variable";
        let mut base = if workspace {
            json!({"workspaceCode":WORKSPACE})
        } else {
            json!({})
        };
        base["key"] = json!(key);
        base["value"] = json!("fixture-secret");
        base["isSensitive"] = json!(true);
        let created = f.call(&format!("{prefix}_create"), base);
        assert!(
            !created.to_string().contains("fixture-secret"),
            "sensitive value leaked"
        );
        let filter = if workspace {
            json!({"workspaceCode":WORKSPACE,"search":key})
        } else {
            json!({"search":key})
        };
        assert!(
            f.call(&format!("{prefix}_page"), filter.clone())
                .to_string()
                .contains(key)
        );
        let edit = if workspace {
            json!({"workspaceCode":WORKSPACE,"key":key,"description":"updated"})
        } else {
            json!({"key":key,"description":"updated"})
        };
        f.call(&format!("{prefix}_update"), edit);
        assert!(
            f.call(&format!("{prefix}_page"), filter.clone())
                .to_string()
                .contains("updated")
        );
        let delete = if workspace {
            json!({"workspaceCode":WORKSPACE,"key":key})
        } else {
            json!({"key":key})
        };
        f.call(&format!("{prefix}_delete"), delete);
        assert!(
            !f.call(&format!("{prefix}_page"), filter)
                .to_string()
                .contains(key)
        );
    }
}

#[test]
fn prompt_tool_accepts_managed_variables_and_round_trips() {
    let f = Fixture::new();
    f.call(
        "platform_variable_create",
        json!({"key":"PROMPT_ACCEPTANCE", "value":"hello", "isSensitive":false}),
    );
    let variable = json!({"name":"who", "type":"STRING", "description":"recipient", "sourceKind":"MANAGED", "managedVariableKey":"PROMPT_ACCEPTANCE"});
    let input = json!({"displayName":"Tool prompt acceptance", "category":"TEMPLATE", "promptBody":"Hello {{who}}", "variables":[variable.clone()]});
    let descriptor = BusinessToolCatalog::tool("platform.prompt.create", "en-US").unwrap();
    assert!(
        jsonschema::validator_for(&descriptor.input_schema)
            .unwrap()
            .is_valid(&input)
    );
    let created = f.call("platform_prompt_create", input);
    let code = created["promptCode"]
        .as_str()
        .expect("prompt code")
        .to_string();
    assert!(
        f.call(
            "platform_prompt_page",
            json!({"search":"Tool prompt acceptance"})
        )
        .to_string()
        .contains(&code)
    );
    let detail = f.call("platform_prompt_get", json!({"promptCode":code}));
    assert_eq!(
        detail["variables"][0]["managedVariableKey"],
        "PROMPT_ACCEPTANCE"
    );
    let version = detail["version"].as_u64().expect("prompt version");
    f.call("platform_prompt_update", json!({"promptCode":code,"expectedVersion":version,"displayName":"Updated tool prompt","promptBody":"Hi {{who}}","variables":[variable]}));
    assert_eq!(
        f.call("platform_prompt_get", json!({"promptCode":code}))["displayName"],
        "Updated tool prompt"
    );
    f.call("platform_prompt_delete", json!({"promptCode":code}));
}

#[test]
fn mcp_management_tools_round_trip_without_contacting_remote_server() {
    let f = Fixture::new();
    f.call("platform_mcp_create", json!({"serverCode":"fixture-mcp","name":"Fixture MCP","transportKind":"streamable_http","url":"http://127.0.0.1:1/mcp","authKind":"none"}));
    assert!(
        f.call("platform_mcp_page", json!({"search":"fixture-mcp"}))
            .to_string()
            .contains("fixture-mcp")
    );
    assert_eq!(
        f.call("platform_mcp_get", json!({"serverCode":"fixture-mcp"}))["name"],
        "Fixture MCP"
    );
    f.call("platform_mcp_update",json!({"serverCode":"fixture-mcp","name":"Updated MCP","transportKind":"streamable_http","url":"http://127.0.0.1:1/mcp","authKind":"none"}));
    assert_eq!(
        f.call("platform_mcp_get", json!({"serverCode":"fixture-mcp"}))["name"],
        "Updated MCP"
    );
    f.call("platform_mcp_delete", json!({"serverCode":"fixture-mcp"}));
}

#[test]
fn workspace_task_and_application_tools_round_trip() {
    let f = Fixture::new();
    assert_eq!(
        f.call("workspace_get", json!({"workspaceCode":WORKSPACE}))["workspaceCode"],
        WORKSPACE
    );
    assert!(
        f.call(
            "workspace_page",
            json!({"workspaceCode":WORKSPACE,"rootOnly":false})
        )
        .to_string()
        .contains(WORKSPACE)
    );
    let missing = f
        .state
        .agent_tool_registry
        .execute_allowed(
            ToolRequest {
                tool_name: "task_page".into(),
                input: json!({}),
                invocation_id: None,
            },
            &f.context,
        )
        .unwrap_err();
    assert!(missing.to_string().contains("workspaceCode"));
    let task = f.call(
        "task_create",
        json!({"workspaceCode":WORKSPACE,"title":"Tool task"}),
    );
    let task_code = task["taskCode"].as_str().expect("task code").to_string();
    assert!(
        f.call(
            "task_page",
            json!({"workspaceCode":WORKSPACE,"search":"Tool task"})
        )
        .to_string()
        .contains(&task_code)
    );
    f.call(
        "task_update",
        json!({"workspaceCode":WORKSPACE,"taskCode":task_code,"title":"Updated tool task"}),
    );
    assert!(
        f.call(
            "task_page",
            json!({"workspaceCode":WORKSPACE,"search":"Updated tool task"})
        )
        .to_string()
        .contains(&task_code)
    );
    f.call(
        "task_delete",
        json!({"workspaceCode":WORKSPACE,"taskCode":task_code}),
    );
    f.call(
        "task_restore",
        json!({"workspaceCode":WORKSPACE,"taskCode":task_code}),
    );
    assert!(
        f.call(
            "task_page",
            json!({"workspaceCode":WORKSPACE,"search":"Updated tool task"})
        )
        .to_string()
        .contains(&task_code)
    );

    let app = f.call("app_create", json!({"workspaceCode":WORKSPACE,"name":"Tool app","localDirectory":f._directory.path().display().to_string(),"runtimeRole":"frontend"}));
    let app_code = app["appCode"].as_str().expect("app code").to_string();
    assert!(
        f.call(
            "app_page",
            json!({"workspaceCode":WORKSPACE,"search":"Tool app"})
        )
        .to_string()
        .contains(&app_code)
    );
    f.call(
        "app_update",
        json!({"workspaceCode":WORKSPACE,"appCode":app_code,"name":"Updated tool app"}),
    );
    assert!(
        f.call(
            "app_page",
            json!({"workspaceCode":WORKSPACE,"search":"Updated tool app"})
        )
        .to_string()
        .contains(&app_code)
    );
    f.call(
        "app_delete",
        json!({"workspaceCode":WORKSPACE,"appCode":app_code}),
    );
    f.call(
        "app_restore",
        json!({"workspaceCode":WORKSPACE,"appCode":app_code}),
    );
    assert!(
        f.call(
            "app_page",
            json!({"workspaceCode":WORKSPACE,"search":"Updated tool app"})
        )
        .to_string()
        .contains(&app_code)
    );
}

#[test]
fn automation_tools_manage_a_paused_workspace_automation() {
    let f = Fixture::new();
    let created = f.call("automation_create", json!({"workspaceCode":WORKSPACE,"name":"Tool automation","prompt":"Summarize the test","cronExpression":"0 9 * * *","status":"paused"}));
    let code = created["automationCode"]
        .as_str()
        .expect("automation code")
        .to_string();
    assert!(
        f.call(
            "automation_page",
            json!({"workspaceCode":WORKSPACE,"search":"Tool automation"})
        )
        .to_string()
        .contains(&code)
    );
    assert!(
        f.call(
            "automation_get",
            json!({"workspaceCode":WORKSPACE,"automationCode":code})
        )
        .to_string()
        .contains("Tool automation")
    );
    f.call(
        "automation_update",
        json!({"workspaceCode":WORKSPACE,"automationCode":code,"name":"Updated automation"}),
    );
    assert!(
        f.call(
            "automation_get",
            json!({"workspaceCode":WORKSPACE,"automationCode":code})
        )
        .to_string()
        .contains("Updated automation")
    );
    f.call(
        "automation_delete",
        json!({"workspaceCode":WORKSPACE,"automationCode":code}),
    );
    f.call(
        "automation_restore",
        json!({"workspaceCode":WORKSPACE,"automationCode":code}),
    );
    assert!(
        f.call(
            "automation_get",
            json!({"workspaceCode":WORKSPACE,"automationCode":code})
        )
        .to_string()
        .contains("Updated automation")
    );
}
