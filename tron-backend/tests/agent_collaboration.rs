mod common;
#[path = "support/knowledge_base_fixture.rs"]
mod fixture;
#[path = "support/candidate_model_fixture.rs"]
mod models;
#[path = "support/system_agent_collaboration.rs"]
mod system_agents;
use eliteforge_agent_core as fw;
use fixture::*;
use serde_json::{Value, json};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tron_backend::{
    application::agent::{
        CreateWorkspaceInternalAgentCommand, ModelClient, ModelStreamSink, ReactAgentBuilder,
    },
    domain::agent::{AgentModelConfig, AgentRunContext, ModelRequest, ModelResponse},
    shared::event_bus::EventBusHandle,
};

#[derive(Default)]
struct Script {
    requests: Mutex<Vec<fw::ChatRequest>>,
    configs: Mutex<Vec<AgentModelConfig>>,
    management: Option<tron_backend::application::agent::AgentManagementService>,
    session_id: Mutex<Option<String>>,
    child_run: Mutex<Option<String>>,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    child_tool_goal: usize,
    skill_scope: Option<(String, String)>,
}
impl ModelClient for Script {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Err("native model required".into()) })
    }
    fn generate_native_once<'a>(
        &'a self,
        config: &'a AgentModelConfig,
        request: &'a fw::ChatRequest,
        _: &'a ModelRequest,
        _: Option<ModelStreamSink>,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request.clone());
            self.configs.lock().unwrap().push(config.clone());
            let root = request.messages.iter().find_map(|m| {
                let text = m.content.text();
                text.starts_with("root:").then_some(text)
            });
            let results: Vec<_> = request
                .messages
                .iter()
                .filter_map(|m| m.tool_result.as_ref().map(|_| m.content.text()))
                .collect();
            Ok(match root.as_deref() {
                Some("root:discovery") => match results.len() {
                    0 => ModelResponse::tool_call(
                        "discovery_expand_tool_domain",
                        json!({"domainCode":"native"}),
                    ),
                    1 => ModelResponse::tool_call("agent_get", json!({"agentCode":"reviewer"})),
                    2 => ModelResponse::tool_call(
                        "discovery_expand_tool_domain",
                        json!({"domainCode":"skill"}),
                    ),
                    _ => ModelResponse::final_output("discovery complete"),
                },
                Some("root:revoke-during-run") if results.is_empty() => {
                    self.management
                        .as_ref()
                        .unwrap()
                        .assign_agent_workspaces("reviewer", vec![])
                        .unwrap();
                    ModelResponse::tool_call(
                        "agent_spawn",
                        json!({"agentCode":"reviewer","task":"child:review"}),
                    )
                }
                Some("root:read-other") if results.is_empty() => ModelResponse::tool_call(
                    "thread_read",
                    json!({"session_id":self.session_id.lock().unwrap().clone().unwrap()}),
                ),
                Some("root:thread-leave" | "root:thread-revoke") if results.is_empty() => {
                    ModelResponse::tool_call(
                        "thread_create",
                        json!({"agentCode":"reviewer","task":"child:first"}),
                    )
                }
                Some("root:thread-leave" | "root:thread-revoke") => {
                    let created: Value = serde_json::from_str(&results[0]).unwrap();
                    let id = created["session_id"].as_str().unwrap();
                    *self.session_id.lock().unwrap() = Some(id.into());
                    if results.len() == 1 {
                        ModelResponse::tool_call("thread_wait", json!({"session_id":id}))
                    } else if root.as_deref() == Some("root:thread-revoke") && results.len() == 2 {
                        self.management
                            .as_ref()
                            .unwrap()
                            .assign_agent_workspaces("reviewer", vec![])
                            .unwrap();
                        ModelResponse::tool_call(
                            "thread_send",
                            json!({"session_id":id,"message":"child:denied"}),
                        )
                    } else {
                        ModelResponse::final_output(results.join("\n"))
                    }
                }
                Some("root:cancel") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:slow"}),
                ),
                Some("root:system-fast" | "root:system-deep") if results.is_empty() => {
                    ModelResponse::tool_call(
                        "agent_spawn",
                        json!({"agentCode": if root.as_deref() == Some("root:system-fast") { "codex-fast" } else { "codex-deep" }, "task":"child:external"}),
                    )
                }
                Some("root:skill-scope") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:skill-scope"}),
                ),
                Some("root:spawn-default") if results.is_empty() => {
                    ModelResponse::tool_call("agent_spawn", json!({"task":"child:review"}))
                }
                Some("root:detached" | "root:detached-leave") if results.is_empty() => {
                    ModelResponse::tool_call(
                        "agent_spawn",
                        json!({"agentCode":"reviewer", "task":"child:review", "background":true}),
                    )
                }
                Some("root:detached") => {
                    let spawned: Value = serde_json::from_str(&results[0])
                        .map_err(|e| format!("detached spawn result: {e}: {}", results[0]))?;
                    let child_run_id = spawned["child_run_id"]
                        .as_str()
                        .ok_or("detached spawn returns a child run ID")?;
                    assert_eq!(spawned["status"], "Running", "{spawned}");
                    if results.len() == 1 {
                        ModelResponse::tool_call("child_wait", json!({"child_run_id":child_run_id}))
                    } else {
                        ModelResponse::final_output(results.join("\n"))
                    }
                }
                Some("root:detached-leave") => {
                    let spawned: Value = serde_json::from_str(&results[0]).unwrap();
                    *self.child_run.lock().unwrap() =
                        Some(spawned["child_run_id"].as_str().unwrap().into());
                    ModelResponse::final_output("left detached child in background")
                }
                Some("root:detached-collect") => {
                    let child_run_id = self
                        .child_run
                        .lock()
                        .unwrap()
                        .clone()
                        .expect("detached child was left in background");
                    if results.is_empty() {
                        ModelResponse::tool_call("child_wait", json!({"child_run_id":child_run_id}))
                    } else {
                        ModelResponse::final_output(results.join("\n"))
                    }
                }
                Some("root:detached-inspect") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:review", "background":true}),
                ),
                Some("root:detached-inspect") => {
                    let spawned: Value = serde_json::from_str(&results[0]).unwrap();
                    let child_run_id = spawned["child_run_id"].as_str().unwrap();
                    match results.len() {
                        1 => ModelResponse::tool_call("child_list", json!({})),
                        2 => {
                            let listed: Value = serde_json::from_str(&results[1]).unwrap();
                            assert!(
                                listed["children"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .any(|child| child["child_run_id"].as_str()
                                        == Some(child_run_id)),
                                "{listed}"
                            );
                            ModelResponse::tool_call(
                                "child_wait",
                                json!({"child_run_id":child_run_id}),
                            )
                        }
                        3 => ModelResponse::tool_call(
                            "child_read",
                            json!({"child_run_id":child_run_id, "limit": 20}),
                        ),
                        _ => {
                            let read: Value =
                                serde_json::from_str(&results[3]).unwrap_or_else(|e| {
                                    panic!("child_read result: {e}: {}", results[3])
                                });
                            assert!(read.get("conversation").is_some(), "{read}");
                            ModelResponse::final_output("inspected detached child")
                        }
                    }
                }
                Some("root:detached-cancel") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:slow", "background":true}),
                ),
                Some("root:detached-cancel") => {
                    let spawned: Value = serde_json::from_str(&results[0]).unwrap();
                    let child_run_id: String = spawned["child_run_id"].as_str().unwrap().into();
                    match results.len() {
                        1 => ModelResponse::tool_call(
                            "child_cancel",
                            json!({"child_run_id":child_run_id}),
                        ),
                        2 => ModelResponse::tool_call(
                            "child_wait",
                            json!({"child_run_id":child_run_id}),
                        ),
                        _ => ModelResponse::final_output(results.join("\n")),
                    }
                }
                Some("root:detached-steer") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:review", "background":true}),
                ),
                Some("root:detached-steer") => {
                    let spawned: Value = serde_json::from_str(&results[0]).unwrap();
                    let child_run_id: String = spawned["child_run_id"].as_str().unwrap().into();
                    match results.len() {
                        1 => ModelResponse::tool_call(
                            "child_wait",
                            json!({"child_run_id":child_run_id}),
                        ),
                        2 => ModelResponse::tool_call(
                            "child_send",
                            json!({"child_run_id":child_run_id, "message":"child:reconsider"}),
                        ),
                        _ => ModelResponse::final_output(results.join("\n")),
                    }
                }
                Some("root:spawn") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:review"}),
                ),
                Some("root:limits") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:limits"}),
                ),
                Some("root:unknown") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"missing", "task":"child:review"}),
                ),
                Some("root:revoked") if results.is_empty() => ModelResponse::tool_call(
                    "agent_spawn",
                    json!({"agentCode":"reviewer", "task":"child:review"}),
                ),
                Some("root:thread" | "root:system-thread") if results.is_empty() => {
                    ModelResponse::tool_call(
                        "thread_create",
                        json!({"agentCode": if root.as_deref() == Some("root:system-thread") { "codex-fast" } else { "reviewer" }, "task":"child:first", "title":"Review"}),
                    )
                }
                Some("root:thread" | "root:system-thread") => {
                    let created: Value = serde_json::from_str(&results[0])
                        .map_err(|e| format!("thread result: {e}: {}", results[0]))?;
                    let id = created["session_id"].as_str().ok_or("missing thread ID")?;
                    match results.len() {
                        1 => ModelResponse::tool_call("thread_wait", json!({"session_id":id})),
                        2 => ModelResponse::tool_call(
                            "thread_send",
                            json!({"session_id":id,"message":"child:second"}),
                        ),
                        3 => ModelResponse::tool_call("thread_wait", json!({"session_id":id})),
                        4 => ModelResponse::tool_call("thread_read", json!({"session_id":id})),
                        5 => ModelResponse::tool_call("thread_close", json!({"session_id":id})),
                        _ => ModelResponse::final_output(results.join("\n")),
                    }
                }
                Some(_) => ModelResponse::final_output(results.join("\n")),
                None => {
                    if request
                        .messages
                        .iter()
                        .any(|message| message.content.text() == "child:skill-scope")
                    {
                        let (parent_skill, child_skill) = self.skill_scope.as_ref().unwrap();
                        return Ok(match results.len() {
                            0 => {
                                ModelResponse::tool_call("skill_load", json!({"name":parent_skill}))
                            }
                            1 => {
                                ModelResponse::tool_call("skill_load", json!({"name":child_skill}))
                            }
                            _ => {
                                let results = request
                                    .messages
                                    .iter()
                                    .filter_map(|message| message.tool_result.as_ref())
                                    .collect::<Vec<_>>();
                                assert!(results[0].is_error, "child loaded parent-only skill");
                                assert!(!results[1].is_error, "child could not load its own skill");
                                ModelResponse::final_output(
                                    "target skill loaded; parent skill denied",
                                )
                            }
                        });
                    }
                    if request
                        .messages
                        .iter()
                        .any(|m| m.content.text() == "child:limits")
                    {
                        return Ok(if results.len() < self.child_tool_goal {
                            ModelResponse::tool_call("thread_list", json!({}))
                        } else {
                            ModelResponse::final_output("completed with target limits")
                        });
                    }
                    if request
                        .messages
                        .iter()
                        .any(|m| m.content.text() == "child:slow")
                    {
                        self.entered.notify_one();
                        self.release.notified().await;
                    }
                    assert!(
                        request.messages.iter().any(|m| m.role == fw::Role::System
                            && m.content.text().contains("REVIEWER_ROLE")),
                        "target role was not loaded"
                    );
                    assert!(
                        request.tools.iter().all(|t| t.call_name == "agent_get"
                            || t.call_name == "agent_spawn"
                            || matches!(
                                t.call_name.as_str(),
                                "skill_load" | "skill_read_resource" | "skill_diagnostics"
                            )
                            || t.call_name.starts_with("thread_")
                            || t.call_name.starts_with("child_")),
                        "target inherited unauthorized tools: {:?}",
                        request
                            .tools
                            .iter()
                            .map(|tool| &tool.call_name)
                            .collect::<Vec<_>>()
                    );
                    ModelResponse::final_output("reviewed by target")
                }
            })
        })
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async {})
    }
    fn close_observed<'a>(
        &'a self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<eliteforge_agent_plugin_core::ShutdownReport, String>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Default::default()) })
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn domain_discovery_hides_native_schemas_until_their_domain_is_expanded() {
    use tron_backend::domain::agent::{ExpandedDomainPersistenceMode, ToolExposureMode};
    let f = Fixture::new();
    reviewer(&f);
    let host = f.state.agent_framework.clone();
    let script = Arc::new(Script::default());
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut root = context("root:discovery");
    root.tools = vec![
        tron_backend::application::agent::discovery_expand_tool_definition("en-US"),
        tron_backend::application::agent::ask_user_definition("en-US"),
    ];
    root.tool_exposure.tool_exposure_mode = Some(ToolExposureMode::DomainDiscovery);
    root.tool_exposure.expanded_domain_persistence_mode =
        Some(ExpandedDomainPersistenceMode::StickyExpanded);
    let result = agent.run(root).await.unwrap();
    assert_eq!(result.final_output, "discovery complete");
    let requests = script.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    let names = |index: usize| {
        requests[index]
            .tools
            .iter()
            .map(|tool| tool.call_name.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(0), vec!["discovery_expand_tool_domain"]);
    let directory = requests[0]
        .messages
        .iter()
        .map(|m| m.content.text())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        directory.contains("agent_spawn"),
        "native tools must remain discoverable"
    );
    assert!(
        directory.contains("skill_load"),
        "skill tools must remain discoverable"
    );
    assert!(names(1).contains(&"agent_get"));
    assert!(names(1).contains(&"agent_spawn"));
    assert!(names(1).contains(&"child_wait"));
    assert!(names(1).contains(&"thread_create"));
    assert!(
        !names(1).contains(&"skill_load"),
        "another domain must remain hidden"
    );
    assert!(
        !names(2).contains(&"skill_load"),
        "ordinary calls must not expose other domains"
    );
    assert!(names(3).contains(&"skill_load"));
    assert!(names(3).contains(&"skill_read_resource"));
    assert!(
        requests[3]
            .messages
            .iter()
            .filter_map(|m| m.tool_result.as_ref())
            .all(|result| !result.is_error)
    );
    drop(requests);
    assert!(host.close().await.is_success());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delegated_child_uses_its_definition_limits_and_stops_at_them() {
    let f = Fixture::new();
    reviewer(&f);
    let management = &f.state.agent_management;
    let host = f.state.agent_framework.clone();
    let script = Arc::new(Script {
        child_tool_goal: 5,
        ..Default::default()
    });
    let agent = ReactAgentBuilder::new(
        script,
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    for (steps, tools, expected_status, expected_steps, expected_tools) in [
        (7, 5, "Completed", 6, 5),
        (4, 5, "StepLimit", 4, 4),
        (7, 3, "ToolCallLimit", 4, 3),
    ] {
        management
            .update_internal_agent_policy(
                WORKSPACE,
                "reviewer",
                tron_backend::application::agent::UpdateWorkspaceInternalAgentPolicyCommand {
                    max_steps: Some(steps),
                    max_tool_calls: Some(tools),
                    ..Default::default()
                },
            )
            .unwrap();
        let mut root = context("root:limits");
        root.limits.max_steps = 3;
        root.limits.max_tool_calls = 2;
        let result = agent.run(root).await.unwrap();
        let child: Value = serde_json::from_str(&result.final_output).unwrap();
        assert_eq!(child["status"], expected_status, "{child}");
        assert_eq!(child["steps"], expected_steps, "{child}");
        assert_eq!(child["tool_calls"], expected_tools, "{child}");
    }
    assert!(host.close().await.is_success());
}
fn context(prompt: &str) -> AgentRunContext {
    serde_json::from_value(json!({
        "agentCode":"main-agent", "runCode":fw::RunId::new_v4().to_string(), "workspaceCode":WORKSPACE,"userCode":USER,"locale":"en-US","agentKind":"REACT",
        "model":{"modelCode":"fixture-chat","protocol":"CHAT_COMPLETIONS","baseUrl":"http://127.0.0.1:1/v1","apiKey":"synthetic","modelName":"fixture","maxTokens":2048,"contextWindowTokens":32000},
        "variables":[],"skills":[],"tools":[],"executionScope":{},"limits":{"maxSteps":12,"maxToolCalls":12,"maxParallelToolCalls":1},"messages":[{"role":"user","parts":[{"type":"text","text":prompt}]}],"observations":[],"startedAt":"fixture"
    })).unwrap()
}
fn reviewer(f: &Fixture) {
    models::seed_loop(&f.state);
    let candidate = models::add_text_model(&f.state, "Reviewer model");
    f.state
        .agent_management
        .create_platform_internal_agent(CreateWorkspaceInternalAgentCommand {
            description: String::new(),
            candidate_model_codes: vec![candidate],
            avatar_id: Some("fox".into()),
            loop_code: Some("test-react".into()),
            agent_code: "reviewer".into(),
            name: "Reviewer".into(),
            role_prompt: "REVIEWER_ROLE: inspect changes and report concrete findings.".into(),
            group_name: None,
        })
        .unwrap();
    f.state
        .agent_management
        .assign_agent_workspaces("reviewer", vec![WORKSPACE.into()])
        .unwrap();
    f.state
        .agent_management
        .update_internal_agent_policy(
            WORKSPACE,
            "reviewer",
            tron_backend::application::agent::UpdateWorkspaceInternalAgentPolicyCommand {
                tool_assignment_mode: Some(
                    tron_backend::domain::agent::ToolAssignmentMode::SelectedTools,
                ),
                tool_exposure_mode: Some(tron_backend::domain::agent::ToolExposureMode::DirectList),
                ..Default::default()
            },
        )
        .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_directory_named_child_and_revocation() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(f.state.short_term_memory.clone()))
    .build()
    .unwrap();
    let root = context("root:spawn");
    let root_session = root.run_code.clone();
    let result = agent.run(root).await.unwrap();
    assert!(
        result.final_output.contains("reviewed by target"),
        "{}",
        result.final_output
    );
    let children = host.subagents(&root_session, USER).await.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].record.agent_code, "reviewer");
    assert_eq!(children[0].record.avatar_id.as_deref(), Some("fox"));
    assert_eq!(children[0].record.task, "child:review");
    assert_ne!(children[0].record.session_id, root_session);
    assert_eq!(children[0].run.status, fw::HistoryStatus::Completed);
    let framework = host.storage_framework().await.unwrap();
    let parent_history = framework
        .history()
        .unwrap()
        .read_session(&root_session, Default::default())
        .await
        .unwrap();
    assert!(
        parent_history
            .runs
            .iter()
            .all(|run| run.session_id == root_session)
    );
    assert!(
        !parent_history
            .records
            .messages
            .iter()
            .any(|message| message.role == fw::Role::User
                && message.content
                    == vec![fw::HistoryContentBlock::Text {
                        text: "child:review".into()
                    }])
    );
    let (_, child_history) = host
        .subagent_history(&root_session, children[0].record.run_id, USER)
        .await
        .unwrap();
    assert!(
        child_history
            .records
            .messages
            .iter()
            .any(|message| message.role == fw::Role::User
                && message.content
                    == vec![fw::HistoryContentBlock::Text {
                        text: "child:review".into()
                    }])
    );
    assert!(
        host.subagents(&root_session, "OTHER-USER")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        host.subagent_history(&root_session, children[0].record.run_id, "OTHER-USER")
            .await,
        Err(fw::StateError::NotFound)
    ));
    {
        let requests = script.requests.lock().unwrap();
        let first = &requests[0];
        assert!(
            first
                .messages
                .iter()
                .any(|m| m.content.text().contains("\"agentCode\":\"reviewer\"")),
            "Agent directory missing"
        );
        assert!(first.tools.iter().any(|t| t.call_name == "agent_spawn"));
    }
    let unknown = agent.run(context("root:unknown")).await.unwrap();
    assert!(
        unknown.final_output.contains("unavailable"),
        "{}",
        unknown.final_output
    );
    f.state
        .agent_management
        .assign_agent_workspaces("reviewer", vec![])
        .unwrap();
    let before = script.requests.lock().unwrap().len();
    let revoked = agent.run(context("root:revoked")).await.unwrap();
    assert!(
        revoked.final_output.contains("unavailable"),
        "{}",
        revoked.final_output
    );
    let requests = script.requests.lock().unwrap();
    assert!(requests[before..].iter().all(|r| {
        !r.messages
            .iter()
            .any(|m| m.content.text().contains("\"agentCode\":\"reviewer\""))
    }));
    drop(requests);
    assert!(host.close().await.is_success());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_thread_keeps_target_across_messages() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        agent.run(context("root:thread")),
    )
    .await
    .expect("thread conversation should finish within 60 seconds")
    .unwrap();
    assert!(
        result.final_output.contains("reviewed by target"),
        "{}",
        result.final_output
    );
    assert!(
        result.final_output.contains("child:second"),
        "{}",
        result.final_output
    );
    assert!(
        result.final_output.contains("\"closed\":true"),
        "{}",
        result.final_output
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_checks_authorization_after_directory_was_read() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script {
        management: Some(f.state.agent_management.clone()),
        ..Default::default()
    });
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let result = agent.run(context("root:revoke-during-run")).await.unwrap();
    assert!(
        result.final_output.contains("unavailable"),
        "{}",
        result.final_output
    );
    assert_eq!(
        script.requests.lock().unwrap().len(),
        2,
        "revoked target executed"
    );
    assert!(host.close().await.is_success());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_threads_recheck_access_and_hide_other_users() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script {
        management: Some(f.state.agent_management.clone()),
        ..Default::default()
    });
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    agent.run(context("root:thread-leave")).await.unwrap();
    let mut other = context("root:read-other");
    other.user_code = Some("OTHER-USER".into());
    let denied = agent.run(other).await.unwrap();
    assert!(
        denied.final_output.contains("thread not found"),
        "{}",
        denied.final_output
    );
    let revoked = agent.run(context("root:thread-revoke")).await.unwrap();
    assert!(
        revoked.final_output.contains("unavailable"),
        "{}",
        revoked.final_output
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_child_stops_with_parent_cancellation() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let context = context("root:cancel");
    let run_code = context.run_code.clone();
    let run = tokio::spawn(async move { agent.run(context).await });
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        script.entered.notified(),
    )
    .await
    .unwrap();
    host.cancel_run(&run_code).unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), run)
        .await
        .unwrap()
        .unwrap();
    assert!(
        matches!(
            result,
            Err(tron_backend::domain::agent::AgentError::Cancelled)
        ),
        "{result:?}"
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_detached_child_runs_and_waits_inline() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script,
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        agent.run(context("root:detached")),
    )
    .await
    .expect("detached child conversation should finish within 60 seconds")
    .unwrap();
    assert!(
        result.final_output.contains("reviewed by target"),
        "{}",
        result.final_output
    );
    assert!(
        result.final_output.contains("\"status\":\"Completed\""),
        "{}",
        result.final_output
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_detached_child_survives_parent_run_end() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script,
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    // The first run leaves the child in the background and ends its turn;
    // the parent run completing must not cancel the detached child.
    let first = agent.run(context("root:detached-leave")).await.unwrap();
    assert!(
        first
            .final_output
            .contains("left detached child in background"),
        "{}",
        first.final_output
    );
    let second = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        agent.run(context("root:detached-collect")),
    )
    .await
    .expect("orphan detached child should be collectable within 60 seconds")
    .unwrap();
    assert!(
        second.final_output.contains("reviewed by target"),
        "{}",
        second.final_output
    );
    // Delivered results stay addressable: a later run can still wait again.
    let third = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        agent.run(context("root:detached-collect")),
    )
    .await
    .expect("delivered detached child should remain addressable")
    .unwrap();
    assert!(
        third.final_output.contains("reviewed by target"),
        "{}",
        third.final_output
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_detached_child_cancel_stops_it() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script,
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        agent.run(context("root:detached-cancel")),
    )
    .await
    .expect("cancelled detached child should settle within 60 seconds")
    .unwrap();
    assert!(
        result.final_output.to_lowercase().contains("cancel"),
        "{}",
        result.final_output
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_detached_child_list_and_read() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script,
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        agent.run(context("root:detached-inspect")),
    )
    .await
    .expect("detached child inspection should finish within 60 seconds")
    .unwrap();
    assert!(
        result.final_output.contains("inspected detached child"),
        "{}",
        result.final_output
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_detached_child_rejects_steering_when_finished() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script,
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    // child_wait delivers the result but keeps the child addressable; the
    // following child_send must be rejected because the run already finished.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        agent.run(context("root:detached-steer")),
    )
    .await
    .expect("detached child steering should finish within 60 seconds")
    .unwrap();
    assert!(
        result.final_output.contains("is not running"),
        "{}",
        result.final_output
    );
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn collaboration_child_does_not_inherit_parent_fixed_provider_model() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .build()
    .unwrap();
    let mut context = context("root:spawn");
    context.model.model_code = "PERSON_AI:fixture".into();
    context.model = tron_backend::adapters::outbound::model::with_person_ai_conversation(
        context.model,
        "parent-session",
        Some(&context.run_code),
    );
    let result = agent.run(context).await.unwrap();
    assert!(
        result.final_output.contains("reviewed by target"),
        "{}",
        result.final_output
    );
    let configs = script.configs.lock().unwrap();
    assert!(
        configs[0]
            .api_path
            .as_ref()
            .unwrap()
            .contains("parent-session")
    );
    let selected = f
        .state
        .agent_management
        .workspace_internal_agent_policy(WORKSPACE, "reviewer")
        .unwrap()
        .candidate_model_codes;
    assert_eq!(configs[1].model_code, selected[0]);
    assert!(
        !configs[1]
            .api_path
            .as_deref()
            .unwrap_or_default()
            .contains("parent-session")
    );
    drop(configs);
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subagents_and_threads_use_target_auto_pool_instead_of_parent_auto_pool() {
    for prompt in ["root:spawn", "root:thread"] {
        let f = Fixture::new();
        reviewer(&f);
        let parent_model = models::add_text_model(&f.state, "Parent model");
        f.state
            .agent_management
            .update_platform_internal_agent_definition(
                "main-agent",
                tron_backend::application::agent::UpdateWorkspaceInternalAgentDefinitionCommand {
                    candidate_model_codes: Some(vec![parent_model.clone()]),
                    ..Default::default()
                },
            )
            .unwrap();
        let router = tron_backend::application::model_router::AgentModelRouter::new(
            f.state.models.clone(),
            f.state.model_providers.clone(),
        )
        .with_agent_management(f.state.agent_management.clone());
        let (model, route) = router
            .resolve_initial(WORKSPACE, "parent-session", "main-agent", None)
            .unwrap();
        let script = Arc::new(Script::default());
        let host = f.state.agent_framework.clone();
        let agent = ReactAgentBuilder::new(
            script.clone(),
            f.state.agent_tool_registry.clone(),
            EventBusHandle::disabled(),
        )
        .with_framework_host(Some(host.clone()))
        .with_auto_model_router(Some(router))
        .build()
        .unwrap();
        let mut root = context(prompt);
        root.model = model;
        root.auto_model_route = Some(route);
        let result = agent.run(root).await.unwrap();
        assert!(
            result.final_output.contains("reviewed by target"),
            "{}",
            result.final_output
        );
        let child_model = f
            .state
            .agent_management
            .workspace_internal_agent_policy(WORKSPACE, "reviewer")
            .unwrap()
            .candidate_model_codes[0]
            .clone();
        let configs = script.configs.lock().unwrap();
        assert!(
            configs
                .iter()
                .any(|config| config.model_code == child_model)
        );
        assert!(configs.iter().all(|config| config.model_code == parent_model || config.model_code == child_model));
        drop(configs);
        assert!(host.close().await.is_success());
    }
}

#[path = "support/dude_thread_intervention.rs"]
mod dude_intervention;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_child_runs_with_production_short_term_memory_and_distinct_history() {
    let f = Fixture::new();
    reviewer(&f);
    let script = Arc::new(Script::default());
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script.clone(),
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(f.state.short_term_memory.clone()))
    .build()
    .unwrap();
    let mut root = context("root:spawn-default");
    root.agent_code = Some("reviewer".into());
    let session = root.run_code.clone();
    let result = agent.run(root).await.unwrap();
    assert!(
        result.final_output.contains("reviewed by target"),
        "{}",
        result.final_output
    );
    let children = host.subagents(&session, USER).await.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].record.agent_code, "reviewer");
    assert_ne!(children[0].record.session_id, session);
    assert_eq!(children[0].run.status, fw::HistoryStatus::Completed);
    let requests = script.requests.lock().unwrap();
    assert!(requests.iter().any(|request| {
        request
            .messages
            .iter()
            .any(|message| message.content.text() == "child:review")
    }));
    drop(requests);
    assert!(host.close().await.is_success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spawned_child_loads_only_its_own_configured_skills() {
    let f = Fixture::new();
    reviewer(&f);
    let available = f
        .state
        .skills
        .list_registry(tron_backend::domain::skill::RegistrySkillListQuery { search: None })
        .unwrap()
        .into_iter()
        .filter(|skill| skill.available && skill.provider_pack_id.is_some())
        .map(|skill| skill.skill_name)
        .collect::<Vec<_>>();
    assert!(
        available.len() >= 2,
        "fixture requires two installed built-in skills"
    );
    let parent_skill = available[0].clone();
    let child_skill = available[1].clone();
    f.state
        .skills
        .replace_agent_skills("main-agent", std::slice::from_ref(&parent_skill))
        .unwrap();
    f.state
        .skills
        .replace_agent_skills("reviewer", std::slice::from_ref(&child_skill))
        .unwrap();
    let script = Arc::new(Script {
        skill_scope: Some((parent_skill, child_skill)),
        ..Default::default()
    });
    let host = f.state.agent_framework.clone();
    let agent = ReactAgentBuilder::new(
        script,
        f.state.agent_tool_registry.clone(),
        EventBusHandle::disabled(),
    )
    .with_framework_host(Some(host.clone()))
    .with_short_term_memory(Some(f.state.short_term_memory.clone()))
    .build()
    .unwrap();
    let result = agent.run(context("root:skill-scope")).await.unwrap();
    assert!(
        result
            .final_output
            .contains("target skill loaded; parent skill denied"),
        "{}",
        result.final_output
    );
    assert!(host.close().await.is_success());
}
