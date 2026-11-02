mod common;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
};
use tron_backend::{
    adapters::outbound::application_directory::GitApplicationDirectoryInspector,
    application::{
        agent::{
            BusinessToolCatalog, FrameworkHost, ModelClient, ReactAgentBuilder, terminal_owner,
            tool_record_to_effective_tool,
        },
        project::ApplicationDirectoryInspector,
    },
    config::AppConfig,
    domain::agent::{
        AgentExecutionScope, AgentModelConfig, AgentRunContext, ModelRequest, ModelResponse,
    },
};
fn git(path: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
struct Script {
    step: Mutex<usize>,
    root: String,
    selected: String,
}
impl ModelClient for Script {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ModelResponse, String>> + Send + 'a>,
    > {
        Box::pin(async move {
            let step = {
                let mut next = self.step.lock().unwrap();
                let step = *next;
                *next += 1;
                step
            };
            // Gateway mode: every wrapped tool is addressed through the
            // stable `tool_execute` envelope.
            let execute = |tool: &str, arguments: serde_json::Value| {
                let arguments = if matches!(tool, "worktree_create" | "worktree_select") {
                    arguments
                } else {
                    json!({"items":[arguments]})
                };
                ModelResponse::tool_call("tool_execute", json!({"tool":tool,"arguments":arguments}))
            };
            Ok(match step {
                0 => execute(
                    "worktree_create",
                    json!({"name":"selected","branch":"feature/selected"}),
                ),
                1 => {
                    std::fs::write(Path::new(&self.selected).join("value.txt"), "selected")
                        .map_err(|e| e.to_string())?;
                    execute("fs_read_text", json!({"path":"value.txt"}))
                }
                2 => execute("command_exec", json!({"command":"pwd"})),
                3 => execute("terminal_start", json!({"command":"pwd; sleep 20"})),
                4 => execute("worktree_select", json!({"path":self.root})),
                5 => execute("fs_read_text", json!({"path":"value.txt"})),
                6 => execute("command_exec", json!({"command":"pwd"})),
                7 => execute("terminal_start", json!({"command":"pwd; sleep 20"})),
                _ => ModelResponse::final_output("verified"),
            })
        })
    }
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

    fn close<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
}
#[tokio::test]
async fn real_worktree_changes_rebind_native_fs_bash_and_pty_in_the_same_host_run() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("repository");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    git(&root, &["init", "-b", "main"]);
    std::fs::write(root.join("value.txt"), "main").unwrap();
    git(&root, &["add", "value.txt"]);
    git(
        &root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
    );
    let selected = root.parent().unwrap().join(".tron-worktrees/selected");
    let state = tron_backend::app::AppState::new_with_id_provider(
        common::prepared_config(AppConfig {
            database_url: temp
                .path()
                .join("tools.sqlite")
                .to_string_lossy()
                .into_owned(),
            ..Default::default()
        }),
        Arc::new(tron_backend::shared::id::FixedSnowflakeIdProvider::new(
            801_700_000_000_000_000,
        )),
    )
    .unwrap();
    let mut context:AgentRunContext=serde_json::from_value(json!({"conversationCode":"scope-session","runCode":eliteforge_agent_core::RunId::new_v4().to_string(),"workspaceCode":"TRON_CORE","userCode":"USR-1","locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"MODEL-1","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"unused","modelName":"test","maxTokens":128,"contextWindowTokens":131072},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":20,"maxToolCalls":20,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":"Verify local tools after selecting each worktree."}]}],"observations":[],"startedAt":"test"})).unwrap();
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
    context.tools = BusinessToolCatalog::list_tools(None, None, "en-US")
        .into_iter()
        .map(tool_record_to_effective_tool)
        .filter(|t| {
            [
                "worktree_create",
                "worktree_select",
                "fs_read_text",
                "command_exec",
                "terminal_start",
            ]
            .contains(&t.tool_name.as_str())
        })
        .collect();
    let host = FrameworkHost::temporary(None).unwrap();
    let check_host = host.clone();
    let checked = tokio::spawn(async move {
        let host = check_host;
        let agent = ReactAgentBuilder::new(
            Arc::new(Script {
                step: Mutex::new(0),
                root: root.to_string_lossy().into_owned(),
                selected: selected.to_string_lossy().into_owned(),
            }),
            state.agent_tool_registry.clone(),
            state.event_bus.clone(),
        )
        .with_directory_inspector(Some(Arc::new(GitApplicationDirectoryInspector)))
        .with_framework_host(Some(host.clone()))
        .build()
        .unwrap();
        let result = agent.run(context).await.unwrap();
        assert_eq!(result.final_output, "verified");
        let out = result
            .observations
            .iter()
            .map(|o| {
                serde_json::from_str::<Value>(&o.output)
                    .unwrap_or_else(|error| panic!("{error}: {}", o.output))
            })
            .collect::<Vec<_>>();
        assert_eq!(out[1]["content"], "selected");
        assert_eq!(out[5]["content"], "main");
        assert!(
            out[2]["stdout"]
                .as_str()
                .unwrap()
                .contains(selected.to_str().unwrap())
        );
        assert!(
            out[6]["stdout"]
                .as_str()
                .unwrap()
                .contains(root.to_str().unwrap())
        );
        assert_eq!(out[3]["cwd"], selected.to_str().unwrap());
        assert_eq!(out[7]["cwd"], root.to_str().unwrap());
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let owner = terminal_owner("TRON_CORE", Some("USR-1"), "scope-session");
        for (index, cwd) in [(3, &selected), (7, &root)] {
            let output = host
                .pty()
                .read(&owner, out[index]["terminalId"].as_str().unwrap(), 0)
                .unwrap();
            assert!(output.output.contains(cwd.to_str().unwrap()));
        }
    })
    .await;
    // Await SDK shutdown even if a tool-result assertion panics, then preserve
    // that failure. Closing this fixture's PTY owner is an independent fallback.
    let report = host.close().await;
    host.pty().close();
    assert!(report.is_success(), "{report:?}");
    checked.unwrap();
}
