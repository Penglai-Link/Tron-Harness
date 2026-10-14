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
            tools: BusinessToolCatalog::list_tools(Some("dude"), None, "en-US")
                .into_iter()
                .map(tool_record_to_effective_tool)
                .collect(),
            effective_tools: vec![],
            tool_assignment: None,
            tool_exposure: Default::default(),
            execution_scope: Default::default(),
            limits: Default::default(),
            messages: vec![],
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
