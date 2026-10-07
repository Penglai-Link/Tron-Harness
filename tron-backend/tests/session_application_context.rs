mod common;

use axum::http::{Method, StatusCode};
use common::{app_with_config, authenticated_request, response_json};
use serde_json::json;
use std::{io::Cursor, path::Path, process::Command};
use tron_backend::{
    adapters::outbound::application_directory::GitApplicationDirectoryInspector,
    application::{
        agent::execution_context_message, project::ApplicationDirectoryInspector,
        resource::ChatSessionAttachmentCreateCommand,
    },
    config::AppConfig,
    domain::agent::{AgentExecutionScope, ApplicationDirectoryStatus},
};

fn git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn inspector_tracks_actual_branch_worktree_and_detached_head() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("repository with spaces");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-b", "main"]);
    git(
        &root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "initial",
        ],
    );
    let inspector = GitApplicationDirectoryInspector;
    let initial = inspector.inspect(root.to_str().unwrap());
    assert_eq!(initial.status, ApplicationDirectoryStatus::Ready);
    assert_eq!(initial.branch_name.as_deref(), Some("main"));
    assert!(!initial.is_worktree);
    git(&root, &["switch", "-c", "feature/current"]);
    assert_eq!(
        inspector
            .inspect(root.to_str().unwrap())
            .branch_name
            .as_deref(),
        Some("feature/current")
    );

    let worktree = temporary.path().join("linked tree");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "feature/linked",
            worktree.to_str().unwrap(),
        ],
    );
    let linked = inspector.inspect(worktree.to_str().unwrap());
    assert!(linked.is_worktree);
    assert_eq!(linked.branch_name.as_deref(), Some("feature/linked"));
    assert_eq!(
        linked.working_directory.as_deref(),
        worktree.canonicalize().unwrap().to_str()
    );
    git(&worktree, &["checkout", "--detach"]);
    let detached = inspector.inspect(worktree.to_str().unwrap());
    assert_eq!(detached.branch_name, None);
    assert!(detached.head_commit.is_some());
    assert!(detached.is_worktree);
    assert_eq!(
        inspector.inspect(temporary.path().to_str().unwrap()).status,
        ApplicationDirectoryStatus::NotGit
    );
    assert_eq!(
        inspector
            .inspect(temporary.path().join("missing").to_str().unwrap())
            .status,
        ApplicationDirectoryStatus::MissingDirectory
    );
    assert_eq!(
        inspector.inspect("relative/path").status,
        ApplicationDirectoryStatus::MissingDirectory
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_applications_resolve_live_project_scope_before_first_message() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("app");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-b", "feature/actual"]);
    let config = AppConfig {
        database_url: temporary
            .path()
            .join("test.sqlite")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    };
    let router = app_with_config(config.clone());
    let response = authenticated_request(&router, Method::POST, "/api/v1/apps/workspaces/TRON_CORE", Some(json!({
        "name": "tron-api", "localDirectory": root, "runtimeRole": "Backend", "defaultBranch": "main", "tags": []
    }))).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_app = response_json(response).await;
    let app_code = created_app["data"]["appCode"].as_str().unwrap();
    let project = response_json(authenticated_request(&router, Method::POST, "/api/v1/projects/save", Some(json!({
        "workspaceCode": "TRON_CORE", "name": "Linked applications", "appCodes": [app_code, "APP-TRON-CONSOLE"], "primaryAppCode": app_code
    }))).await).await;
    let session = response_json(authenticated_request(&router, Method::POST, "/api/v1/chats/sessions", Some(json!({
        "workspaceCode": "TRON_CORE", "projectCode": project["data"]["projectCode"], "title": "New chat", "agentCode": "main-agent"
    }))).await).await;
    let url = format!(
        "/api/v1/chats/sessions/applications?sessionCode={}",
        session["data"]["sessionCode"].as_str().unwrap()
    );
    let response = authenticated_request(&router, Method::GET, &url, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "0");
    assert_eq!(payload["data"]["applications"].as_array().unwrap().len(), 2);
    assert_eq!(
        payload["data"]["applications"][0]["branchName"],
        "feature/actual"
    );
    assert_eq!(payload["data"]["applications"][0]["appName"], "tron-api");
    assert_eq!(
        payload["data"]["workingDirectory"],
        root.canonicalize().unwrap().to_str().unwrap()
    );
    git(
        &root,
        &["symbolic-ref", "HEAD", "refs/heads/feature/changed"],
    );
    let refreshed =
        response_json(authenticated_request(&router, Method::GET, &url, None).await).await;
    assert_eq!(
        refreshed["data"]["applications"][0]["branchName"],
        "feature/changed"
    );
    // The page and the next invocation restore the selection from the persisted
    // mandatory context, not the project's originally configured checkout.
    git(
        &root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
    let selected = temporary.path().join("session-current");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "feature/session-current",
            selected.to_str().unwrap(),
        ],
    );
    let state = tron_backend::app::AppState::new_with_id_provider(
        common::prepared_config(config),
        std::sync::Arc::new(tron_backend::shared::id::FixedSnowflakeIdProvider::new(
            801_700_000_000_000_000,
        )),
    )
    .unwrap();
    let session_code = session["data"]["sessionCode"].as_str().unwrap();
    let mut context = productivity_context(&root);
    context.tools.clear();
    context.conversation_code = Some(session_code.into());
    context.execution_scope = state.chats.session_execution_scope(session_code).unwrap();
    let previous = context.execution_scope.applications[0].clone();
    context.execution_scope.applications[0] =
        tron_backend::application::project::inspect_current_application(
            &GitApplicationDirectoryInspector,
            &previous,
            selected.to_str(),
        );
    context.execution_scope.local_directory = context.execution_scope.applications[0]
        .working_directory
        .clone();
    context.execution_scope.branch_name =
        context.execution_scope.applications[0].branch_name.clone();
    let framework = state.agent_framework.storage_framework().await.unwrap();
    framework.sessions().unwrap().store.append_event(eliteforge_agent_core::SessionEvent::new(session_code,"tron.context-view",json!({"executionScope":context.execution_scope,"messages":context.messages,"contextReport":{"blocks":[]}}))).await.unwrap();
    let payload =
        response_json(authenticated_request(&router, Method::GET, &url, None).await).await;
    assert_eq!(
        payload["data"]["workingDirectory"],
        selected.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(
        payload["data"]["applications"][0]["branchName"],
        "feature/session-current"
    );
    assert_eq!(
        state
            .chats
            .session_execution_scope(session_code)
            .unwrap()
            .local_directory,
        context.execution_scope.local_directory
    );
    git(&root, &["worktree", "remove", selected.to_str().unwrap()]);
    let payload =
        response_json(authenticated_request(&router, Method::GET, &url, None).await).await;
    assert_eq!(
        payload["data"]["workingDirectory"],
        context.execution_scope.local_directory.unwrap()
    );
    assert_eq!(
        payload["data"]["applications"][0]["status"],
        "MissingDirectory"
    );
}

#[test]
fn execution_prefix_preserves_all_directories_and_does_not_invent_a_branch() {
    let temporary = tempfile::tempdir().unwrap();
    let mut app = GitApplicationDirectoryInspector.inspect(temporary.path().to_str().unwrap());
    app.app_code = "APP-1".into();
    app.app_name = "example".into();
    let scope = AgentExecutionScope {
        local_directory: app.working_directory.clone(),
        applications: vec![app],
        ..Default::default()
    };
    let message = execution_context_message(&scope).unwrap();
    assert_eq!(message.role, "system");
    let text = message.text_content();
    assert!(text.contains("APP-1"));
    assert!(text.contains("\"branchName\":null"));
    assert!(text.contains(temporary.path().canonicalize().unwrap().to_str().unwrap()));
    assert!(execution_context_message(&AgentExecutionScope::default()).is_none());
}

fn memory_context(_run_code: &str) -> tron_backend::domain::agent::AgentRunContext {
    use tron_backend::domain::agent::*;
    AgentRunContext {
        agent_code: None,
        conversation_code: Some("CONV-1".into()),
        run_code: eliteforge_agent_core::RunId::new_v4().to_string(),
        workspace_code: "TRON_CORE".into(),
        user_code: Some("USR-1".into()),
        locale: "en-US".into(),
        request_id: Some("REQ-1".into()),
        stream_key: Some("CONV-1".into()),
        agent_kind: AgentKind::React,
        system_agent_session_id: None,
        model: AgentModelConfig {
            model_code: "MODEL-1".into(),
            protocol: AgentModelProtocol::ChatCompletions,
            base_url: "http://127.0.0.1:1/v1".into(),
            api_path: None,
            api_key: "sk-test".into(),
            model_name: "gpt-test".into(),
            thinking_enabled: false,
            reasoning_effort: None,
            responses_stateful_enabled: false,
            temperature: Some(0.2),
            max_tokens: Some(128),
            context_window_tokens: Some(8_192),
            top_p: Some(1.0),
            max_retries: Some(0),
        },
        variables: vec![EffectiveVariable {
            key: "REGION".into(),
            value: "us".into(),
            source: "LOCAL".into(),
            sensitive: false,
        }],
        skills: vec![EffectiveSkill {
            skill_name: "planner".into(),
            description: "Plan work".into(),
            instruction: Some("Think briefly.".into()),
            tool_names: vec!["echo".into()],
        }],
        tools: vec![EffectiveTool {
            tool_name: "echo".into(),
            domain_code: None,
            description: "Echo input".into(),
            input_schema: json!({"type": "object"}),
            skill_name: Some("planner".into()),
            required_variable_keys: vec![],
        }],
        effective_tools: Vec::new(),
        tool_exposure: Default::default(),
        execution_scope: AgentExecutionScope {
            applications: Vec::new(),
            project_code: None,
            app_code: Some("APP-1".into()),
            repository_url: Some("https://example.test/repo.git".into()),
            local_directory: Some("/workspace/repo".into()),
            branch_name: Some("main".into()),
            task_code: Some("TASK-1".into()),
            allowed_local_directories: Vec::new(),
        },
        limits: AgentRunLimits {
            max_steps: 4,
            max_tool_calls: 2,
            max_parallel_tool_calls: 2,
        },
        messages: vec![AgentMessage::user("hello")],
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-05-27T00:00:00Z".into(),
        auto_model_route: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_working_directory_never_runs_in_the_backend_directory() {
    use std::sync::Arc;
    use tron_backend::{
        app::AppState,
        application::agent::RuntimeAdapterSelection,
        domain::agent::{EffectiveTool, ToolRequest},
        shared::id::FixedSnowflakeIdProvider,
    };
    let temporary = tempfile::tempdir().unwrap();
    let config = common::prepared_config(AppConfig {
        database_url: temporary
            .path()
            .join("tools.sqlite")
            .to_string_lossy()
            .into_owned(),
        ..AppConfig::default()
    });
    let state = AppState::new_with_id_provider(
        config,
        Arc::new(FixedSnowflakeIdProvider::new(801_300_000_000_000_000)),
    )
    .unwrap();
    let mut context = memory_context("RUN-MISSING-CWD");
    let missing = temporary
        .path()
        .join("missing")
        .to_string_lossy()
        .into_owned();
    context.execution_scope.local_directory = Some(missing.clone());
    context.tools = vec![EffectiveTool {
        tool_name: "bash_exec".into(),
        domain_code: None,
        description: "shell".into(),
        input_schema: json!({"type": "object"}),
        skill_name: None,
        required_variable_keys: vec![],
    }];
    let result = execute_tool(
        &state,
        ToolRequest {
            tool_name: "bash_exec".into(),
            input: json!({"command": "pwd"}),
            invocation_id: None,
        },
        &context,
    );
    assert!(result.unwrap_err().to_string().contains("filesystem root"));
    context.conversation_code = None;
    for selection in [
        RuntimeAdapterSelection::CodexExec {
            command_name: "must-not-be-started".into(),
            working_directory: Some(missing.clone()),
        },
        RuntimeAdapterSelection::OpenCodeRun {
            command_name: "must-not-be-started".into(),
            working_directory: Some(missing.clone()),
        },
    ] {
        let runtime = tron_backend::application::agent::AgentRuntimeRegistry::new(
            state.agent_model_client.clone(),
            state.agent_tool_registry.clone(),
            state.event_bus.clone(),
            state.runtime_http_client.clone(),
        )
        .resolve(selection);
        let error = runtime.start(context.clone(), None).await.unwrap_err();
        assert!(
            format!("{error:?}").contains("configured working directory"),
            "{error:?}"
        );
    }
    context.tools[0].tool_name = "fs_read_text".into();
    for cwd in [missing, "relative/project".into()] {
        context.execution_scope.local_directory = Some(cwd);
        let error = execute_tool(
            &state,
            ToolRequest {
                tool_name: "fs_read_text".into(),
                input: json!({"path": "Cargo.toml"}),
                invocation_id: None,
            },
            &context,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("filesystem root") || error.to_string().contains("absolute"),
            "{error}"
        );
    }
}

fn productivity_state(root: &Path) -> tron_backend::app::AppState {
    tron_backend::app::AppState::new_with_id_provider(
        common::prepared_config(AppConfig {
            database_url: root.join("tools.sqlite").to_string_lossy().into_owned(),
            ..Default::default()
        }),
        std::sync::Arc::new(tron_backend::shared::id::FixedSnowflakeIdProvider::new(
            801_400_000_000_000_000,
        )),
    )
    .unwrap()
}
fn productivity_context(root: &Path) -> tron_backend::domain::agent::AgentRunContext {
    let mut context = memory_context("RUN-PRODUCTIVITY");
    let mut app = GitApplicationDirectoryInspector.inspect(root.to_str().unwrap());
    app.app_code = "APP-1".into();
    app.app_name = "Test".into();
    app.is_primary = true;
    context.execution_scope = AgentExecutionScope {
        local_directory: app.working_directory.clone(),
        allowed_local_directories: vec![root.to_string_lossy().into_owned()],
        applications: vec![app],
        ..Default::default()
    };
    context.tools =
        tron_backend::application::agent::BusinessToolCatalog::list_tools(None, None, "en-US")
            .into_iter()
            .map(tron_backend::application::agent::tool_record_to_effective_tool)
            .collect();
    context.limits.max_steps = 10;
    context.limits.max_tool_calls = 10;
    context
}
fn call_tool(
    state: &tron_backend::app::AppState,
    context: &tron_backend::domain::agent::AgentRunContext,
    name: &str,
    input: serde_json::Value,
) -> serde_json::Value {
    call_tool_with_invocation(state, context, name, input, None)
}
fn call_tool_with_invocation(
    state: &tron_backend::app::AppState,
    context: &tron_backend::domain::agent::AgentRunContext,
    name: &str,
    input: serde_json::Value,
    invocation_id: Option<String>,
) -> serde_json::Value {
    let result = execute_tool(
        &state,
        tron_backend::domain::agent::ToolRequest {
            tool_name: name.into(),
            input,
            invocation_id,
        },
        context,
    )
    .unwrap();
    serde_json::from_str(&result.observation.output).unwrap()
}

// Generic tools are exercised through the same framework loop as production.
// Business-only tool unit cases retain their native harness registry boundary.
fn execute_tool(
    state: &tron_backend::app::AppState,
    request: tron_backend::domain::agent::ToolRequest,
    context: &tron_backend::domain::agent::AgentRunContext,
) -> Result<tron_backend::domain::agent::ToolExecutionResult, tron_backend::domain::agent::ToolError>
{
    use tron_backend::{
        application::agent::{ModelClient, ReactAgentBuilder},
        domain::agent::*,
    };
    ensure_native_fixture_session(state);
    // Terminal contract cases deliberately supply stable invocation IDs and
    // share the same public executor/service as the production tool provider.
    // Framework dispatch and schema validation have a separate public integration test.
    if request.tool_name.starts_with("terminal_") {
        use eliteforge_agent_official_plugin_tools_local::{
            FileSystemToolsConfig, SystemCommandConfig, pty_tools::PtyTools,
        };
        if !context.is_tool_authorized(&request.tool_name) {
            return Err(ToolError::ExecutionFailed(
                "terminal tool is not authorized".into(),
            ));
        }
        let cwd = context
            .execution_scope
            .local_directory
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("working directory unavailable".into()))?;
        let roots: Vec<std::path::PathBuf> =
            if context.execution_scope.allowed_local_directories.is_empty() {
                vec![std::path::PathBuf::from(cwd)]
            } else {
                context
                    .execution_scope
                    .allowed_local_directories
                    .iter()
                    .map(std::path::PathBuf::from)
                    .collect()
            };
        let shell = std::env::var_os("SHELL")
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_file())
            .unwrap_or_else(|| "/bin/sh".into());
        let config = SystemCommandConfig::new()
            .with_shell(shell)
            .with_directory_scope(
                FileSystemToolsConfig::new(cwd)
                    .with_authorized_roots(roots)
                    .with_authorized_symlinks(true),
            );
        let owner = tron_backend::application::agent::terminal_owner(
            &context.workspace_code,
            context.user_code.as_deref(),
            context
                .conversation_code
                .as_deref()
                .unwrap_or(&context.run_code),
        );
        let output = PtyTools::new(state.agent_framework.pty(), owner, config)
            .execute(&request.tool_name, request.input, request.invocation_id)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        return Ok(ToolExecutionResult::observation(ToolObservation {
            tool_name: request.tool_name,
            output: serde_json::to_string(&output).unwrap(),
        }));
    }
    if !(request.tool_name.starts_with("fs_") || request.tool_name.starts_with("bash_")) {
        return state.agent_tool_registry.execute_allowed(request, context);
    }
    struct CallModel(std::sync::Mutex<Option<ToolRequest>>);
    impl ModelClient for CallModel {
        fn generate<'a>(
            &'a self,
            _: &'a AgentModelConfig,
            _: &'a ModelRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
        > {
            Box::pin(async move {
                Ok(match self.0.lock().unwrap().take() {
                    Some(call) => ModelResponse::tool_call(call.tool_name, call.input),
                    None => ModelResponse::final_output("verified"),
                })
            })
        }

        // Test adapters explicitly declare one transport attempt per mock invocation.
        fn generate_once<'a>(
            &'a self,
            config: &'a AgentModelConfig,
            request: &'a ModelRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
        > {
            self.generate(config, request)
        }

        fn close_observed<'a>(
            &'a self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async { Ok(Default::default()) })
        }

        fn close<'a>(
            &'a self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
            Box::pin(async {})
        }
    }
    let mut context = context.clone();
    context.run_code = eliteforge_agent_core::RunId::new_v4().to_string();
    if request.input["fileRef"]["kind"] != "session_attachment" {
        context.conversation_code = None;
    }
    context.model.context_window_tokens = Some(128_000);
    context.limits.max_steps = 2;
    context.limits.max_tool_calls = 1;
    context.observations.clear();
    context
        .messages
        .push(AgentMessage::user("execute this tool"));
    context.short_term_memory = None;
    context
        .tools
        .retain(|tool| tool.tool_name == request.tool_name);
    let name = request.tool_name.clone();
    let result = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    let host = state.agent_framework.clone();
                    let agent = ReactAgentBuilder::new(
                        std::sync::Arc::new(CallModel(std::sync::Mutex::new(Some(request)))),
                        state.agent_tool_registry.clone(),
                        state.event_bus.clone(),
                    )
                    .with_framework_host(Some(host.clone()))
                    .build()
                    .unwrap();
                    let result = agent.run(context).await;
                    result
                })
            })
            .join()
            .unwrap()
    })
    .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
    let observation = result
        .observations
        .into_iter()
        .find(|value| value.tool_name == name)
        .ok_or_else(|| ToolError::ExecutionFailed("tool did not produce an observation".into()))?;
    let output: serde_json::Value = serde_json::from_str(&observation.output)
        .map_err(|_| ToolError::ExecutionFailed(observation.output.clone()))?;
    if output.get("error").is_some_and(|value| !value.is_null()) {
        return Err(ToolError::ExecutionFailed(observation.output));
    }
    Ok(ToolExecutionResult {
        observation,
        context_delta: Default::default(),
    })
}

#[test]
fn office_tools_round_trip_without_overwriting_sources() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    call_tool(
        &state,
        &context,
        "word_create",
        json!({"path":"report.docx","paragraphs":[{"text":"报告","heading":1},{"text":"状态：草稿 & 待检查"}],"table":[["项目","结果"],["测试","通过"]]}),
    );
    let read = call_tool(&state, &context, "word_read", json!({"path":"report.docx"}));
    assert_eq!(read["paragraphs"][1]["text"], "状态：草稿 & 待检查");
    assert!(
        read["paragraphs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["inTable"] == true)
    );
    let original = std::fs::read(root.join("report.docx")).unwrap();
    call_tool(
        &state,
        &context,
        "word_replace_text",
        json!({"path":"report.docx","outputPath":"final.docx","find":"草稿","replace":"正式","expectedCount":1}),
    );
    assert_eq!(std::fs::read(root.join("report.docx")).unwrap(), original);
    assert_eq!(
        call_tool(&state, &context, "word_read", json!({"path":"final.docx"}))["paragraphs"][1]["text"],
        "状态：正式 & 待检查"
    );
    let error=execute_tool(&state, tron_backend::domain::agent::ToolRequest{tool_name:"word_replace_text".into(),input:json!({"path":"report.docx","outputPath":"bad.docx","find":"草稿","replace":"正式","expectedCount":2}),invocation_id:None},&context).unwrap_err();
    assert!(error.to_string().contains("found 1"));
    assert!(!root.join("bad.docx").exists());
    call_tool(
        &state,
        &context,
        "excel_write",
        json!({"path":"book.xlsx","sheets":[{"name":"Sheet1","cells":[{"row":1,"column":1,"value":"金额"},{"row":2,"column":1,"value":42},{"row":3,"column":1,"formula":"SUM(A2:A2)"},{"row":4,"column":1,"value":"=literal"}]}]}),
    );
    let original = std::fs::read(root.join("book.xlsx")).unwrap();
    let read = call_tool(
        &state,
        &context,
        "excel_read",
        json!({"path":"book.xlsx","sheet":"Sheet1","rows":4,"columns":1}),
    );
    assert_eq!(read["cells"][1]["value"], "42");
    assert_eq!(read["cells"][2]["formula"], "SUM(A2:A2)");
    assert_eq!(read["cells"][3]["value"], "=literal");
    assert_eq!(read["cells"][3]["formula"], "");
    call_tool(
        &state,
        &context,
        "excel_write",
        json!({"path":"book.xlsx","outputPath":"book-updated.xlsx","sheets":[{"name":"Sheet1","cells":[{"row":2,"column":1,"value":99}]}]}),
    );
    assert_eq!(std::fs::read(root.join("book.xlsx")).unwrap(), original);
    let updated = call_tool(
        &state,
        &context,
        "excel_read",
        json!({"path":"book-updated.xlsx","sheet":"Sheet1","rows":4,"columns":1}),
    );
    assert_eq!(updated["cells"][1]["value"], "99");
    assert_eq!(updated["cells"][2]["formula"], "SUM(A2:A2)");
    for (name, input) in [
        (
            "word_create",
            json!({"path":"report.docx","paragraphs":[{"text":"overwrite"}]}),
        ),
        (
            "excel_write",
            json!({"path":"book.xlsx","sheets":[{"name":"Sheet1","cells":[]}]}),
        ),
        ("word_read", json!({"path":"../outside.docx"})),
    ] {
        assert!(
            execute_tool(
                &state,
                tron_backend::domain::agent::ToolRequest {
                    tool_name: name.into(),
                    input,
                    invocation_id: None,
                },
                &context
            )
            .is_err()
        );
    }
}

#[test]
fn office_tools_read_session_attachments_through_file_refs() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);

    let text_code = "CHAT-ATTACH-TEXT-1";
    state
        .resources
        .store_chat_session_attachment(
            ChatSessionAttachmentCreateCommand {
                workspace_code: context.workspace_code.clone(),
                session_code: context.conversation_code.clone().unwrap(),
                attachment_code: text_code.into(),
                file_name: "notes.txt".into(),
                content_type: "text/plain".into(),
            },
            &mut Cursor::new("第一行\nsecond line\n".as_bytes()),
        )
        .unwrap();
    let text = call_tool(
        &state,
        &context,
        "fs_read_text",
        json!({
            "fileRef":{"kind":"session_attachment","attachmentCode":text_code},
            "startLine":1,
            "endLine":2,
        }),
    );
    assert_eq!(text["content"], "第一行\nsecond line");
    assert_eq!(text["fileName"], "notes.txt");
    assert_eq!(text["fileRef"]["kind"], "session_attachment");

    for foreign_workspace in [false, true] {
        let mut foreign = context.clone();
        if foreign_workspace {
            foreign.workspace_code = "FOREIGN-WORKSPACE".into();
        } else {
            foreign.conversation_code = Some("FOREIGN-SESSION".into());
        }
        let result = execute_tool(
            &state,
            tron_backend::domain::agent::ToolRequest {
                tool_name: "fs_read_text".into(),
                input: json!({"fileRef":{"kind":"session_attachment","attachmentCode":text_code}}),
                invocation_id: None,
            },
            &foreign,
        );
        assert!(
            result.is_err(),
            "an attachment must remain scoped to its workspace and session"
        );
    }

    std::fs::write(root.join("payload.json"), r#"{"topics":["alpha","beta"]}"#).unwrap();
    let json_from_workspace = call_tool(
        &state,
        &context,
        "chat_attachment_parse_json",
        json!({
            "fileRef":{"kind":"workspace_path","path":"payload.json"},
            "jsonPath":"/topics/1",
        }),
    );
    assert_eq!(json_from_workspace["selection"]["value"], "beta");
    assert_eq!(json_from_workspace["fileRef"]["kind"], "workspace_path");

    let json_code = "CHAT-ATTACH-JSON-1";
    state
        .resources
        .store_chat_session_attachment(
            ChatSessionAttachmentCreateCommand {
                workspace_code: context.workspace_code.clone(),
                session_code: context.conversation_code.clone().unwrap(),
                attachment_code: json_code.into(),
                file_name: "payload.json".into(),
                content_type: "application/json".into(),
            },
            &mut Cursor::new(br#"{"topics":["alpha","beta"]}"#),
        )
        .unwrap();
    let json_from_session = call_tool(
        &state,
        &context,
        "chat_attachment_parse_json",
        json!({
            "fileRef":{"kind":"session_attachment","attachmentCode":json_code},
            "jsonPath":"/topics/0",
        }),
    );
    assert_eq!(json_from_session["selection"]["value"], "alpha");
    assert_eq!(json_from_session["attachment"]["attachmentCode"], json_code);
    let legacy_json = call_tool(
        &state,
        &context,
        "chat_attachment_parse_json",
        json!({"attachmentCode":json_code,"jsonPath":"/topics/1"}),
    );
    assert_eq!(legacy_json["selection"]["value"], "beta");

    call_tool(
        &state,
        &context,
        "word_create",
        json!({"path":"attachment-source.docx","paragraphs":[{"text":"附件正文：待处理"}]}),
    );
    let word_bytes = std::fs::read(root.join("attachment-source.docx")).unwrap();
    let attachment_code = "CHAT-ATTACH-WORD-1";
    state
        .resources
        .store_chat_session_attachment(
            ChatSessionAttachmentCreateCommand {
                workspace_code: context.workspace_code.clone(),
                session_code: context.conversation_code.clone().unwrap(),
                attachment_code: attachment_code.into(),
                file_name: "upload.docx".into(),
                content_type:
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                        .into(),
            },
            &mut Cursor::new(word_bytes.clone()),
        )
        .unwrap();

    let attachment_ref = json!({
        "kind":"session_attachment",
        "attachmentCode":attachment_code,
    });
    let read = call_tool(
        &state,
        &context,
        "word_read",
        json!({"fileRef":attachment_ref,"limit":100}),
    );
    assert_eq!(read["fileName"], "upload.docx");
    assert_eq!(read["paragraphs"][0]["text"], "附件正文：待处理");
    assert_eq!(read["fileRef"]["kind"], "session_attachment");

    call_tool(
        &state,
        &context,
        "word_replace_text",
        json!({
            "fileRef":attachment_ref,
            "outputPath":"attachment-export.docx",
            "find":"待处理",
            "replace":"已完成",
            "expectedCount":1,
        }),
    );
    let exported = call_tool(
        &state,
        &context,
        "word_read",
        json!({"path":"attachment-export.docx"}),
    );
    assert_eq!(exported["paragraphs"][0]["text"], "附件正文：已完成");
    let (record, stored_word_bytes) = state
        .resources
        .read_chat_session_attachment(
            &context.workspace_code,
            context.conversation_code.as_deref().unwrap(),
            attachment_code,
        )
        .unwrap();
    assert_eq!(record.file_name, "upload.docx");
    assert_eq!(stored_word_bytes, word_bytes);

    call_tool(
        &state,
        &context,
        "excel_write",
        json!({"path":"attachment-source.xlsx","sheets":[{"name":"Sheet1","cells":[{"row":1,"column":1,"value":"主机名"},{"row":2,"column":1,"value":"db-01"}]}]}),
    );
    let excel_bytes = std::fs::read(root.join("attachment-source.xlsx")).unwrap();
    let excel_code = "CHAT-ATTACH-XLSX-1";
    state
        .resources
        .store_chat_session_attachment(
            ChatSessionAttachmentCreateCommand {
                workspace_code: context.workspace_code.clone(),
                session_code: context.conversation_code.clone().unwrap(),
                attachment_code: excel_code.into(),
                file_name: "servers.xlsx".into(),
                content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    .into(),
            },
            &mut Cursor::new(excel_bytes),
        )
        .unwrap();
    let excel = call_tool(
        &state,
        &context,
        "excel_read",
        json!({
            "fileRef":{"kind":"session_attachment","attachmentCode":excel_code},
            "sheet":"Sheet1",
            "rows":2,
            "columns":1,
        }),
    );
    assert_eq!(excel["fileName"], "servers.xlsx");
    assert_eq!(excel["cells"][1]["value"], "db-01");
}

#[test]
fn productivity_domains_are_localized_and_discoverable() {
    use tron_backend::application::agent::BusinessToolCatalog;
    let temporary = tempfile::tempdir().unwrap();
    let state = productivity_state(temporary.path());
    for locale in ["en-US", "zh-CN"] {
        let domains = BusinessToolCatalog::domains(locale);
        let mut context = productivity_context(temporary.path());
        context.locale = locale.into();
        context.tool_exposure.hidden_tools = context.tools.clone();
        context.tools =
            vec![tron_backend::application::agent::discovery_expand_tool_definition(locale)];
        context.effective_tools = context
            .tools
            .iter()
            .chain(context.tool_exposure.hidden_tools.iter())
            .cloned()
            .collect();
        context.tool_exposure.visible_domains = domains.clone();

        for domain in ["worktree", "git", "word", "excel"] {
            assert!(domains.iter().any(|item| item.domain_code == domain));
            let tools = BusinessToolCatalog::list_tools(Some(domain), None, locale);
            assert!(!tools.is_empty());
            let expanded = execute_tool(
                &state,
                tron_backend::domain::agent::ToolRequest {
                    tool_name: "discovery_expand_tool_domain".into(),
                    input: json!({"domainCode":domain}),
                    invocation_id: None,
                },
                &context,
            )
            .unwrap();
            assert_eq!(expanded.context_delta.add_tools.len(), tools.len());
            assert!(
                expanded
                    .context_delta
                    .add_tools
                    .iter()
                    .all(|tool| tool.domain_code.as_deref() == Some(domain))
            );

            for tool in tools {
                let encoded = serde_json::to_string(&tool).unwrap();
                assert!(!encoded.contains("UNTRANSLATED"), "{encoded}");
                assert!(!encoded.contains("x-i18n-description-key"));
                jsonschema::validator_for(&tool.input_schema).unwrap();
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_run_switches_worktrees_and_refreshes_next_model_request_without_inventory() {
    use std::sync::{Arc, Mutex};
    use tron_backend::{
        application::agent::{ModelClient, ReactAgentBuilder},
        domain::agent::{AgentModelConfig, ModelRequest, ModelResponse},
    };
    struct Script {
        step: Mutex<usize>,
        root: String,
        requests: Arc<Mutex<Vec<ModelRequest>>>,
    }
    impl ModelClient for Script {
        fn generate<'a>(
            &'a self,
            _config: &'a AgentModelConfig,
            request: &'a ModelRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
        > {
            Box::pin(async move {
                self.requests.lock().unwrap().push(request.clone());
                let mut step = self.step.lock().unwrap();
                let current = *step;
                *step += 1;
                Ok(match current {
                    0 => ModelResponse::tool_call(
                        "worktree_create",
                        json!({"name":"selected","branch":"feature/selected"}),
                    ),
                    1 => ModelResponse::tool_call(
                        "git_switch_branch",
                        json!({"branch":"feature/new","create":true}),
                    ),
                    2 => ModelResponse::tool_call("worktree_select", json!({"path":self.root})),
                    _ => ModelResponse::final_output("verified"),
                })
            })
        }

        // Test adapters explicitly declare one transport attempt per mock invocation.
        fn generate_once<'a>(
            &'a self,
            config: &'a AgentModelConfig,
            request: &'a ModelRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
        > {
            self.generate(config, request)
        }

        fn close_observed<'a>(
            &'a self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async { Ok(Default::default()) })
        }

        fn close<'a>(
            &'a self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
            Box::pin(async {})
        }
    }
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("repository");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    git(&root, &["init", "-b", "main"]);
    git(
        &root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
    std::fs::write(root.join("AGENTS.md"), "main-only project instruction").unwrap();
    let unused = temporary.path().join("never-selected");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-b",
            "feature/unused",
            unused.to_str().unwrap(),
        ],
    );
    let state = productivity_state(temporary.path());
    ensure_native_fixture_session(&state);
    let mut context = productivity_context(&root);
    // Keep request budget small enough to verify the execution prefix, without unrelated catalog schemas.
    context.tools.retain(|tool| {
        ["worktree_create", "worktree_select", "git_switch_branch"]
            .contains(&tool.tool_name.as_str())
    });
    let requests = Arc::new(Mutex::new(Vec::new()));
    let agent = ReactAgentBuilder::new(
        Arc::new(Script {
            step: Mutex::new(0),
            root: root.to_string_lossy().into_owned(),
            requests: requests.clone(),
        }),
        state.agent_tool_registry.clone(),
        state.event_bus.clone(),
    )
    .with_framework_host(Some(state.agent_framework.clone()))
    .with_directory_inspector(Some(Arc::new(GitApplicationDirectoryInspector)))
    .build()
    .unwrap();
    let result = agent.run(context).await.unwrap();
    assert_eq!(result.final_output, "verified");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    for (index, request) in requests.iter().enumerate() {
        let has_main = request
            .messages
            .iter()
            .any(|m| m.text_content().contains("main-only project instruction"));
        assert_eq!(
            has_main,
            index == 0 || index == 3,
            "instruction scope at request {index}"
        );
    }
    let scopes = requests
        .iter()
        .map(|request| {
            let message = request
                .messages
                .iter()
                .find(|message| {
                    message
                        .text_content()
                        .contains("application-execution-context")
                })
                .unwrap();
            let content = message.text_content();
            let begin = content.find('{').unwrap();
            let end = content.rfind('}').unwrap();
            let value: serde_json::Value = serde_json::from_str(&content[begin..=end]).unwrap();
            assert!(!content.contains("never-selected"));
            assert!(!content.contains("feature/unused"));
            assert!(value["applications"][0].get("worktrees").is_none());
            value
        })
        .collect::<Vec<_>>();
    assert_eq!(scopes[0]["branchName"], "main");
    assert_eq!(scopes[1]["branchName"], "feature/selected");
    assert_eq!(scopes[2]["branchName"], "feature/new");
    assert_eq!(scopes[3]["branchName"], "main");
    assert!(
        scopes[1]["localDirectory"]
            .as_str()
            .unwrap()
            .ends_with(".tron-worktrees/selected")
    );
    assert_eq!(scopes[3]["localDirectory"], root.to_str().unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_selected_worktree_never_reverts_to_old_tool_observations() {
    use tron_backend::domain::agent::{ToolObservation, ToolRequest};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("repository");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    git(&root, &["init", "-b", "main"]);
    git(
        &root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
    let state = productivity_state(temporary.path());
    ensure_native_fixture_session(&state);
    let mut context = productivity_context(&root);
    context.observations.push(ToolObservation {
        tool_name: "bash_exec".into(),
        output: json!({"cwd":root}).to_string(),
    });
    let created = call_tool(
        &state,
        &context,
        "worktree_create",
        json!({"name":"disappearing", "branch":"feature/disappearing"}),
    );
    let selected = created["cwd"].as_str().unwrap();
    context.observations.push(ToolObservation {
        tool_name: "worktree_create".into(),
        output: created.to_string(),
    });
    let inspector = std::sync::Arc::new(GitApplicationDirectoryInspector);
    tron_backend::application::agent::refresh_execution_context(&mut context, inspector.clone())
        .await
        .unwrap();
    git(&root, &["worktree", "remove", selected]);
    tron_backend::application::agent::refresh_execution_context(&mut context, inspector)
        .await
        .unwrap();
    assert_eq!(
        context.execution_scope.local_directory.as_deref(),
        Some(selected)
    );
    assert_eq!(
        context.execution_scope.applications[0].status,
        ApplicationDirectoryStatus::MissingDirectory
    );
    assert!(
        execute_tool(
            &state,
            ToolRequest {
                tool_name: "bash_exec".into(),
                input: json!({"command":"pwd"}),
                invocation_id: None,
            },
            &context
        )
        .is_err()
    );
}

#[test]
fn word_replacement_preserves_split_run_formatting_and_other_package_parts() {
    use std::io::{Read, Write};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(root.join("split.docx")).unwrap());
    zip.start_file(
        "word/document.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>hello </w:t></w:r><w:r><w:t>world!</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
    zip.start_file(
        "customXml/keep.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"<keep>unchanged</keep>").unwrap();
    zip.finish().unwrap();
    call_tool(
        &state,
        &context,
        "word_replace_text",
        json!({"path":"split.docx","outputPath":"split-updated.docx","find":"hello world","replace":"Hello & welcome"}),
    );
    let read = call_tool(
        &state,
        &context,
        "word_read",
        json!({"path":"split-updated.docx"}),
    );
    assert_eq!(read["paragraphs"][0]["text"], "Hello & welcome!");
    let mut zip =
        zip::ZipArchive::new(std::fs::File::open(root.join("split-updated.docx")).unwrap())
            .unwrap();
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("<w:rPr><w:b/></w:rPr>"));
    let mut retained = String::new();
    zip.by_name("customXml/keep.xml")
        .unwrap()
        .read_to_string(&mut retained)
        .unwrap();
    assert_eq!(retained, "<keep>unchanged</keep>");
    for (name, input) in [
        (
            "word_create",
            json!({"path":"invalid.docx","paragraphs":[{"text":"invalid\u{1}"}]}),
        ),
        ("excel_read", json!({"path":"none.xlsx","startRow":0})),
        (
            "excel_write",
            json!({"path":"invalid.xlsx","sheets":[{"name":"Sheet1","cells":[{"row":1,"column":1,"value":1,"formula":"SUM(A1:A2)"}]}]}),
        ),
    ] {
        assert!(
            execute_tool(
                &state,
                tron_backend::domain::agent::ToolRequest {
                    tool_name: name.into(),
                    input,
                    invocation_id: None,
                },
                &context
            )
            .is_err()
        );
    }
    assert!(!root.join("invalid.docx").exists());
    assert!(!root.join("invalid.xlsx").exists());
}

#[test]
fn code_patch_previews_and_applies_multiple_files_and_moves() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let mut context = productivity_context(&root);
    // The full native multi-file patch observation costs 9,820 history tokens,
    // above the default 8k fixture's 7,776 allowance. Retain every diff and move
    // while giving this filesystem behavior test an adequate declared window.
    context.model.context_window_tokens = Some(16_384);
    std::fs::write(root.join("main.rs"), "fn old() {}\n").unwrap();
    std::fs::write(root.join("move.rs"), "fn moved() {}\n").unwrap();
    std::fs::write(root.join("remove.txt"), "obsolete\n").unwrap();
    let patch = "*** Begin Patch\n*** Update File: main.rs\n@@\n-fn old() {}\n+fn new() {}\n*** Update File: move.rs\n*** Move to: nested/moved.rs\n@@\n-fn moved() {}\n+pub fn moved() {}\n*** Delete File: remove.txt\n*** Add File: nested/new.txt\n+你好，补丁\n*** End Patch";
    let preview = call_tool(
        &state,
        &context,
        "fs_apply_patch",
        json!({"patch":patch,"dryRun":true}),
    );
    assert_eq!(preview["applied"], false);
    assert!(preview["changes"].as_array().unwrap().iter().any(|change| {
        change["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("+fn new() {}"))
    }));
    assert_eq!(preview["changes"].as_array().unwrap().len(), 5);
    assert!(!root.join("nested").exists());
    assert!(root.join("remove.txt").exists());
    let result = call_tool(&state, &context, "fs_apply_patch", json!({"patch":patch}));
    assert_eq!(result["applied"], true);
    assert_eq!(
        std::fs::read_to_string(root.join("main.rs")).unwrap(),
        "fn new() {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("nested/moved.rs")).unwrap(),
        "pub fn moved() {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("nested/new.txt")).unwrap(),
        "你好，补丁\n"
    );
    assert!(!root.join("move.rs").exists());
    assert!(!root.join("remove.txt").exists());
}

#[test]
fn code_patch_rejects_invalid_edits_and_preserves_unrelated_changes() {
    use tron_backend::domain::agent::ToolRequest;
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let state = productivity_state(temporary.path());
    let context = productivity_context(&root);
    std::fs::write(root.join("a.txt"), "same\nsame\n").unwrap();
    let patches = [
        "*** Begin Patch\n*** Add File: new.txt\n+new\n*** Update File: a.txt\n@@\n-missing\n+replacement\n*** End Patch".to_string(),
        "*** Begin Patch\n*** Update File: a.txt\n@@\n-same\n+replacement\n*** End Patch".into(),
        "*** Begin Patch\n*** Add File: ../outside.txt\n+bad\n*** End Patch".into(),
        format!("*** Begin Patch\n*** Add File: {}\n+bad\n*** End Patch",temporary.path().join("outside.txt").display()),
        "*** Begin Patch\n*** Add File: a.txt\n+bad\n*** End Patch".into(),
        "*** Begin Patch\n*** Add File: new.txt\n+new\n*** Add File: new.txt/child.txt\n+conflict\n*** End Patch".into(),
        "*** Begin Patch\n*** Delete File: a.txt\n*** Add File: a.txt\n+bad\n*** End Patch".into(),
        "*** Begin Patch\n*** Update File: a.txt\n*** Move to: a.txt\n*** End Patch".into(),
        "*** Begin Patch\n*** Update File: a.txt\n@@\n+ambiguous insertion\n*** End Patch".into(),
    ];
    for patch in patches {
        assert!(
            execute_tool(
                &state,
                ToolRequest {
                    tool_name: "fs_apply_patch".into(),
                    input: json!({"patch":patch}),
                    invocation_id: None,
                },
                &context
            )
            .is_err(),
            "{patch}"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("a.txt")).unwrap(),
            "same\nsame\n"
        );
        assert!(!root.join("new.txt").exists());
        assert!(!temporary.path().join("outside.txt").exists());
    }
    std::fs::write(root.join("a.txt"), "first\n").unwrap();
    call_tool(&state, &context, "fs_read_text", json!({"path":"a.txt"}));
    std::fs::write(root.join("a.txt"), "first\nconcurrent change\n").unwrap();
    assert!(execute_tool(&state, ToolRequest{tool_name:"fs_apply_patch".into(),input:json!({"patch":"*** Begin Patch\n*** Update File: a.txt\n@@\n-first\n+updated\n*** End Patch"}),invocation_id:None},&context).is_ok());
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        "updated\nconcurrent change\n"
    );
}

#[test]
fn code_patch_preserves_line_endings_and_supports_anchors_and_eof() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    std::fs::write(root.join("windows.txt"), "前文\r\nold\r\n函数二\r\nold\r\n").unwrap();
    call_tool(
        &state,
        &context,
        "fs_apply_patch",
        json!({"patch":"*** Begin Patch\n*** Update File: windows.txt\n@@ 函数二\n-old\n+新内容\n*** End of File\n*** End Patch"}),
    );
    assert_eq!(
        std::fs::read_to_string(root.join("windows.txt")).unwrap(),
        "前文\r\nold\r\n函数二\r\n新内容\r\n"
    );
    std::fs::write(root.join("no-newline.txt"), "old").unwrap();
    call_tool(
        &state,
        &context,
        "fs_apply_patch",
        json!({"patch":"*** Begin Patch\n*** Update File: no-newline.txt\n@@\n-old\n+new\n*** End Patch"}),
    );
    assert_eq!(
        std::fs::read_to_string(root.join("no-newline.txt")).unwrap(),
        "new"
    );
    call_tool(
        &state,
        &context,
        "fs_apply_patch",
        json!({"patch":"*** Begin Patch\n*** Update File: no-newline.txt\n@@\n+tail\n*** End of File\n*** End Patch"}),
    );
    assert_eq!(
        std::fs::read_to_string(root.join("no-newline.txt")).unwrap(),
        "new\ntail"
    );
}

#[test]
fn legacy_patch_guards_ambiguity_and_catalog_removes_code_check() {
    use tron_backend::{application::agent::BusinessToolCatalog, domain::agent::ToolRequest};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    std::fs::write(root.join("a.txt"), "old old").unwrap();
    let request = |input| {
        execute_tool(
            &state,
            ToolRequest {
                tool_name: "fs_apply_patch".into(),
                input,
                invocation_id: None,
            },
            &context,
        )
    };
    assert!(
        request(json!({"path":"a.txt","action":"replace","target":"old","content":"new"})).is_err()
    );
    assert!(request(json!({"path":"a.txt","action":"replace","target":"old","content":"new","replaceAll":true,"expectedCount":1})).is_err());
    let read = call_tool(
        &state,
        &context,
        "fs_read_text",
        json!({"path":"a.txt","maxBytes":1}),
    );
    assert!(read.get("contentHash").is_none());
    request(json!({"path":"a.txt","action":"replace","target":"old","content":"new","replaceAll":true,"expectedCount":2})).unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        "new new"
    );
    for locale in ["en-US", "zh-CN"] {
        assert!(BusinessToolCatalog::tool("code.check", locale).is_none());
        assert!(
            !BusinessToolCatalog::domains(locale)
                .iter()
                .any(|domain| domain.domain_code == "code")
        );
        let tool = BusinessToolCatalog::tool("fs.apply_patch", locale).unwrap();
        let encoded = serde_json::to_string(&tool).unwrap();
        assert!(!encoded.contains("x-i18n-description-key"));
        let validator = jsonschema::validator_for(&tool.input_schema).unwrap();
        assert!(validator.is_valid(
            &json!({"patch":"*** Begin Patch\n*** Add File: a\n+x\n*** End Patch","dryRun":true})
        ));
        assert!(
            validator.is_valid(&json!({"path":"a","action":"replace","target":"x","content":"y"}))
        );
        assert!(
            !validator.is_valid(&json!({"patch":"x","path":"a","action":"replace","target":"x"}))
        );
    }
}

#[cfg(unix)]
#[test]
fn code_patch_preserves_executable_permissions_and_rejects_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    use tron_backend::domain::agent::ToolRequest;
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    std::fs::write(root.join("run.sh"), "echo old\n").unwrap();
    std::fs::set_permissions(root.join("run.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    call_tool(
        &state,
        &context,
        "fs_apply_patch",
        json!({"patch":"*** Begin Patch\n*** Update File: run.sh\n*** Move to: bin/run.sh\n@@\n-echo old\n+echo new\n*** End Patch"}),
    );
    assert_eq!(
        std::fs::metadata(root.join("bin/run.sh"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    symlink(root.join("bin/run.sh"), root.join("link.sh")).unwrap();
    assert!(
        execute_tool(
            &state,
            ToolRequest {
                tool_name: "fs_apply_patch".into(),
                input: json!({"patch":"*** Begin Patch\n*** Delete File: link.sh\n*** End Patch"}),
                invocation_id: None
            },
            &context
        )
        .is_err()
    );
    assert!(root.join("bin/run.sh").exists());
}

#[cfg(unix)]
#[test]
fn agent_terminal_accepts_a_complete_command_line_without_args() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    let started = call_tool(
        &state,
        &context,
        "terminal_start",
        json!({"command":"printf ready && sleep 30"}),
    );
    let id = started["terminalId"].as_str().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut output = started["output"].as_str().unwrap().to_string();
    let mut cursor = started["nextCursor"].as_u64().unwrap();
    while !output.contains("ready") {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let snapshot = call_tool(
            &state,
            &context,
            "terminal_read",
            json!({"terminalId":id,"cursor":cursor}),
        );
        output.push_str(snapshot["output"].as_str().unwrap());
        cursor = snapshot["nextCursor"].as_u64().unwrap();
    }
    assert_eq!(
        call_tool(&state, &context, "terminal_read", json!({"terminalId":id}))["running"],
        true
    );
    call_tool(&state, &context, "terminal_stop", json!({"terminalId":id}));
}

#[cfg(unix)]
#[test]
fn agent_terminal_retries_are_idempotent_and_bare_shells_are_rejected() {
    use tron_backend::domain::agent::ToolRequest;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    let input = json!({"command":"sleep 30"});
    let first = call_tool_with_invocation(
        &state,
        &context,
        "terminal_start",
        input.clone(),
        Some("run-id:call-1".into()),
    );
    let retried = call_tool_with_invocation(
        &state,
        &context,
        "terminal_start",
        input.clone(),
        Some("run-id:call-1".into()),
    );
    assert_eq!(first["terminalId"], retried["terminalId"]);

    let second = call_tool_with_invocation(
        &state,
        &context,
        "terminal_start",
        input,
        Some("run-id:call-2".into()),
    );
    assert_ne!(first["terminalId"], second["terminalId"]);
    assert_eq!(
        call_tool(&state, &context, "terminal_list", json!({}))
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let bare_shell = execute_tool(
        &state,
        ToolRequest {
            tool_name: "terminal_start".into(),
            input: json!({"command":"bash"}),
            invocation_id: None,
        },
        &context,
    );
    assert!(
        bare_shell
            .unwrap_err()
            .to_string()
            .contains("interactive: true")
    );

    for terminal in [first, second] {
        call_tool(
            &state,
            &context,
            "terminal_stop",
            json!({"terminalId":terminal["terminalId"]}),
        );
    }
}

#[cfg(unix)]
#[test]
fn agent_terminal_survives_turns_accepts_input_and_isolates_conversations() {
    use tron_backend::domain::agent::ToolRequest;
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let mut context = productivity_context(&root);
    context.conversation_code = Some("terminal-owner".into());
    let started = call_tool(
        &state,
        &context,
        "terminal_start",
        json!({"command":"/bin/bash","args":["--noprofile","--norc","-c","printf 'ready\n'; read answer; printf 'answer=%s\n' \"$answer\"; pwd"]}),
    );
    let id = started["terminalId"].as_str().unwrap();
    assert_eq!(started["cwd"], root.to_string_lossy().as_ref());
    let mut outsider = context.clone();
    outsider.conversation_code = Some("other".into());
    assert!(
        execute_tool(
            &state,
            ToolRequest {
                tool_name: "terminal_read".into(),
                input: json!({"terminalId":id}),
                invocation_id: None,
            },
            &outsider
        )
        .is_err()
    );
    assert_eq!(
        call_tool(&state, &outsider, "terminal_list", json!({})),
        json!([])
    );
    let other_dir = root.join("other-worktree");
    std::fs::create_dir(&other_dir).unwrap();
    context.execution_scope.local_directory = Some(other_dir.to_string_lossy().into_owned());
    context.execution_scope.applications[0].working_directory =
        Some(other_dir.to_string_lossy().into_owned());
    assert_eq!(
        call_tool(&state, &context, "terminal_read", json!({"terminalId":id}))["cwd"],
        root.to_string_lossy().as_ref()
    );
    context.run_code = "next-agent-turn".into();
    call_tool(
        &state,
        &context,
        "terminal_write",
        json!({"terminalId":id,"data":"hello\n"}),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut cursor = started["nextCursor"].as_u64().unwrap();
    let mut output = started["output"].as_str().unwrap().to_string();
    loop {
        let result = call_tool(
            &state,
            &context,
            "terminal_read",
            json!({"terminalId":id,"cursor":cursor}),
        );
        output.push_str(result["output"].as_str().unwrap());
        cursor = result["nextCursor"].as_u64().unwrap();
        if result["running"] == false && result["outputClosed"] == true {
            assert_eq!(result["exitCode"], 0);
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(output.contains("answer=hello"));
    assert!(output.contains(root.to_str().unwrap()));
    let again = call_tool(
        &state,
        &context,
        "terminal_read",
        json!({"terminalId":id,"cursor":cursor}),
    );
    assert_eq!(again["output"], "");
    call_tool(&state, &context, "terminal_stop", json!({"terminalId":id}));
}

#[cfg(unix)]
#[test]
fn agent_terminal_bounds_output_and_stops_running_process() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let context = productivity_context(&root);
    let started = call_tool(
        &state,
        &context,
        "terminal_start",
        json!({"command":"/bin/bash","args":["--noprofile","--norc","-c","head -c 300000 /dev/zero | tr '\\0' x; printf '\nDONE\n'; sleep 30"]}),
    );
    let id = started["terminalId"].as_str().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let result = call_tool(&state, &context, "terminal_read", json!({"terminalId":id}));
        assert!(result["output"].as_str().unwrap().len() <= 32768);
        if result["truncated"] == true {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    call_tool(&state, &context, "terminal_stop", json!({"terminalId":id}));
    assert_eq!(
        call_tool(&state, &context, "terminal_read", json!({"terminalId":id}))["running"],
        false
    );
    for locale in ["en-US", "zh-CN"] {
        for code in [
            "terminal.start",
            "terminal.read",
            "terminal.write",
            "terminal.stop",
            "terminal.list",
        ] {
            let record =
                tron_backend::application::agent::BusinessToolCatalog::tool(code, locale).unwrap();
            assert!(!record.description.contains("agent.toolCatalog"));
            assert!(
                !serde_json::to_string(&record.input_schema)
                    .unwrap()
                    .contains("x-i18n")
            );
        }
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn terminal_http_and_agent_share_process_with_conversation_isolation() {
    use axum::{Extension, body::Body, http::Request};
    use tower::ServiceExt;
    use tron_backend::{
        adapters::inbound::http::{auth::RequestAuthContext, chats::routes},
        application::chat::CreateChatSessionCommand,
    };
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let auth = RequestAuthContext::from_profile(
        &state.profiles.current_context().unwrap(),
        "en-US".into(),
    );
    let create = || {
        state
            .chats
            .create_session(
                CreateChatSessionCommand {
                    workspace_code: Some("TRON_CORE".into()),
                    project_code: None,
                    title: Some("Terminal UI test".into()),
                    app_code: None,
                    task_code: None,
                    source_type: None,
                    source_code: None,
                    agent_code: Some("main-agent".into()),
                },
                None,
            )
            .unwrap()
    };
    let session = create();
    let other = create();
    let mut context = productivity_context(&root);
    context.workspace_code = session.workspace_code.clone();
    context.user_code = Some(auth.user().user_code.clone());
    context.conversation_code = Some(session.session_code.clone());
    // Exercise the production FrameworkHost and official scoped tool lease,
    // then access that exact process through the product HTTP API.
    use tron_backend::application::agent::{ModelClient, ReactAgentBuilder};
    use tron_backend::domain::agent::{AgentModelConfig, ModelRequest, ModelResponse};
    struct StartTerminalModel(std::sync::Mutex<Option<serde_json::Value>>);
    impl ModelClient for StartTerminalModel {
        fn generate<'a>(
            &'a self,
            _: &'a AgentModelConfig,
            _: &'a ModelRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
        > {
            Box::pin(async move {
                Ok(match self.0.lock().unwrap().take() {
                    Some(input) => ModelResponse::tool_call("terminal_start", input),
                    None => ModelResponse::final_output("terminal started"),
                })
            })
        }

        // Test adapters explicitly declare one transport attempt per mock invocation.
        fn generate_once<'a>(
            &'a self,
            config: &'a AgentModelConfig,
            request: &'a ModelRequest,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
        > {
            self.generate(config, request)
        }

        fn close_observed<'a>(
            &'a self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async { Ok(Default::default()) })
        }

        fn close<'a>(
            &'a self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
            Box::pin(async {})
        }
    }
    let model = StartTerminalModel(std::sync::Mutex::new(Some(
        json!({"command":"/bin/bash","args":["--noprofile","--norc","-c","printf 'shared process'; read answer"],"cwd":root}),
    )));
    let mut run_context = context.clone();
    run_context.short_term_memory = None;
    run_context
        .tools
        .retain(|tool| tool.tool_name == "terminal_start");
    run_context.limits.max_steps = 2;
    run_context.limits.max_tool_calls = 1;
    let run = ReactAgentBuilder::new(
        std::sync::Arc::new(model),
        state.agent_tool_registry.clone(),
        state.event_bus.clone(),
    )
    .with_framework_host(Some(state.agent_framework.clone()))
    .build()
    .unwrap()
    .run(run_context)
    .await
    .expect("official terminal tool should run through FrameworkHost");
    let observation = run
        .observations
        .iter()
        .find(|item| item.tool_name == "terminal_start")
        .unwrap();
    let started: serde_json::Value = serde_json::from_str(&observation.output).unwrap();
    let id = started["terminalId"].as_str().unwrap();
    let router = routes::routes()
        .with_state(state.clone())
        .layer(Extension(auth));
    let base = format!("/api/v1/chats/sessions/{}/terminals", session.session_code);
    let response = router
        .clone()
        .oneshot(Request::builder().uri(&base).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let list = response_json(response).await;
    assert_eq!(list["data"][0]["terminalId"], id);
    assert_eq!(list["data"][0]["command"], "/bin/bash");
    let invalid = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("{base}/{id}?cursor=-1"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response_json(invalid).await["code"], "101028002");
    for method in [Method::GET, Method::POST] {
        let suffix = if method == Method::POST { "/stop" } else { "" };
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(format!(
                        "/api/v1/chats/sessions/{}/terminals/{id}{suffix}",
                        other.session_code
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    assert_eq!(
        call_tool(&state, &context, "terminal_read", json!({"terminalId":id}))["running"],
        true
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("{base}/{id}/stop"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        call_tool(&state, &context, "terminal_read", json!({"terminalId":id}))["running"],
        false
    );
    state.agent_framework.close().await;
}

#[test]
fn project_instructions_follow_scope_precedence_and_size_limit() {
    use tron_backend::application::project::ApplicationDirectoryInspector;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    git(&root, &["init", "-b", "main"]);
    let nested = root.join("src");
    std::fs::create_dir(&nested).unwrap();
    std::fs::write(root.join("AGENTS.md"), "root rules").unwrap();
    std::fs::write(root.join("CLAUDE.md"), "unused fallback").unwrap();
    std::fs::write(nested.join("CLAUDE.md"), "nested rules").unwrap();
    let reader = GitApplicationDirectoryInspector;
    let files = reader.instruction_files(nested.to_str().unwrap()).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].content, "root rules");
    assert_eq!(files[1].content, "nested rules");
    std::fs::write(root.join("AGENTS.override.md"), "override rules").unwrap();
    assert_eq!(
        reader.instruction_files(nested.to_str().unwrap()).unwrap()[0].content,
        "override rules"
    );
    std::fs::write(root.join("AGENTS.override.md"), "规则".repeat(10000)).unwrap();
    let limited = reader.instruction_files(nested.to_str().unwrap()).unwrap();
    assert_eq!(limited.len(), 1);
    assert!(limited[0].truncated);
    assert!(limited[0].content.len() <= 32768);
    assert!(!limited[0].content.contains('\u{fffd}'));
}

#[cfg(unix)]
#[test]
fn project_instructions_reject_external_links_and_ignore_unrelated_files() {
    use tron_backend::application::project::ApplicationDirectoryInspector;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(temp.path().join("AGENTS.md"), "unrelated parent").unwrap();
    std::fs::write(root.join("README.md"), "not automatically loaded").unwrap();
    let reader = GitApplicationDirectoryInspector;
    assert!(
        reader
            .instruction_files(root.to_str().unwrap())
            .unwrap()
            .is_empty()
    );
    std::os::unix::fs::symlink(temp.path().join("AGENTS.md"), root.join("AGENTS.md")).unwrap();
    assert!(reader.instruction_files(root.to_str().unwrap()).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_instructions_refresh_without_duplicating_and_primary_tools_use_cwd() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let state = productivity_state(&root);
    let mut context = productivity_context(&root);
    context.short_term_memory = Some(
        tron_backend::domain::agent::AgentShortTermMemoryRunContext {
            session_code: "instruction-test".into(),
            input_code: "input-test".into(),
            input_kind: "USER_PROMPT".into(),
            input_metadata: None,
            prompt_prefix: context.messages.clone(),
            prompt_prefix_sources: vec![None; context.messages.len()],
            current_input_message: None,
            agent_code: None,
            todo_enabled: false,
        },
    );
    let reader = std::sync::Arc::new(GitApplicationDirectoryInspector);
    std::fs::write(root.join("AGENTS.md"), "first version").unwrap();
    assert!(
        tron_backend::application::agent::refresh_execution_context(&mut context, reader.clone())
            .await
            .unwrap()
    );
    assert!(
        context
            .messages
            .iter()
            .any(|m| m.role == "system" && m.text_content().contains("first version"))
    );
    assert!(
        !tron_backend::application::agent::refresh_execution_context(&mut context, reader.clone())
            .await
            .unwrap()
    );
    std::fs::write(root.join("AGENTS.md"), "second version").unwrap();
    assert!(
        tron_backend::application::agent::refresh_execution_context(&mut context, reader.clone())
            .await
            .unwrap()
    );
    assert!(
        !context
            .messages
            .iter()
            .any(|m| m.text_content().contains("first version"))
    );
    assert_eq!(
        context
            .messages
            .iter()
            .filter(|m| m.text_content().contains("second version"))
            .count(),
        1
    );
    let memory = context.short_term_memory.as_ref().unwrap();
    assert_eq!(
        memory
            .prompt_prefix
            .iter()
            .filter(|m| m.text_content().contains("second version"))
            .count(),
        1
    );
    assert_eq!(
        memory.prompt_prefix.len(),
        memory.prompt_prefix_sources.len()
    );
    let output = call_tool(&state, &context, "bash_exec", json!({"command":"pwd"}));
    assert_eq!(
        std::fs::canonicalize(output["cwd"].as_str().unwrap()).unwrap(),
        root
    );
    assert_eq!(
        output["stdout"].as_str().unwrap().trim(),
        root.to_str().unwrap()
    );
    std::fs::remove_file(root.join("AGENTS.md")).unwrap();
    assert!(
        tron_backend::application::agent::refresh_execution_context(&mut context, reader)
            .await
            .unwrap()
    );
    assert!(
        !context
            .messages
            .iter()
            .any(|m| m.text_content().contains("second version"))
    );
}

fn ensure_native_fixture_session(state: &tron_backend::app::AppState) {
    use tron_backend::application::chat::ChatRepository;
    let mut repository =
        tron_backend::adapters::outbound::persistence::SqliteChatRepository::with_host(
            state.agent_framework.clone(),
        )
        .unwrap();
    if repository.get_session("CONV-1").is_err() {
        let session = serde_json::from_value(json!({"id":1,"sessionCode":"CONV-1","workspaceCode":"TRON_CORE","title":"tool fixture","status":"ACTIVE","sourceType":"MANUAL","createdAt":"","updatedAt":""})).unwrap();
        repository.create_session(&session).unwrap();
    }
}
