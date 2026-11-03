//! Exercise the registered tools against an isolated platform database; no model calls.
mod common;

use serde_json::{Value, json};
use tron_backend::{
    app::AppState,
    application::{
        agent::{BusinessToolCatalog, tool_record_to_effective_tool},
        dude::register_dude_tools,
    },
    config::{AppConfig, ResourceConfig},
    domain::{agent::*, dude::SoulState},
};

struct Fixture {
    _dir: tempfile::TempDir,
    state: AppState,
    registry: ToolRegistry,
    code: String,
    session: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = common::prepared_config(AppConfig {
            database_url: dir.path().join("tools.sqlite3").display().to_string(),
            graph_storage_path: dir.path().join("tools.grafeo").display().to_string(),
            skills_registry_root: dir.path().join("skills").display().to_string(),
            resource: ResourceConfig::with_default_root(
                dir.path().join("resources").display().to_string(),
            ),
            ..AppConfig::default()
        });
        let mut captured = None;
        let _router = common::app_with_state_setup(config, |state| captured = Some(state.clone()));
        let state = captured.unwrap();
        state.knowledge_bases.stop_ingestion_workers();
        let code = state.dudes.main().unwrap().dude_code;
        let (soul, _) = state.dudes.begin_awakening(&code, "USR-AUTH-0001").unwrap();
        let session = state.dudes.session(soul.session_code.as_deref().unwrap()).unwrap().unwrap();
        state.chats.ensure_dude_session(&session, Some("USR-AUTH-0001".into())).unwrap();
        let registry = ToolRegistry::default();
        register_dude_tools(
            &registry,
            state.dudes.clone(),
            state.tasks.clone(),
            state.agent_framework.clone(),
        )
        .unwrap();
        Self {
            _dir: dir,
            state,
            registry,
            code,
            session: soul.session_code.unwrap(),
        }
    }
    fn context(&self, source: &str) -> AgentRunContext {
        let model: AgentModelConfig = serde_json::from_value(json!({"modelCode":"fixture","protocol":"RESPONSES","baseUrl":"http://127.0.0.1:1","apiKey":"fixture","modelName":"fixture"})).unwrap();
        AgentRunContext {
            system_agent_fast_mode: false,
            collaboration_owner: Some(AgentCollaborationOwner {
                kind: "DUDE".into(),
                code: self.code.clone(),
                origin_session_code: Some(self.session.clone()),
                task_code: None,
            }),
            agent_code: Some(self.code.clone()),
            conversation_code: Some(self.session.clone()),
            run_code: "tool-test".into(),
            workspace_code: String::new(),
            user_code: Some("USR-AUTH-0001".into()),
            locale: "en-US".into(),
            request_id: None,
            stream_key: None,
            agent_kind: AgentKind::React,
            system_agent_session_id: None,
            model,
            auto_model_route: None,
            variables: vec![],
            skills: vec![],
            tools: BusinessToolCatalog::list_tools(None, None, "en-US")
                .into_iter()
                .filter(|tool| tool.domain_code.starts_with("dude_"))
                .map(tool_record_to_effective_tool)
                .collect(),
            effective_tools: vec![],
            tool_assignment: None,
            execution_scope: Default::default(),
            limits: Default::default(),
            messages: vec![].into(),
            observations: vec![],
            started_at: "2026-10-14T00:00:00Z".into(),
            short_term_memory: Some(AgentShortTermMemoryRunContext {
                session_code: self.session.clone(),
                input_code: "input".into(),
                input_kind: "USER_PROMPT".into(),
                input_metadata: None,
                prompt_prefix: vec![],
                prompt_prefix_sources: vec![],
                current_input_message: Some(AgentMessage::user(source)),
                agent_code: Some(self.code.clone()),
                todo_enabled: false,
            }),
        }
    }
    fn call(&self, tool: &str, input: Value, ctx: &AgentRunContext) -> Result<Value, ToolError> {
        self.registry
            .execute_allowed(
                ToolRequest {
                    tool_name: tool.into(),
                    input,
                    invocation_id: None,
                },
                ctx,
            )
            .map(|r| serde_json::from_str(&r.observation.output).unwrap())
    }
    fn update(&self, input: Value, ctx: &AgentRunContext) -> Result<Value, ToolError> {
        self.call("dude_soul_update", input, ctx)
    }
    fn task(&self) -> String {
        use tron_backend::{
            application::task::CreateTaskCommand,
            domain::task::{TaskPriority, TaskSourceType, TaskType},
        };
        self.state
            .tasks
            .create_task(CreateTaskCommand {
                workspace_code: None,
                owner_dude_code: Some(self.code.clone()),
                due_at: None,
                parent_task_code: None,
                title: "Notification milestone".into(),
                description: None,
                priority: TaskPriority::Medium,
                task_type: TaskType::General,
                source_type: TaskSourceType::Internal,
                source_url: None,
                agent_code: None,
                model_code: None,
                assignee_user_codes: vec![],
                agent_codes: vec![],
                app_codes: vec![],
            })
            .unwrap()
            .task_code
    }

}

#[tokio::test]
async fn soul_tools_bind_current_dude_and_enforce_input_shape() {
    let f = Fixture::new();
    let ctx = f.context("以后叫小川");
    let read = f.call("dude_soul_read", json!({}), &ctx).unwrap();
    assert!(read.get("sessionCode").is_none());
    assert!(read.get("userCode").is_none());
    assert!(
        f.call("dude_soul_read", json!({"dudeCode":"memory-dude"}), &ctx)
            .is_err()
    );
    assert!(f.call("dude_soul_read", Value::Null, &ctx).is_err());
    let seed = f.state.dudes.soul(&f.code).unwrap();
    for input in [
        json!({}),
        json!({"name":" "}),
        json!({"name":"x".repeat(101)}),
        json!({"initialSoul":"正文","edits":[{"old":"","new":"冲突"}]}),
        json!({"name":"小川","dudeCode":"another"}),
    ] {
        assert!(f.update(input, &ctx).is_err());
    }
    assert_eq!(f.state.dudes.soul(&f.code).unwrap().name, seed.name);
    let renamed = f
        .update(json!({"name":"小川"}), &ctx)
        .unwrap();
    assert_eq!(renamed["name"], "小川");
    assert_ne!(renamed["state"], "Ready");
}

#[tokio::test]
async fn valid_answer_initializes_once_and_late_answer_cannot_overwrite_editor() {
    let f = Fixture::new();
    f.state
        .dudes
        .record_soul_question(&f.session, "question")
        .unwrap();
    let mut ctx = f.context("小川，回答简洁");
    let input = json!({"name":"小川","initialSoul":"回答简洁。"});
    ctx.short_term_memory.as_mut().unwrap().input_metadata =
        Some(json!({"resume":[{"interruptId":"wrong-question"}]}));
    assert!(f.update(input.clone(), &ctx).is_err());
    ctx.short_term_memory.as_mut().unwrap().input_metadata =
        Some(json!({"resume":[{"interruptId":"question"}]}));
    assert_eq!(f.update(input.clone(), &ctx).unwrap()["state"], "Ready");
    assert!(
        f.state
            .dudes
            .soul(&f.code)
            .unwrap()
            .pending_interaction_id
            .is_none()
    );
    f.state
        .dudes
        .save_soul(&f.code, "Edited".into(), "编辑后的灵魂".into())
        .unwrap();
    assert!(f.update(input, &ctx).is_err());
    assert_eq!(f.state.dudes.soul(&f.code).unwrap().name, "Edited");
}

#[tokio::test]
async fn soul_updates_require_host_bound_user_input() {
    let f = Fixture::new();
    f.state
        .dudes
        .save_soul(&f.code, "Tron".into(), "原始原则".into())
        .unwrap();
    let request = "把回答简洁写进你的灵魂";
    let input = json!({"edits":[{"old":"","new":"回答简洁"}]});
    for kind in ["DUDE_HEARTBEAT", "DUDE_MEMORYBATCH", "DUDE_THREADFEEDBACK"] {
        let mut ctx = f.context(request);
        ctx.short_term_memory.as_mut().unwrap().input_kind = kind.into();
        assert!(f.update(input.clone(), &ctx).is_err());
    }
    let mut child = f.context(request);
    child.agent_code = Some("child-agent".into());
    assert!(f.update(input.clone(), &child).is_err());
    let mut anonymous = f.context(request);
    anonymous.user_code = None;
    assert!(f.update(input.clone(), &anonymous).is_err());
    let mut tool_output = f.context(request);
    tool_output.short_term_memory.as_mut().unwrap().current_input_message.as_mut().unwrap().role = "tool".into();
    assert!(f.update(input.clone(), &tool_output).is_err());
    let mut missing_input = f.context(request);
    missing_input.short_term_memory.as_mut().unwrap().current_input_message = None;
    assert!(f.update(input.clone(), &missing_input).is_err());
    assert!(f.update(input.clone(), &f.context("  ")).is_err());
    assert_eq!(f.state.dudes.soul(&f.code).unwrap().markdown, "原始原则");
    let saved = f.update(input, &f.context(request)).unwrap();
    assert_eq!(saved["markdown"], "原始原则\n回答简洁");
    assert_eq!(saved["state"], "Ready");
}

#[tokio::test]
async fn soul_edits_use_latest_content_and_fail_atomically_on_stale_or_invalid_patches() {
    let f = Fixture::new();
    f.state
        .dudes
        .save_soul(&f.code, "Tron".into(), "温和\n简洁".into())
        .unwrap();
    let source = "修改灵魂的表达方式";
    let ctx = f.context(source);
    for patch in [
        json!({"edits":[{"old":"缺失原文","new":"简洁"}]}),
        json!({"edits":[{"old":"温和","new":"直接"},{"old":"不存在","new":"新内容"}]}),
        json!({"edits":[{"old":"","new":""}]}),
        json!({"edits":[{"old":"温和","new":"温和"}]}),
        json!({"edits":[{"old":"","new":"字".repeat(6000)}]}),
        json!({"initialSoul":"禁止全量覆盖"}),
    ] {
        assert!(f.update(patch, &ctx).is_err());
        assert_eq!(f.state.dudes.soul(&f.code).unwrap().markdown, "温和\n简洁");
    }
    f.state
        .dudes
        .save_soul(&f.code, "Tron".into(), "温和\n简洁\n编辑页新增原则".into())
        .unwrap();
    let saved = f
        .update(
            json!({"edits":[{"old":"温和","new":"直接"}]}),
            &ctx,
        )
        .unwrap();
    assert_eq!(saved["markdown"], "直接\n简洁\n编辑页新增原则");
    assert_eq!(f.state.dudes.soul(&f.code).unwrap().state, SoulState::Ready);
}

#[tokio::test]
async fn work_operations_reject_missing_fields_at_the_registered_tool_boundary() {
    let f = Fixture::new();
    let ctx = f.context("work");
    for operation in ["list", "recordResult", "record", "resolve"] {
        let error = f
            .call("dude_work_change", json!({"operation":operation}), &ctx)
            .unwrap_err();
        assert!(error.to_string().contains("Invalid work change"), "{error}");
    }
    assert!(
        f.call(
            "dude_notify",
            json!({"changeId":"missing","title":"hi"}),
            &ctx
        )
        .is_err()
    );
}

#[tokio::test]
async fn model_selected_updates_do_not_require_keywords_or_quoted_evidence() {
    let f = Fixture::new();
    let source = "River suits you. Let's go with that.";
    assert_eq!(
        f.update(json!({"name":"River"}), &f.context(source)).unwrap()["name"],
        "River"
    );
    let source = "Garde un ton chaleureux dans nos échanges à venir.";
    assert_eq!(
        f.update(json!({"initialSoul":"Use a warm tone."}), &f.context(source)).unwrap()["state"],
        "Ready"
    );
    let source = "And make that a little more direct going forward.";
    assert_eq!(
        f.update(json!({"edits":[{"old":"warm","new":"direct"}]}), &f.context(source)).unwrap()["markdown"],
        "Use a direct tone."
    );
    assert!(f.update(json!({"name":"Other","userRequest":"approved"}), &f.context(source)).is_err());
}

#[tokio::test]
async fn work_and_notifications_match_schema_character_limits_and_keep_ownership() {
    use tron_backend::application::task::CreateTaskCommand;
    use tron_backend::domain::task::{TaskPriority, TaskSourceType, TaskType};
    let f = Fixture::new();
    let ctx = f.context("work");
    let create = |owner: String| {
        f.state
            .tasks
            .create_task(CreateTaskCommand {
                workspace_code: None,
                owner_dude_code: Some(owner),
                due_at: None,
                parent_task_code: None,
                title: "Tool limit test".into(),
                description: None,
                priority: TaskPriority::Medium,
                task_type: TaskType::General,
                source_type: TaskSourceType::Internal,
                source_url: None,
                agent_code: None,
                model_code: None,
                assignee_user_codes: vec![],
                agent_codes: vec![],
                app_codes: vec![],
            })
            .unwrap()
    };
    let task = create(f.code.clone());
    let record=f.call("dude_work_change",json!({"operation":"record","taskCode":task.task_code,"changeKey":"键".repeat(200),"kind":"NeedsDecision","summary":"摘要".repeat(4000),"evidence":["据".repeat(2000)],"requestedAction":"确认执行时间"}),&ctx).unwrap();
    let id = record["changeId"].as_str().unwrap();
    for (title, body) in [
        ("a".repeat(201), "body".into()),
        ("title".into(), "a".repeat(8001)),
    ] {
        assert!(
            f.call(
                "dude_notify",
                json!({"changeId":id,"title":title,"body":body}),
                &ctx
            )
            .is_err()
        );
    }
    let title = "题".repeat(200);
    let body = "文".repeat(8000);
    assert!(
        f.call(
            "dude_notify",
            json!({"changeId":id,"title":title,"body":body}),
            &ctx
        )
        .is_ok()
    );
    assert!(f.call("dude_work_change",json!({"operation":"resolve","changeId":id,"resolution":"已确认","evidence":["answer:1"]}),&ctx).is_ok());
    assert!(f.call("dude_work_change",json!({"operation":"recordResult","taskCode":task.task_code,"result":"果".repeat(32000)}),&ctx).is_ok());
    assert!(f.call("dude_work_change",json!({"operation":"recordResult","taskCode":task.task_code,"result":"x".repeat(32001)}),&ctx).is_err());
    let other = f
        .state
        .dudes
        .list()
        .unwrap()
        .into_iter()
        .find(|d| d.dude_code != f.code)
        .unwrap();
    let private = create(other.dude_code);
    assert!(
        f.call(
            "dude_work_change",
            json!({"operation":"list","taskCode":private.task_code}),
            &ctx
        )
        .is_err()
    );
}


#[tokio::test]
async fn initial_soul_preserves_onboarding_language_instead_of_interface_or_seed_language() {
    // The model chooses language from the conversation. Persistence must retain its
    // complete Markdown, even when the conversation differs from the en-US UI.
    for (answer, markdown) in [
        ("Call yourself River. Keep replies concise.", "# Personality\nBe warm.\n\n# Expression\nKeep replies concise."),
        ("Appelle-toi River. Réponds avec concision.", "# Personnalité\nSois chaleureux.\n\n# Expression\nRéponds avec concision."),
        ("叫你 River，回答简洁。", "# 性格\n温和。\n\n# 表达\n回答简洁。"),
    ] {
        let f = Fixture::new();
        let context = f.context(answer);
        assert_eq!(context.locale, "en-US");
        let saved = f.update(json!({"name":"River", "initialSoul":markdown}), &context).unwrap();
        assert_eq!(saved["state"], "Ready");
        assert_eq!(saved["name"], "River");
        assert_eq!(saved["markdown"], markdown);
        let read = f.call("dude_soul_read", json!({}), &context).unwrap();
        assert_eq!(read["markdown"], markdown);
    }
}

#[tokio::test]
async fn platform_memory_tools_work_without_workspace_and_keep_scope_boundaries() {
    use tron_backend::application::rag::RagSettings;
    let f = Fixture::new();
    f.state
        .rag
        .configure_corpus(
            "memory-card",
            RagSettings {
                vector: false,
                graph: false,
                ..Default::default()
            },
        )
        .unwrap();
    let mut ctx = f.context("Remember my notification preference");
    ctx.tools = BusinessToolCatalog::list_tools(None, None, "en-US")
        .into_iter()
        .map(tool_record_to_effective_tool)
        .collect();
    let call = |name: &str, input: Value, context: &AgentRunContext| -> Result<Value, ToolError> {
        f.state
            .agent_tool_registry
            .execute_allowed(
                ToolRequest {
                    tool_name: name.into(),
                    input,
                    invocation_id: None,
                },
                context,
            )
            .map(|result| serde_json::from_str(&result.observation.output).unwrap())
    };
    assert!(ctx.workspace_code.is_empty());
    let saved = call(
        "dude_memory_save",
        json!({"text":"One notification per milestone", "semanticKey":"notification-preference"}),
        &ctx,
    )
    .unwrap();
    assert_eq!(saved["scope"], "USER");
    assert_eq!(saved["scopeCode"], "USR-AUTH-0001");
    let code = saved["memoryCode"].as_str().unwrap();
    call(
        "dude_memory_save",
        json!({"memoryCode":code, "text":"Reuse notification receipts across sessions"}),
        &ctx,
    )
    .unwrap();
    call(
        "dude_memory_save",
        json!({"scope":"WORK_DOMAIN", "text":"Shared workflow preference"}),
        &ctx,
    )
    .unwrap();
    let mut space = ctx.clone();
    space.workspace_code = "TRON_CORE".into();
    space.collaboration_owner = None;
    let space_memory = call(
        "memory_save",
        json!({"workspaceCode":"TRON_CORE", "scope":"SPACE", "text":"Workspace private"}),
        &space,
    )
    .unwrap();
    let mut other = ctx.clone();
    other.user_code = Some("USR-OTHER".into());
    call("dude_memory_save", json!({"text":"Other user private"}), &other).unwrap();
    let found = call("dude_memory_search", json!({}), &ctx).unwrap();
    assert_eq!(found["count"], 2, "{found}");
    assert!(
        found["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["scope"] != "SPACE")
    );
    let pack = call(
        "dude_memory_search",
        json!({"mode":"CONTEXT_PACK", "scope":"USER"}),
        &ctx,
    )
    .unwrap();
    assert_eq!(pack["count"], 1, "{pack}");
    assert!(pack["contextPack"]["items"].is_array());
    for input in [
        json!({"scope":"SPACE"}),
        json!({"scope":"SPACE", "scopeCode":"TRON_CORE"}),
        json!({"memoryCode":space_memory["memoryCode"]}),
    ] {
        assert!(call("dude_memory_search", input, &ctx).is_err());
    }
    for input in [
        json!({"scope":"SPACE", "text":"Invalid"}),
        json!({"scope":"SPACE", "scopeCode":"TRON_CORE", "text":"Invalid"}),
        json!({"workspaceCode":"TRON_CORE", "text":"Invalid"}),
    ] {
        assert!(call("dude_memory_save", input, &ctx).is_err());
    }
    assert!(
        call(
            "dude_memory_delete",
            json!({"memoryCode":space_memory["memoryCode"], "reason":"Outside scope"}),
            &ctx
        )
        .is_err()
    );
    call(
        "dude_memory_delete",
        json!({"memoryCode":code, "reason":"Superseded"}),
        &ctx,
    )
    .unwrap();
    assert_eq!(
        call("dude_memory_search", json!({"memoryCode":code}), &ctx).unwrap()["count"],
        0
    );
    ctx.user_code = None;
    assert!(
        call("dude_memory_search", json!({}), &ctx)
            .unwrap_err()
            .to_string()
            .contains("userCode")
    );
}

#[tokio::test]
async fn dude_task_tools_require_owner_and_do_not_accept_workspace() {
    let f = Fixture::new();
    let mut context = f.context("Manage my tasks");
    context.tools = BusinessToolCatalog::list_tools(None, None, "en-US")
        .into_iter().map(tool_record_to_effective_tool).collect();
    let call = |name: &str, input: Value, ctx: &AgentRunContext| {
        f.state.agent_tool_registry.execute_allowed(ToolRequest {
            tool_name: name.into(), input, invocation_id: None,
        }, ctx)
    };
    let created = call("dude_task_create", json!({"title":"Owned task"}), &context).unwrap();
    let result: Value = serde_json::from_str(&created.observation.output).unwrap();
    assert_eq!(result["ownerDudeCode"], f.code);
    assert!(result["workspaceCode"].is_null());
    let task_code = result["taskCode"].as_str().expect("created task code");
    let updated = call("dude_task_update", json!({"taskCode":task_code, "title":"Updated task"}), &context).unwrap();
    let updated: Value = serde_json::from_str(&updated.observation.output).unwrap();
    assert_eq!(updated["title"], "Updated task");
    assert!(call("dude_task_page", json!({}), &context).is_ok());
    assert!(call("dude_task_delete", json!({"taskCode":task_code}), &context).is_ok());
    assert!(call("dude_task_restore", json!({"taskCode":task_code}), &context).is_ok());
    assert!(call("dude_task_create", json!({"title":"Bad", "workspaceCode":"TRON_CORE"}), &context).is_err());
    assert!(call("task_create", json!({"title":"Bad", "workspaceCode":"TRON_CORE"}), &context).is_err());
    let mut other = context.clone();
    other.collaboration_owner = None;
    assert!(call("dude_task_page", json!({}), &other).is_err());
}

#[tokio::test]
async fn progress_notifications_reuse_the_result_across_sessions_and_can_be_resolved() {
    use tron_backend::application::task::UpdateTaskStatusCommand;
    let f = Fixture::new();
    let ctx = f.context("Send one test notification");
    let task = f.task();
    f.call("dude_work_change", json!({"operation":"recordResult", "taskCode":task, "result":"Notification delivery is ready for verification"}), &ctx).unwrap();
    let record = |key: &str, context: &AgentRunContext| {
        f.call("dude_work_change", json!({"operation":"record", "taskCode":task, "changeKey":key, "kind":"Progress", "summary":"Verify delivery", "evidence":[format!("event:{key}")]}), context).unwrap()
    };
    let first = record("interactive-delivery", &ctx);
    let mut background = ctx.clone();
    background.conversation_code = Some("background-task-session".into());
    background
        .collaboration_owner
        .as_mut()
        .unwrap()
        .origin_session_code = background.conversation_code.clone();
    f.state
        .tasks
        .update_status(
            &task,
            UpdateTaskStatusCommand {
                status: "IN_PROGRESS".into(),
                progress: Some(20),
            },
        )
        .unwrap();
    let second = record("reminder-delivery", &background);
    assert_eq!(first["changeId"], second["changeId"]);
    let notify = |id: &Value| {
        f.call(
            "dude_notify",
            json!({"changeId":id, "title":"Test", "body":"Delivery test"}),
            &ctx,
        )
        .unwrap()
    };
    let notice = notify(&second["changeId"]);
    f.state
        .tasks
        .update_status(
            &task,
            UpdateTaskStatusCommand {
                status: "IN_PROGRESS".into(),
                progress: Some(60),
            },
        )
        .unwrap();
    let third = record("heartbeat-delivery", &background);
    assert_eq!(third["notification"]["id"], notice["id"]);
    assert_eq!(notify(&third["changeId"])["id"], notice["id"]);
    assert_eq!(f.state.dudes.notifications().unwrap().len(), 1);
    f.state
        .tasks
        .update_status(
            &task,
            UpdateTaskStatusCommand {
                status: "CLOSED".into(),
                progress: None,
            },
        )
        .unwrap();
    f.state
        .dudes
        .update_notification(notice["id"].as_str().unwrap(), true, true)
        .unwrap();
    let resolved = f.call("dude_work_change", json!({"operation":"resolve", "changeId":first["changeId"], "resolution":"User confirmed delivery", "evidence":["interaction:receipt"]}), &ctx).unwrap();
    assert!(resolved["resolvedAt"].is_number());
    let retry = f.call("dude_work_change", json!({"operation":"resolve", "changeId":first["changeId"], "resolution":"Retry", "evidence":["interaction:receipt"]}), &ctx).unwrap();
    assert_eq!(retry["resolvedAt"], resolved["resolvedAt"]);
    let completed = f.call("dude_work_change", json!({"operation":"record", "taskCode":task, "changeKey":"completed-delivery", "kind":"Completed", "summary":"Delivery verified", "evidence":["interaction:receipt"]}), &ctx).unwrap();
    notify(&completed["changeId"]);
    f.call("dude_work_change", json!({"operation":"resolve", "changeId":completed["changeId"], "resolution":"Acknowledged", "evidence":["interaction:receipt"]}), &ctx).unwrap();
    let notices = f.state.dudes.notifications().unwrap();
    assert_eq!(notices.len(), 2);
    assert!(notices.iter().all(|notice| notice.resolved_at.is_some()));
    let archived = notices
        .iter()
        .find(|item| Some(item.id.as_str()) == notice["id"].as_str())
        .unwrap();
    assert!(archived.read_at.is_some());
    assert!(archived.archived_at.is_some());
    assert_eq!(notify(&first["changeId"])["id"], notice["id"]);
    assert_eq!(f.state.dudes.notifications().unwrap().len(), 2);
}

#[tokio::test]
async fn a_new_concrete_result_is_a_new_milestone_and_stale_results_cannot_notify() {
    let f = Fixture::new();
    let ctx = f.context("Report real results");
    let task = f.task();
    let result = |text: &str| {
        f.call(
            "dude_work_change",
            json!({"operation":"recordResult", "taskCode":task, "result":text}),
            &ctx,
        )
        .unwrap()
    };
    let record = |key: &str| {
        f.call("dude_work_change", json!({"operation":"record", "taskCode":task, "changeKey":key, "kind":"Progress", "summary":"Milestone", "evidence":["artifact:1"]}), &ctx).unwrap()
    };
    result("First deliverable");
    let first = record("first");
    result("Second deliverable");
    let second = record("second");
    assert_ne!(first["changeId"], second["changeId"]);
    assert!(
        f.call(
            "dude_notify",
            json!({"changeId":first["changeId"], "title":"Old", "body":"Stale"}),
            &ctx
        )
        .is_err()
    );
    f.call(
        "dude_notify",
        json!({"changeId":second["changeId"], "title":"New", "body":"Current"}),
        &ctx,
    )
    .unwrap();
    assert_eq!(f.state.dudes.notifications().unwrap().len(), 1);
    for key in ["deployment-window", "access-approval"] {
        let decision = f.call("dude_work_change", json!({
            "operation":"record", "taskCode":task, "changeKey":key,
            "kind":"NeedsDecision", "summary":key, "requestedAction":key,
            "evidence":[format!("interaction:{key}")],
        }), &ctx).unwrap();
        f.call("dude_notify", json!({
            "changeId":decision["changeId"], "title":key, "body":key,
        }), &ctx).unwrap();
    }
    assert_eq!(f.state.dudes.notifications().unwrap().len(), 3,
        "distinct decisions must not be merged merely because they share a task/result");
}


#[tokio::test]
async fn notification_content_conflicts_are_explicit_and_reads_use_current_scoped_state() {
    let f = Fixture::new();
    let context = f.context("Check notification receipt");
    let task = f.task();
    let change = f.call("dude_work_change", json!({"operation":"record","taskCode":task,"changeKey":"receipt","kind":"NeedsDecision","summary":"Choose the deployment window","evidence":["decision:window"],"requestedAction":"Choose a window"}), &context).unwrap();
    let payload = json!({"changeId":change["changeId"],"title":" Title ","body":"  Exact body\nwith whitespace  "});
    let notice = f.call("dude_notify", payload.clone(), &context).unwrap();
    assert_eq!(notice["title"], payload["title"]);
    assert_eq!(notice["body"], payload["body"]);
    assert_eq!(f.call("dude_notify", payload.clone(), &context).unwrap()["id"], notice["id"]);
    let mut conflicting = payload.clone(); conflicting["body"] = json!("Changed content");
    assert!(f.call("dude_notify", conflicting, &context).unwrap_err().to_string().contains("NOTIFICATION_CONTENT_CONFLICT"));
    f.state.dudes.update_notification(notice["id"].as_str().unwrap(), true, true).unwrap();
    let read = f.call("dude_notification_get",json!({"notificationId":notice["id"]}),&context).unwrap();
    assert!(read["readAt"].is_number());
    assert!(read["archivedAt"].is_number());
    assert!(read["resolvedAt"].is_null());
    assert_eq!(read["delivery"],json!({"persisted":true,"received":null,"pushed":null}));
    let page = f.call("dude_notification_page",json!({}),&context).unwrap();
    assert_eq!(page["total"],0);
    let page = f.call("dude_notification_page",json!({"includeArchived":true}),&context).unwrap();
    assert_eq!(page["total"],1);
    let mut foreign = context.clone(); foreign.user_code = Some("another-user".into());
    assert_eq!(f.call("dude_notification_page",json!({"includeArchived":true}),&foreign).unwrap()["total"],0);
    assert!(f.call("dude_notification_get",json!({"notificationId":notice["id"]}),&foreign).is_err());
    assert!(f.call("dude_notification_page",json!({"size":101}),&context).is_err());
    f.state.dudes.delete_archived_notification(notice["id"].as_str().unwrap()).unwrap();
    assert!(f.call("dude_notification_get",json!({"notificationId":notice["id"]}),&context).is_err());
    assert_eq!(f.call("dude_notify",payload,&context).unwrap()["id"],notice["id"]);
    assert!(f.state.dudes.notifications().unwrap().is_empty());
}

#[tokio::test]
async fn task_flow_is_discoverable_and_reopening_resets_progress_and_followup() {
    let f = Fixture::new();
    let mut context = f.context("Reopen my task");
    context.tools = BusinessToolCatalog::list_tools(None,None,"en-US").into_iter().map(tool_record_to_effective_tool).collect();
    let call = |name: &str, input: Value| {
        let output = f.state.agent_tool_registry.execute_allowed(ToolRequest { tool_name:name.into(),input,invocation_id:None },&context)?;
        Ok::<Value,ToolError>(serde_json::from_str(&output.observation.output).unwrap())
    };
    let task = f.task();
    let detail = call("dude_task_get",json!({"taskCode":task})).unwrap();
    assert_eq!(detail["taskFlow"]["closedStatus"],"CLOSED");
    assert!(detail["taskFlow"]["statuses"].as_array().unwrap().iter().any(|s| s["statusCode"] == "IN_PROGRESS"));
    let error = call("dude_task_update",json!({"taskCode":task,"status":"done"})).unwrap_err().to_string();
    assert!(error.contains("Valid statuses:") && error.contains("IN_PROGRESS") && error.contains("CLOSED"));
    call("dude_task_update",json!({"taskCode":task,"status":"completed"})).unwrap();
    let closed = f.state.tasks.get_task(&task).unwrap(); assert_eq!(closed.progress,100); assert!(closed.next_followup_at.is_none());
    let reopened = call("dude_task_update",json!({"taskCode":task,"status":"IN_PROGRESS"})).unwrap();
    assert_eq!(reopened["progress"],0);
    assert!(f.state.tasks.get_task(&task).unwrap().next_followup_at.is_some());
    assert!(call("dude_task_get",json!({"taskCode":task,"workspaceCode":"foreign"})).is_err());
}

#[tokio::test]
async fn similar_milestones_warn_but_distinct_results_are_not_merged() {
    let f=Fixture::new(); let context=f.context("Report two results"); let task=f.task();
    let record=|key:&str| f.call("dude_work_change",json!({"operation":"record","taskCode":task,"changeKey":key,"kind":"NeedsDecision","summary":"Choose the deployment window","evidence":[key],"requestedAction":key}),&context).unwrap();
    let first=record("window-one"); let second=record("window-two");
    assert_ne!(first["changeId"],second["changeId"]);
    assert_eq!(second["warnings"][0]["code"],"SIMILAR_WORK_CHANGE");
    assert_eq!(second["warnings"][0]["existingChangeId"],first["changeId"]);
    assert!(f.state.dudes.notifications().unwrap().is_empty());
}
