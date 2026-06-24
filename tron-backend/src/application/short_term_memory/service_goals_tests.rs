use crate::{
    adapters::outbound::persistence::SqliteShortTermMemoryRepository,
    application::{
        chat::ChatRepository,
        short_term_memory::{
            SetGoalCommand, ShortTermMemoryConfig, ShortTermMemoryService, TodoUpdateItem,
        },
    },
    domain::agent::{
        AgentExecutionScope, AgentKind, AgentMessage, AgentModelConfig, AgentModelProtocol,
        AgentRunContext, AgentRunLimits, AgentShortTermMemoryRunContext, AgentToolExposureState,
        EffectiveSkill, EffectiveTool, EffectiveVariable, ModelResponse, ToolObservation,
    },
    shared::{id::FixedSnowflakeIdProvider, time::current_timestamp},
};

use std::sync::Arc;

#[test]
fn llm_goal_judge_request_is_stateless_and_tool_free() {
    let service = service("goal-judge-request");
    let input = service
        .prepare_goal_judge_input("SESSION-1", "RUN-1", "继续执行", 1)
        .expect("judge input should prepare")
        .expect("active goal should exist");
    let model = service.goal_judge_model_config(&model_config());
    let request = service.goal_judge_model_request(&input);

    assert!(request.tools.is_empty());
    assert!(request.previous_response_id.is_none());
    assert!(request.system_agent_session_id.is_none());
    assert!(!model.thinking_enabled);
    assert!(!model.responses_stateful_enabled);
    assert_eq!(
        model.max_tokens,
        Some(
            ShortTermMemoryConfig::default()
                .goal_loop
                .judge_output_max_tokens
        )
    );
}

#[test]
fn llm_goal_judge_requires_continuation_prompt_for_continue() {
    let service = service("goal-judge-parse");
    let response = ModelResponse::final_output(
        r#"{"verdict":"CONTINUE","reason":"more work remains","completionEvidence":[],"missingEvidence":[],"requiredUserInput":null,"nextStepHint":null,"continuationPrompt":null}"#,
    );

    let failure = service
        .model_goal_judge_output(&response)
        .expect_err("continue without prompt should fail");
    assert_eq!(failure.error_code, "GOAL_JUDGE_MODEL_SCHEMA_INVALID");
}

#[test]
fn llm_goal_judge_parses_completed_json() {
    let service = service("goal-judge-completed");
    let response = ModelResponse::final_output(
        r#"```json
{"verdict":"COMPLETED","reason":"all todos are completed","completionEvidence":["activeTodos is empty"],"missingEvidence":[],"requiredUserInput":null,"nextStepHint":null,"continuationPrompt":null}
```"#,
    );

    let output = service
        .model_goal_judge_output(&response)
        .expect("completed JSON should parse");
    assert_eq!(output.verdict, "COMPLETED");
    assert_eq!(output.continuation_prompt, None);
}

#[test]
fn llm_goal_judge_stops_repeated_no_progress_continue() {
    let service = service("goal-judge-no-progress");
    let final_output = "10 次循环测试完成";
    let first_input = service
        .prepare_goal_judge_input("SESSION-1", "RUN-1", final_output, 1)
        .expect("judge input should prepare")
        .expect("active goal should exist");
    let first_response = ModelResponse::final_output(
        r#"{"verdict":"CONTINUE","reason":"check once more","completionEvidence":[],"missingEvidence":["judge requested continuation"],"requiredUserInput":null,"nextStepHint":"avoid repetition","continuationPrompt":"Check whether any concrete work remains."}"#,
    );
    let first_output = service
        .model_goal_judge_output(&first_response)
        .expect("first continue should parse");
    let first_decision = service
        .commit_goal_judge_decision(first_input, first_output, &model_config(), &first_response)
        .expect("first judge decision should commit");
    assert_eq!(first_decision.verdict, "CONTINUE");
    assert!(first_decision.continuation_prompt.is_some());

    let repeated_input = service
        .prepare_goal_judge_input("SESSION-1", "RUN-2", final_output, 1)
        .expect("repeated judge input should prepare")
        .expect("active goal should still exist before guard commit");
    assert!(repeated_input.no_progress_detected);
    let repeated_response = ModelResponse::final_output(
        r#"{"verdict":"CONTINUE","reason":"still continuing","completionEvidence":[],"missingEvidence":["judge repeated continuation"],"requiredUserInput":null,"nextStepHint":"avoid repetition","continuationPrompt":"Check whether any concrete work remains."}"#,
    );
    let repeated_output = service
        .model_goal_judge_output(&repeated_response)
        .expect("repeated continue should parse");
    let repeated_decision = service
        .commit_goal_judge_decision(
            repeated_input,
            repeated_output,
            &model_config(),
            &repeated_response,
        )
        .expect("repeated judge decision should commit");

    assert_eq!(repeated_decision.verdict, "SKIPPED");
    assert_eq!(repeated_decision.reason, "no_progress_detected");
    assert_eq!(repeated_decision.continuation_prompt, None);
    let goal = service
        .get_active_goal("SESSION-1")
        .expect("goal lookup should work")
        .expect("paused goal remains visible as active goal view");
    assert_eq!(goal.goal.status, "PAUSED");
}

fn service(label: &str) -> ShortTermMemoryService {
    let database_url = temp_database_url(label);
    crate::adapters::outbound::persistence::SqliteChatRepository::connect(&database_url)
        .expect("chat repository should connect")
        .create_session(&crate::domain::chat::ChatSession {
            id: 1,
            session_code: "SESSION-1".into(),
            workspace_code: "TRON_CORE".into(),
            user_code: None,
            app_code: None,
            task_code: None,
            source_type: crate::domain::chat::ChatSessionSourceType::Manual,
            source_code: None,
            agent_code: Some("main-agent".into()),
            system_agent_session_id: None,
            title: "goal judge".into(),
            status: crate::domain::chat::ChatSessionStatus::Active,
            last_run_code: None,
            last_run_status: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
        })
        .expect("session should be inserted");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let service = ShortTermMemoryService::with_id_provider(
        repository,
        ShortTermMemoryConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(910_000_000_000_000_000)),
    );
    service
        .set_goal(SetGoalCommand {
            session_code: "SESSION-1".into(),
            objective: "完成一个待办循环测试".into(),
            max_runs: Some(3),
            criteria: Vec::new(),
        })
        .expect("goal should be set");
    let mut context = base_context();
    let input_code = service
        .record_user_input(&context, "开始测试")
        .expect("input should record");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: "SESSION-1".into(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix: vec![AgentMessage::system("system")],
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });
    service
        .record_run_started(&context)
        .expect("run should start");
    service
        .replace_todos(
            &context,
            vec![TodoUpdateItem {
                todo_code: Some("TODO-1".into()),
                position: 0,
                content: "循环测试".into(),
                status: "completed".into(),
                priority: "high".into(),
            }],
        )
        .expect("todo should update");
    service
}

fn base_context() -> AgentRunContext {
    AgentRunContext {
        conversation_code: Some("SESSION-1".into()),
        run_code: "RUN-1".into(),
        workspace_code: "TRON_CORE".into(),
        user_code: None,
        locale: "en-US".into(),
        request_id: None,
        stream_key: None,
        agent_kind: AgentKind::React,
        system_agent_session_id: None,
        model: model_config(),
        variables: Vec::<EffectiveVariable>::new(),
        skills: Vec::<EffectiveSkill>::new(),
        tools: Vec::<EffectiveTool>::new(),
        tool_exposure: AgentToolExposureState::default(),
        execution_scope: AgentExecutionScope::default(),
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("开始测试")],
        observations: Vec::<ToolObservation>::new(),
        short_term_memory: None,
        started_at: current_timestamp(),
    }
}

fn model_config() -> AgentModelConfig {
    AgentModelConfig {
        model_code: "MODEL-1".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "http://127.0.0.1".into(),
        api_path: None,
        api_key: "test".into(),
        model_name: "test-model".into(),
        thinking_enabled: true,
        responses_stateful_enabled: true,
        temperature: Some(0.7),
        max_tokens: Some(8192),
        context_window_tokens: Some(32_000),
        top_p: Some(0.9),
        max_retries: Some(0),
    }
}

fn temp_database_url(label: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("{label}-{}-{now}.sqlite3", std::process::id()))
        .display()
        .to_string()
}
