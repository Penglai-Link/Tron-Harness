use std::{fs, path::Path, process::Command, sync::Arc, time::Duration};

use serde_json::json;
use tron_backend::{
    adapters::outbound::persistence::{SqliteChatRepository, SqliteShortTermMemoryRepository},
    application::{
        chat::{ChatService, ChatSessionQuery, CreateChatSessionCommand},
        short_term_memory::{
            CreateCompactionCommand, GoalSessionCommand, ModelCompactionOutput,
            ModelRequestTranscriptFacts, ModelSessionSummaryOutput, PreparedCompactionInput,
            SessionSummaryFailure, SessionSummaryJobContext, SessionSummaryRunOutcome,
            SetGoalCommand, ShortTermMemoryConfig, ShortTermMemoryRepository,
            ShortTermMemoryService, TodoUpdateItem, ToolOutputConfig, ToolTranscriptStatus,
        },
    },
    domain::{
        agent::{
            AgentExecutionScope, AgentKind, AgentMessage, AgentMessagePart, AgentModelConfig,
            AgentModelProtocol, AgentRunContext, AgentRunLimits, AgentShortTermMemoryRunContext,
            EffectiveSkill, EffectiveTool, EffectiveVariable, ModelFinish, ModelResponse,
            ModelTokenUsage, TokenEstimateSnapshot, TokenUsageAuditSnapshot, ToolCall,
            ToolObservation,
        },
        chat::ChatSession,
        short_term_memory::{
            ShortTermMemoryCompaction, ShortTermMemoryError, ShortTermMemoryEvent,
            ShortTermMemoryGoalEvent, ShortTermMemoryMessage, ShortTermMemoryPart,
            ShortTermMemoryTodo, TodoPriority, TodoStatus,
        },
    },
    shared::{
        event_bus::{EventBusConfig, EventBusRuntimeBuilder, RecordingEventSink},
        id::FixedSnowflakeIdProvider,
        time::current_timestamp,
    },
};

#[test]
fn context_assembly_publishes_agui_event_when_detailed_event_persistence_is_disabled() {
    let database_url = temp_database_url("short-term-memory-agui-context");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("AG-UI context status".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let id_provider = Arc::new(FixedSnowflakeIdProvider::new(801_000_000_000_000_000));
    let recording_sink = Arc::new(RecordingEventSink::new("stm_agui_context"));
    let event_bus = EventBusRuntimeBuilder::new(EventBusConfig::default(), id_provider.clone())
        .register_sink(recording_sink.clone())
        .build();
    let mut config = ShortTermMemoryConfig::default();
    config.event_log.mode = "lifecycleOnly".into();
    let service = ShortTermMemoryService::with_id_provider(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
        id_provider,
    )
    .with_event_bus(event_bus);

    let mut context = base_context(&session.session_code);
    context.tools.clear();
    let input_code = service
        .record_user_input(&context, "show context status")
        .expect("input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    service
        .prepare_model_context(&context)
        .expect("context should assemble");

    assert!(recording_sink.wait_for_len(1, Duration::from_millis(300)));
    let events = recording_sink.events();
    let event = events
        .iter()
        .find(|event| event.event_name == "agent.context.assembled")
        .expect("context assembly should reach the AG-UI event bus");
    assert_eq!(event.stream_key(), Some(session.session_code.as_str()));
    assert_eq!(
        event
            .subject
            .as_ref()
            .and_then(|subject| subject.subject_code.as_deref()),
        Some(context.run_code.as_str())
    );
    let payload = event
        .payload_fields
        .iter()
        .find(|field| field.key == "context_event_json")
        .expect("context payload should be present");
    let payload = serde_json::from_str::<serde_json::Value>(&payload.value)
        .expect("context payload should be valid JSON");
    assert!(payload.get("budgetBreakdown").is_some());

    let connection = rusqlite::Connection::open(&database_url)
        .expect("short-term memory database should be queryable");
    let persisted_detail_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE event_type = 'agent.context.assembled'",
            [],
            |row| row.get(0),
        )
        .expect("context event count should be queryable");
    assert_eq!(persisted_detail_count, 0);
}

#[test]
fn failed_tool_calls_and_discovery_results_replay_to_the_next_run() {
    let database_url = temp_database_url("short-term-memory-tool-replay");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("tool replay".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );

    let mut first_context = base_context(&session.session_code);
    first_context.run_code = "RUN-TOOL-REPLAY-1".into();
    let first_input = service
        .record_user_input(&first_context, "create a script")
        .expect("first input should be recorded");
    first_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code: first_input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_model_response(
            &first_context,
            &ModelResponse::from_tool_calls(vec![
                ToolCall {
                    call_id: Some("call-write".into()),
                    tool_name: "fs_write_text".into(),
                    input: json!({
                        "path": "long_running_test.sh",
                        "relativeTo": "workspaceRoot",
                        "text": "echo test"
                    }),
                    input_error: None,
                },
                ToolCall {
                    call_id: Some("call-discovery".into()),
                    tool_name: "discovery_expand_tool_domain".into(),
                    input: json!({"domainCode": "fs"}),
                    input_error: None,
                },
            ]),
        )
        .expect("tool calls should be recorded");
    service
        .record_tool_result(
            &first_context,
            &ToolObservation {
                tool_name: "fs_write_text".into(),
                output: "tool input is invalid: missing field `content`".into(),
            },
            Some("call-write".into()),
            1,
            ToolTranscriptStatus::Failed,
        )
        .expect("failed tool result should be recorded");
    service
        .record_tool_result(
            &first_context,
            &ToolObservation {
                tool_name: "discovery_expand_tool_domain".into(),
                output: json!({
                    "domain": {"domainCode": "fs"},
                    "tools": [{"toolName": "fs_write_text"}]
                })
                .to_string(),
            },
            Some("call-discovery".into()),
            2,
            ToolTranscriptStatus::Completed,
        )
        .expect("discovery result should be recorded");

    let mut second_context = base_context(&session.session_code);
    second_context.run_code = "RUN-TOOL-REPLAY-2".into();
    let second_input = service
        .record_user_input(&second_context, "continue")
        .expect("second input should be recorded");
    second_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code,
        input_code: second_input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let prepared = service
        .prepare_model_context(&second_context)
        .expect("next run context should assemble");

    assert!(prepared.messages.iter().flat_map(|message| &message.parts).any(
        |part| matches!(part, AgentMessagePart::ToolCall { tool_name, .. } if tool_name == "fs_write_text")
    ));
    assert!(prepared.messages.iter().flat_map(|message| &message.parts).any(
        |part| matches!(part, AgentMessagePart::ToolResult { tool_name, output, .. } if tool_name == "fs_write_text" && output.contains("missing field"))
    ));
    assert!(prepared.messages.iter().flat_map(|message| &message.parts).any(
        |part| matches!(part, AgentMessagePart::ToolResult { tool_name, output, .. } if tool_name == "discovery_expand_tool_domain" && output.contains("fs_write_text"))
    ));
}

#[test]
fn transcript_service_records_todos_and_compaction() {
    let database_url = temp_database_url("short-term-memory-transcript");
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("short-term memory".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let artifact_root = temp_artifact_root("short-term-memory-tool-output");
    let mut memory_config = ShortTermMemoryConfig::default();
    memory_config.tool_output = ToolOutputConfig {
        root: artifact_root.display().to_string(),
        externalize_above_bytes: 8,
    };
    memory_config
        .context
        .blocks
        .session_family_context
        .max_ratio_bps = Some(10_000);
    memory_config.context.family_context_max_chars = 12_000;
    memory_config.family_load.default_snapshot_max_chars = 512;
    let service = ShortTermMemoryService::with_id_provider(
        repository,
        memory_config,
        Arc::new(FixedSnowflakeIdProvider::new(800_000_000_000_000_000)),
    );

    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "Track the implementation state")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });
    service
        .record_run_started(&context)
        .expect("run start should be recorded");
    service
        .replace_todos(
            &context,
            vec![TodoUpdateItem {
                todo_code: None,
                position: 0,
                content: "Verify transcript rows".into(),
                status: "in_progress".into(),
                priority: "high".into(),
            }],
        )
        .expect("todo update should be recorded");
    service
        .record_model_response(
            &context,
            &ModelResponse::from_tool_calls(vec![ToolCall {
                call_id: Some("call-1".into()),
                tool_name: "todo_update".into(),
                input: json!({"todos": []}),
                input_error: None,
            }]),
        )
        .expect("tool call response should be recorded");
    let large_output = "line-one\nline-two\nline-three";
    let tool_message = service
        .record_tool_result(
            &context,
            &ToolObservation {
                tool_name: "shell.exec".into(),
                output: large_output.into(),
            },
            Some("call-1".into()),
            1,
            ToolTranscriptStatus::Completed,
        )
        .expect("large tool output should be recorded");
    let inline_tool_output = "rawtool";
    service
        .record_tool_result(
            &context,
            &ToolObservation {
                tool_name: "shell.exec".into(),
                output: inline_tool_output.into(),
            },
            Some("call-2".into()),
            2,
            ToolTranscriptStatus::Completed,
        )
        .expect("small inline tool output should be recorded");
    let AgentMessagePart::ToolResult {
        output,
        externalized,
        metadata,
        ..
    } = &tool_message.parts[0]
    else {
        panic!("tool result part should be returned");
    };
    assert!(*externalized);
    assert!(output.contains("\"externalized\":true"));
    let relative_path = metadata
        .as_ref()
        .and_then(|value| value.get("relativePath"))
        .and_then(|value| value.as_str())
        .expect("externalized result should include relative path");
    let root_path = metadata
        .as_ref()
        .and_then(|value| value.get("rootPath"))
        .and_then(|value| value.as_str())
        .expect("externalized result should include root path");
    assert_eq!(root_path, artifact_root.display().to_string());
    assert!(output.contains("\"rootPath\""));
    assert!(output.contains(root_path));
    assert!(output.contains(relative_path));
    assert!(!output.contains(large_output));
    assert_eq!(
        fs::read_to_string(artifact_root.join(relative_path))
            .expect("externalized tool output should exist"),
        large_output
    );
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let (stored_text, stored_json): (String, String) = connection
        .query_row(
            "SELECT COALESCE(content_text, ''), content_json \
             FROM agent_transcript_parts \
             WHERE part_type = 'tool_result'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("tool result part should be persisted");
    assert!(stored_text.contains("\"relativePath\""));
    assert!(stored_text.contains("\"rootPath\""));
    assert!(stored_text.contains(root_path));
    assert!(stored_text.contains(relative_path));
    assert!(stored_json.contains("\"externalized\":true"));
    assert!(!stored_json.contains(large_output));
    service
        .record_model_response(
            &context,
            &ModelResponse::final_output_with_usage_and_session_and_response_id(
                "Done",
                Some(ModelTokenUsage {
                    input_tokens: 10,
                    output_tokens: 4,
                    total_tokens: 14,
                    cached_input_tokens: None,
                    reasoning_tokens: None,
                }),
                None,
                None,
            ),
        )
        .expect("final output should be recorded");
    service
        .record_run_completed(&context)
        .expect("run completion should be recorded");
    let assistant_completed_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE event_type = 'agent.message.assistant_completed'",
            [],
            |row| row.get(0),
        )
        .expect("assistant completed event should be queryable");
    assert_eq!(assistant_completed_count, 1);
    let legacy_completed_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE event_type = 'agent.message.completed'",
            [],
            |row| row.get(0),
        )
        .expect("legacy assistant event should be queryable");
    assert_eq!(legacy_completed_count, 0);
    let summary_count_before_background_job: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_session_family_snapshots WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("session summaries should be queryable");
    assert_eq!(summary_count_before_background_job, 0);

    let summary_job = service
        .prepare_session_summary_job(
            session_summary_context(&context),
            completed_summary_outcome(),
        )
        .expect("session summary job should prepare")
        .expect("new run should create a session summary job");
    let summary_input = service
        .session_summary_model_request(
            summary_job,
            "Summarize only the supplied evidence as Markdown.".into(),
            1,
        )
        .expect("session summary model input should prepare")
        .expect("session summary should not be superseded");
    let summary_model = service.session_summary_model_config(&context.model);
    assert_eq!(summary_model.temperature, Some(0.0));
    assert!(!summary_model.thinking_enabled);
    assert!(!summary_model.responses_stateful_enabled);
    assert_eq!(summary_model.max_tokens, Some(1_024));
    assert_eq!(summary_model.max_retries, Some(0));
    assert!(summary_input.request.tools.is_empty());
    assert_eq!(summary_input.request.messages.len(), 2);
    let summary_evidence = summary_input.request.messages[1].text_content();
    assert!(summary_evidence.contains("Track the implementation state"));
    assert!(summary_evidence.contains("shell.exec"));
    assert!(summary_evidence.contains(inline_tool_output));
    assert!(!summary_evidence.contains(large_output));
    assert!(!summary_evidence.contains("system seed"));
    assert!(!summary_evidence.contains("raw reasoning"));
    service
        .commit_session_summary(
            &summary_input,
            ModelSessionSummaryOutput {
                markdown: "# Session Summary\n\n## Status\nRaw transcript text was summarized; raw reasoning are intentionally omitted."
                    .into(),
                token_usage: None,
                provider_metadata: None,
            },
            &summary_model,
        )
        .expect("session summary should commit");
    let session_summary: String = connection
        .query_row(
            "SELECT summary_markdown FROM agent_session_family_snapshots WHERE session_code = ?1 AND snapshot_kind = 'SESSION_SUMMARY' AND status = 'COMPLETED' ORDER BY source_end_seq DESC, id DESC LIMIT 1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("session summary should be queryable");
    assert!(session_summary.contains("# Session Summary"));
    assert!(session_summary.contains("Raw transcript text"));
    assert!(session_summary.contains("raw reasoning are intentionally omitted"));
    assert!(!session_summary.contains("Track the implementation state"));
    assert!(!session_summary.contains(inline_tool_output));
    assert!(!session_summary.contains(large_output));
    let mut compaction_seed_context = base_context(&session.session_code);
    for index in 0..6 {
        compaction_seed_context.run_code = format!("RUN-STMEM-MANUAL-SEED-{index}");
        service
            .record_user_input(
                &compaction_seed_context,
                &format!("manual compaction seed {index} {}", "S".repeat(2_200)),
            )
            .expect("manual compaction seed input should be recorded");
    }

    let todos = service
        .list_todos(&session.session_code)
        .expect("todos should load");
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].content, "Verify transcript rows");

    let compaction = create_stub_model_compaction(
        &service,
        CreateCompactionCommand {
            session_code: session.session_code.clone(),
            reason: Some("manual".into()),
            note: None,
            todo_enabled: None,
        },
        "## Goal\nTrack implementation state\n\n## Current State\nTodo remains active.\n\n## Tools And Results\nshell.exec output was captured.\n\n## Decisions\nUse model-generated checkpoint summary.",
    );
    assert_eq!(compaction.session_code, session.session_code);
    assert!(compaction.source_end_seq >= compaction.source_start_seq);

    let latest = service
        .latest_compaction(&session.session_code)
        .expect("latest compaction should load")
        .expect("latest compaction should exist");
    assert_eq!(
        latest.checkpoint_message_code,
        compaction.checkpoint_message_code
    );
    let compaction_summary: String = connection
        .query_row(
            "SELECT content_text FROM agent_transcript_parts WHERE message_code = ?1 AND part_type = 'compaction_summary'",
            [compaction.checkpoint_message_code.as_str()],
            |row| row.get(0),
        )
        .expect("compaction summary should be persisted");
    assert!(compaction_summary.contains("## Goal"));
    assert!(compaction_summary.contains("## Current State"));
    assert!(compaction_summary.contains("## Tools And Results"));
    assert!(compaction_summary.contains("Use model-generated checkpoint summary."));
    assert!(!compaction_summary.contains("deterministic checkpoint pass"));
    let prepared_after_compaction = service
        .prepare_model_context(&context)
        .expect("compaction summary should rehydrate into model context");
    let rendered_after_compaction = prepared_after_compaction
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");
    assert!(rendered_after_compaction.contains("Conversation checkpoint:"));
    assert!(rendered_after_compaction.contains("## Goal"));
    assert!(!rendered_after_compaction.contains("Previous conversation was compacted."));
    let compaction_recent_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_parts WHERE message_code = ?1 AND part_type = 'compaction_recent'",
            [compaction.checkpoint_message_code.as_str()],
            |row| row.get(0),
        )
        .expect("compaction recent part should be queryable");
    assert_eq!(compaction_recent_count, 1);
    let compaction_started_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE event_type = 'agent.compaction.started'",
            [],
            |row| row.get(0),
        )
        .expect("compaction started event should be queryable");
    assert!(compaction_started_count > 0);
    let main_agent_message_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND agent_code = 'main-agent'
               AND message_type IN ('normal', 'tool_result')",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("agent code should be queryable");
    assert!(main_agent_message_count > 0);
    for index in 0..6 {
        compaction_seed_context.run_code = format!("RUN-STMEM-MANUAL-SEED-SECOND-{index}");
        service
            .record_user_input(
                &compaction_seed_context,
                &format!(
                    "second manual compaction seed {index} {}",
                    "T".repeat(2_200)
                ),
            )
            .expect("second manual compaction seed input should be recorded");
    }
    let second_compaction = create_stub_model_compaction(
        &service,
        CreateCompactionCommand {
            session_code: session.session_code.clone(),
            reason: Some("second manual".into()),
            note: None,
            todo_enabled: None,
        },
        "## Goal\nTrack implementation state\n\n## Critical Facts\nPrevious checkpoint summary:\nTrack implementation state\n\nPrevious retained recent transcript:\nshell.exec output was captured.\n\n## Decisions\nCarry prior checkpoint context forward.",
    );
    let second_compaction_summary: String = connection
        .query_row(
            "SELECT content_text FROM agent_transcript_parts WHERE message_code = ?1 AND part_type = 'compaction_summary'",
            [second_compaction.checkpoint_message_code.as_str()],
            |row| row.get(0),
        )
        .expect("second compaction summary should be persisted");
    assert!(second_compaction_summary.contains("Previous checkpoint summary:"));
    assert!(second_compaction_summary.contains("Previous retained recent transcript:"));
    assert!(second_compaction_summary.contains("Carry prior checkpoint context forward."));
    for index in 0..6 {
        compaction_seed_context.run_code = format!("RUN-STMEM-MANUAL-SEED-TODO-DISABLED-{index}");
        service
            .record_user_input(
                &compaction_seed_context,
                &format!(
                    "todo-disabled manual compaction seed {index} {}",
                    "U".repeat(2_200)
                ),
            )
            .expect("todo-disabled manual compaction seed input should be recorded");
    }
    let todo_disabled_compaction = create_stub_model_compaction(
        &service,
        CreateCompactionCommand {
            session_code: session.session_code.clone(),
            reason: Some("todo-disabled".into()),
            note: None,
            todo_enabled: Some(false),
        },
        "## Goal\nTrack implementation state\n\n## Critical Facts\nPrevious checkpoint summary:\nTrack implementation state\n\nPrevious retained recent transcript:\nshell.exec output was captured.\n\n## Decisions\nTodo sections redacted.",
    );
    let todo_disabled_summary: String = connection
        .query_row(
            "SELECT content_text FROM agent_transcript_parts WHERE message_code = ?1 AND part_type = 'compaction_summary'",
            [todo_disabled_compaction.checkpoint_message_code.as_str()],
            |row| row.get(0),
        )
        .expect("todo-disabled compaction summary should be persisted");
    assert!(todo_disabled_summary.contains("Previous checkpoint summary:"));
    assert!(todo_disabled_summary.contains("Previous retained recent transcript:"));
    assert!(!todo_disabled_summary.contains("## Current State"));
    assert!(!todo_disabled_summary.contains("## Completed Work"));
    assert!(!todo_disabled_summary.contains("Verify transcript rows"));
    connection
        .execute(
            "INSERT INTO agent_transcript_messages (id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, created_at)
             VALUES (900000000000000001, 'ATM-FAILED-LATEST', ?1, 'TRON_CORE', NULL, 999999, 'checkpoint', 'compaction', 'failed', 0)",
            [session.session_code.as_str()],
        )
        .expect("failed compaction message should be insertable");
    connection
        .execute(
            "INSERT INTO agent_transcript_parts (id, part_code, session_code, workspace_code, message_code, seq, part_type, status, content_text, content_json, visibility, created_at)
             VALUES (900000000000000002, 'ATP-FAILED-LATEST', ?1, 'TRON_CORE', 'ATM-FAILED-LATEST', 1, 'compaction_summary', 'completed', 'failed checkpoint', '{\"sourceStartSeq\":1,\"sourceEndSeq\":999999,\"tailStartSeq\":999999,\"selectorVersion\":2,\"coverage\":\"full_source_includes_tail\"}', 'model', 0)",
            [session.session_code.as_str()],
        )
        .expect("failed compaction part should be insertable");
    let latest_after_failed = service
        .latest_compaction(&session.session_code)
        .expect("latest compaction should load after failed candidate")
        .expect("completed compaction should still exist");
    assert_eq!(
        latest_after_failed.checkpoint_message_code,
        todo_disabled_compaction.checkpoint_message_code
    );

    let second_session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("second short-term memory".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("second session should be created");
    let mut second_context = base_context(&second_session.session_code);
    second_context.run_code = "RUN-STMEM-2".into();
    let second_input_code = service
        .record_user_input(&second_context, "Continue the implementation")
        .expect("second input should be recorded");
    second_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: second_session.session_code.clone(),
        input_code: second_input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });
    service
        .record_run_started(&second_context)
        .expect("second run start should upsert family membership");
    service
        .set_goal(SetGoalCommand {
            session_code: second_session.session_code.clone(),
            objective: "Finish the complete short-term memory design".into(),
            max_runs: Some(5),
        })
        .expect("goal should be set");
    service
        .replace_todos(
            &second_context,
            vec![
                TodoUpdateItem {
                    todo_code: Some("TODO-ACTIVE".into()),
                    position: 0,
                    content: "Inspect family context".into(),
                    status: "in_progress".into(),
                    priority: "high".into(),
                },
                TodoUpdateItem {
                    todo_code: Some("TODO-DONE".into()),
                    position: 1,
                    content: "Do not inject completed todos".into(),
                    status: "completed".into(),
                    priority: "low".into(),
                },
            ],
        )
        .expect("mixed todo update should be recorded");
    let active_state_session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("active state only".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("active-state session should be created");
    let mut active_state_context = base_context(&active_state_session.session_code);
    active_state_context.run_code = "RUN-STMEM-3".into();
    let active_state_input_code = service
        .record_user_input(&active_state_context, "Keep active state visible")
        .expect("active-state input should be recorded");
    active_state_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: active_state_session.session_code.clone(),
        input_code: active_state_input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });
    service
        .record_run_started(&active_state_context)
        .expect("active-state run start should upsert family membership");
    service
        .set_goal(SetGoalCommand {
            session_code: active_state_session.session_code.clone(),
            objective: "Active state without snapshot".into(),
            max_runs: Some(5),
        })
        .expect("active-state goal should be set");
    service
        .replace_todos(
            &active_state_context,
            vec![TodoUpdateItem {
                todo_code: Some("TODO-ACTIVE-STATE".into()),
                position: 0,
                content: "Carry active state without snapshot".into(),
                status: "in_progress".into(),
                priority: "medium".into(),
            }],
        )
        .expect("active-state todos should be recorded");
    let judge_input = service
        .prepare_goal_judge_input(
            &second_session.session_code,
            &second_context.run_code,
            "Continue with the next concrete implementation step.",
            1,
        )
        .expect("goal judge input should prepare")
        .expect("active goal should produce judge input");
    let judge_response = ModelResponse::final_output(
        r#"{"verdict":"CONTINUE","reason":"active todo remains","completionEvidence":[],"missingEvidence":["active todo remains"],"requiredUserInput":null,"nextStepHint":"carry active state","continuationPrompt":"Continue with the next concrete implementation step."}"#,
    );
    let judge_output = service
        .model_goal_judge_output(&judge_response)
        .expect("goal judge output should parse");
    let decision = service
        .commit_goal_judge_decision(
            judge_input,
            judge_output,
            &second_context.model,
            &judge_response,
        )
        .expect("goal judge decision should commit");
    assert_eq!(decision.verdict, "CONTINUE");
    let continuation_prompt = decision
        .continuation_prompt
        .expect("continue verdict should create a continuation prompt");
    second_context.run_code = "RUN-STMEM-CONTINUATION".into();
    let continuation_input = service
        .record_invocation_input(
            &second_context,
            &continuation_prompt,
            "CONTINUATION",
            Some(json!({"goalCode": decision.goal_code, "source": "GOAL_LOOP"})),
        )
        .expect("continuation input should be recorded");
    let (input_kind, input_metadata): (String, String) = connection
        .query_row(
            "SELECT input_kind, metadata_json FROM agent_transcript_inputs WHERE input_code = ?1",
            [continuation_input.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("continuation input should be persisted");
    assert_eq!(input_kind, "CONTINUATION");
    assert!(input_metadata.contains("GOAL_LOOP"));
    let continuation_user_message_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND run_code = ?2
               AND role = 'user'
               AND message_type = 'normal'",
            [
                second_session.session_code.as_str(),
                second_context.run_code.as_str(),
            ],
            |row| row.get(0),
        )
        .expect("continuation transcript messages should be queryable");
    assert_eq!(continuation_user_message_count, 0);
    let (continuation_event_source, continuation_event_payload, continuation_event_message_code): (
        String,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT event_source, payload_json, message_code FROM agent_transcript_events WHERE input_code = ?1 AND event_type = 'agent.input.consumed'",
            [continuation_input.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("continuation input event should be persisted");
    assert_eq!(continuation_event_source, "RUNTIME");
    assert!(continuation_event_payload.contains("GOAL_LOOP"));
    assert!(continuation_event_payload.contains("runtime_context_block"));
    assert!(continuation_event_message_code.is_none());
    let prepared = service
        .prepare_model_context(&second_context)
        .expect("prepared context should load family and goal state");
    let rendered = prepared
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");
    assert!(rendered.contains("<session-family-context>"));
    assert!(rendered.contains("<load-plan total_token_budget=\""));
    assert!(rendered.contains("token_budget="));
    assert!(rendered.contains("# Session Summary"));
    assert!(rendered.contains("Raw transcript text"));
    assert!(!rendered.contains("Track the implementation state"));
    assert!(!rendered.contains(inline_tool_output));
    assert!(rendered.contains("<current-session-working-state>"));
    assert!(rendered.contains("Finish the complete short-term memory design"));
    assert!(rendered.contains("Inspect family context"));
    assert!(!rendered.contains("Do not inject completed todos"));
    let session_summary_without_fixed_expiration_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_session_family_snapshots WHERE session_code = ?1 AND snapshot_kind = 'SESSION_SUMMARY' AND expires_at IS NULL",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("session summary expiration should be queryable");
    assert!(session_summary_without_fixed_expiration_count > 0);
    connection
        .execute(
            "UPDATE agent_session_family_snapshots SET completed_at = 0 WHERE session_code = ?1 AND snapshot_kind = 'SESSION_SUMMARY'",
            [session.session_code.as_str()],
        )
        .expect("session summary should be markable as stale");
    let rendered_after_expiry = service
        .prepare_model_context(&second_context)
        .expect("prepared context should skip expired snapshots")
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");
    assert!(!rendered_after_expiry.contains("Raw transcript text"));
    assert!(rendered_after_expiry.contains("Active state without snapshot"));
    service
        .record_model_requested(
            &second_context,
            ModelRequestTranscriptFacts {
                estimated_tokens: prepared.estimated_tokens,
                message_hashes: vec!["hash".into(); prepared.messages.len()],
                tool_schema_token_estimate: 12,
                protocol: "chatCompletions".into(),
                previous_response_id_present: false,
                context_window_estimate: Some(8_192),
                checkpoint_code: prepared.checkpoint_code.clone(),
                budget_breakdown: None,
                over_budget_reason: None,
                usage_estimate: None,
            },
        )
        .expect("model request event should be recorded");
    let model_requested_payload: String = connection
        .query_row(
            "SELECT payload_json FROM agent_transcript_events WHERE session_code = ?1 AND event_type = 'agent.model.requested' ORDER BY id DESC LIMIT 1",
            [second_session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("model request event payload should be persisted");
    assert!(model_requested_payload.contains("\"toolSchemaTokenEstimate\":12"));
    assert!(model_requested_payload.contains("\"protocol\":\"chatCompletions\""));
    assert!(model_requested_payload.contains("\"previousResponseIdPresent\":false"));
    assert!(model_requested_payload.contains("\"contextWindowEstimate\":8192"));
    assert!(model_requested_payload.contains("\"checkpointCode\""));
    let context_epoch_snapshot: String = connection
        .query_row(
            "SELECT snapshot_json FROM agent_transcript_context_epochs WHERE session_code = ?1 ORDER BY revision DESC LIMIT 1",
            [second_session.session_code.as_str()],
            |row| row.get(0),
        )
    .expect("context epoch should be persisted");
    assert!(context_epoch_snapshot.contains("promptPrefix"));
    assert!(context_epoch_snapshot.contains("exposedTools"));
    assert!(context_epoch_snapshot.contains("effectiveTools"));
    assert!(context_epoch_snapshot.contains("budgetPlan"));
    assert!(context_epoch_snapshot.contains("policyHash"));
    let context_event_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_transcript_events WHERE session_code = ?1 AND event_type IN ('agent.context.baseline_created', 'agent.context.assembled', 'agent.model.requested')",
            [second_session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("context events should be queryable");
    assert!(context_event_count >= 3);

    let heartbeat_session = service
        .ensure_heartbeat_session("TRON_CORE", "main-agent")
        .expect("heartbeat hidden session should be ensured");
    let (heartbeat_source, heartbeat_visibility): (String, String) = connection
        .query_row(
            "SELECT source, visibility FROM chat_sessions WHERE session_code = ?1",
            [heartbeat_session.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("heartbeat session should be queryable");
    assert_eq!(heartbeat_source, "HEARTBEAT");
    assert_eq!(heartbeat_visibility, "HIDDEN");
    let visible_groups = chats
        .list_session_groups(ChatSessionQuery {
            workspace_code: Some("TRON_CORE".into()),
        })
        .expect("visible session groups should load");
    assert!(
        !visible_groups
            .iter()
            .flat_map(|group| group.sessions.iter())
            .any(|session| session.session_code == heartbeat_session),
        "hidden heartbeat sessions must not appear in normal chat lists"
    );

    service
        .recover_goal_state_on_startup()
        .expect("recovery should pause ambiguous in-flight goals");
    let recovered_goal_status: String = connection
        .query_row(
            "SELECT status FROM agent_session_goals WHERE session_code = ?1 ORDER BY updated_at DESC LIMIT 1",
            [second_session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("recovered goal should be queryable");
    assert_eq!(recovered_goal_status, "PAUSED");
    let recovery_event_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_session_goal_events WHERE session_code = ?1 AND event_kind = 'RECOVERY_REQUIRED'",
            [second_session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("recovery goal event should be queryable");
    assert_eq!(recovery_event_count, 1);
}

#[test]
fn session_summary_storage_failure_is_separate_from_run_completion() {
    let database_url = temp_database_url("short-term-memory-family-failure");
    let chat_repository =
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect");
    let chats = ChatService::new(chat_repository);
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("family failure".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let service = ShortTermMemoryService::new(repository, ShortTermMemoryConfig::default());
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "Do not block completion on snapshot failure")
        .expect("input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });
    service
        .record_run_started(&context)
        .expect("run start should create family membership");
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    connection
        .execute_batch(
            "CREATE TRIGGER fail_session_summary_snapshot
             BEFORE INSERT ON agent_session_family_snapshots
             WHEN NEW.snapshot_kind = 'SESSION_SUMMARY'
             BEGIN
               SELECT RAISE(ABORT, 'session summary snapshot blocked');
             END;",
        )
        .expect("failure trigger should be installable");

    service
        .record_run_completed(&context)
        .expect("run completion should not depend on session summary storage");

    let summary_error = match service.prepare_session_summary_job(
        session_summary_context(&context),
        completed_summary_outcome(),
    ) {
        Ok(_) => panic!("session summary persistence should fail"),
        Err(error) => error,
    };
    assert_eq!(summary_error, ShortTermMemoryError::StorageFailure);
    let run_completed_event_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.run.completed'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("run completion event should be queryable");
    assert_eq!(run_completed_event_count, 1);
}

#[test]
fn session_summary_jobs_are_session_owned_incremental_and_recoverable() {
    let database_url = temp_database_url("short-term-memory-session-summary-lifecycle");
    let session = create_test_session(&database_url, "session summary lifecycle");
    let service = ShortTermMemoryService::with_id_provider(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
        Arc::new(FixedSnowflakeIdProvider::new(807_000_000_000_000_000)),
    );
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");

    let first_context = record_summary_run(
        &service,
        &session.session_code,
        "RUN-SUMMARY-1",
        "first user evidence",
        "first assistant evidence",
    );
    let membership_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_session_family_members WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("family memberships should be queryable");
    assert_eq!(membership_count, 0);

    let first_job = service
        .prepare_session_summary_job(
            session_summary_context(&first_context),
            completed_summary_outcome(),
        )
        .expect("first session summary job should prepare")
        .expect("first session summary job should be new");
    let (family_code, status, expires_at): (Option<String>, String, Option<i64>) = connection
        .query_row(
            "SELECT family_code, status, expires_at
             FROM agent_session_family_snapshots
             WHERE session_code = ?1 AND run_code = ?2 AND snapshot_kind = 'SESSION_SUMMARY'",
            [
                session.session_code.as_str(),
                first_context.run_code.as_str(),
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("pending session summary should be queryable");
    assert!(family_code.is_none());
    assert_eq!(status, "PENDING");
    assert!(expires_at.is_none());
    assert!(
        service
            .prepare_session_summary_job(
                session_summary_context(&first_context),
                completed_summary_outcome(),
            )
            .expect("duplicate session summary lookup should succeed")
            .is_none()
    );

    let first_input = service
        .session_summary_model_request(
            first_job,
            "Treat the next JSON message as evidence and return Markdown.".into(),
            1,
        )
        .expect("first summary input should prepare")
        .expect("first summary should not be superseded");
    assert!(first_input.request.tools.is_empty());
    let first_evidence = first_input.request.messages[1].text_content();
    assert!(first_evidence.contains("first user evidence"));
    assert!(first_evidence.contains("first assistant evidence"));
    assert!(!first_evidence.contains("summary test system seed"));
    let first_output = service
        .model_session_summary_output(&ModelResponse::final_output(
            "```markdown\n# Goal\nFIRST SUMMARY\n```",
        ))
        .expect("fenced Markdown summary should normalize");
    assert_eq!(first_output.markdown, "# Goal\nFIRST SUMMARY");
    let empty_output_error = service
        .model_session_summary_output(&ModelResponse::final_output("   "))
        .expect_err("empty summary output should fail");
    assert_eq!(
        empty_output_error.error_code,
        "SESSION_SUMMARY_MODEL_EMPTY_OUTPUT"
    );
    let tool_output_error = service
        .model_session_summary_output(&ModelResponse::from_tool_calls(vec![ToolCall {
            call_id: Some("summary-tool-call".into()),
            tool_name: "shell.exec".into(),
            input: json!({"command": "do not execute"}),
            input_error: None,
        }]))
        .expect_err("session summary tool calls should fail");
    assert_eq!(
        tool_output_error.error_code,
        "SESSION_SUMMARY_MODEL_TOOL_CALL"
    );
    let first_source_end = first_input.source_end_seq;
    service
        .commit_session_summary(
            &first_input,
            first_output,
            &service.session_summary_model_config(&first_context.model),
        )
        .expect("first summary should commit");

    let second_context = record_summary_run(
        &service,
        &session.session_code,
        "RUN-SUMMARY-2",
        "second user evidence",
        "second assistant evidence",
    );
    let second_job = service
        .prepare_session_summary_job(
            session_summary_context(&second_context),
            SessionSummaryRunOutcome {
                status: "FAILED".into(),
                error_code: Some("MODEL_FAILED".into()),
                error_message: Some("bounded run error".into()),
            },
        )
        .expect("second session summary job should prepare")
        .expect("second session summary job should be new");
    let second_input = service
        .session_summary_model_request(second_job, "Return Markdown.".into(), 1)
        .expect("second summary input should prepare")
        .expect("second summary should not be superseded");
    assert_eq!(second_input.source_start_seq, first_source_end + 1);
    service
        .fail_session_summary(
            &second_input.job,
            Some(&second_input),
            SessionSummaryFailure {
                error_code: "SESSION_SUMMARY_MODEL_FAILED".into(),
                message: "model failed without affecting the run".into(),
            },
        )
        .expect("failed summary should persist its own status");

    let third_context = record_summary_run(
        &service,
        &session.session_code,
        "RUN-SUMMARY-3",
        "third user evidence",
        "third assistant evidence",
    );
    let third_job = service
        .prepare_session_summary_job(
            session_summary_context(&third_context),
            completed_summary_outcome(),
        )
        .expect("third session summary job should prepare")
        .expect("third session summary job should be new");
    let third_input = service
        .session_summary_model_request(third_job, "Return Markdown.".into(), 1)
        .expect("third summary input should prepare")
        .expect("third summary should not be superseded");
    assert_eq!(third_input.source_start_seq, first_source_end + 1);
    let third_evidence = third_input.request.messages[1].text_content();
    assert!(third_evidence.contains("FIRST SUMMARY"));
    assert!(third_evidence.contains("second user evidence"));
    assert!(third_evidence.contains("third user evidence"));
    service
        .commit_session_summary(
            &third_input,
            ModelSessionSummaryOutput {
                markdown: "# Goal\nTHIRD SUMMARY".into(),
                token_usage: None,
                provider_metadata: None,
            },
            &service.session_summary_model_config(&third_context.model),
        )
        .expect("third summary should commit");

    let fourth_context = record_summary_run(
        &service,
        &session.session_code,
        "RUN-SUMMARY-4",
        "fourth user evidence",
        "fourth assistant evidence",
    );
    let fourth_input = service
        .session_summary_model_request(
            service
                .prepare_session_summary_job(
                    session_summary_context(&fourth_context),
                    completed_summary_outcome(),
                )
                .expect("fourth job should prepare")
                .expect("fourth job should be new"),
            "Return Markdown.".into(),
            1,
        )
        .expect("fourth input should prepare")
        .expect("fourth summary should not be superseded");
    let fifth_context = record_summary_run(
        &service,
        &session.session_code,
        "RUN-SUMMARY-5",
        "fifth user evidence",
        "fifth assistant evidence",
    );
    let fifth_input = service
        .session_summary_model_request(
            service
                .prepare_session_summary_job(
                    session_summary_context(&fifth_context),
                    completed_summary_outcome(),
                )
                .expect("fifth job should prepare")
                .expect("fifth job should be new"),
            "Return Markdown.".into(),
            1,
        )
        .expect("fifth input should prepare")
        .expect("fifth summary should not be superseded");
    service
        .commit_session_summary(
            &fifth_input,
            ModelSessionSummaryOutput {
                markdown: "# Goal\nFIFTH SUMMARY".into(),
                token_usage: None,
                provider_metadata: None,
            },
            &service.session_summary_model_config(&fifth_context.model),
        )
        .expect("fifth summary should commit first");
    service
        .commit_session_summary(
            &fourth_input,
            ModelSessionSummaryOutput {
                markdown: "# Goal\nFOURTH SUMMARY".into(),
                token_usage: None,
                provider_metadata: None,
            },
            &service.session_summary_model_config(&fourth_context.model),
        )
        .expect("fourth summary should commit out of order");
    let latest_run_code: String = connection
        .query_row(
            "SELECT run_code
             FROM agent_session_family_snapshots
             WHERE session_code = ?1
               AND snapshot_kind = 'SESSION_SUMMARY'
               AND status = 'COMPLETED'
             ORDER BY source_end_seq DESC, id DESC
             LIMIT 1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("latest session summary should be queryable");
    assert_eq!(latest_run_code, fifth_context.run_code);

    let mut empty_context = base_context(&session.session_code);
    empty_context.run_code = "RUN-SUMMARY-EMPTY".into();
    let empty_job = service
        .prepare_session_summary_job(
            session_summary_context(&empty_context),
            completed_summary_outcome(),
        )
        .expect("empty run summary job should prepare")
        .expect("empty run summary job should be new");
    record_summary_run(
        &service,
        &session.session_code,
        "RUN-SUMMARY-LATER",
        "later user evidence must not leak backward",
        "later assistant evidence must not leak backward",
    );
    let empty_input = service
        .session_summary_model_request(empty_job, "Return Markdown.".into(), 1)
        .expect("empty run summary input should prepare")
        .expect("empty run summary should not be superseded");
    assert_eq!(empty_input.source_end_seq, fifth_input.source_end_seq);
    let empty_evidence = empty_input.request.messages[1].text_content();
    assert!(!empty_evidence.contains("later user evidence must not leak backward"));
    assert!(!empty_evidence.contains("later assistant evidence must not leak backward"));
    service
        .commit_session_summary(
            &empty_input,
            ModelSessionSummaryOutput {
                markdown: "# Goal\nEMPTY RUN SUMMARY".into(),
                token_usage: None,
                provider_metadata: None,
            },
            &service.session_summary_model_config(&empty_context.model),
        )
        .expect("empty run summary should commit without later evidence");

    let sixth_context = record_summary_run(
        &service,
        &session.session_code,
        "RUN-SUMMARY-6",
        "sixth user evidence",
        "sixth assistant evidence",
    );
    service
        .prepare_session_summary_job(
            session_summary_context(&sixth_context),
            SessionSummaryRunOutcome {
                status: "CANCELLED".into(),
                error_code: Some("AGENT_RUN_STOPPED".into()),
                error_message: Some("cancelled by user".into()),
            },
        )
        .expect("sixth session summary job should prepare")
        .expect("sixth session summary job should be new");
    assert_eq!(
        service
            .recover_session_summaries_on_startup()
            .expect("stale summary recovery should succeed"),
        1
    );
    let (recovered_status, recovered_error_code): (String, Option<String>) = connection
        .query_row(
            "SELECT status, error_code
             FROM agent_session_family_snapshots
             WHERE session_code = ?1 AND run_code = ?2 AND snapshot_kind = 'SESSION_SUMMARY'",
            [
                session.session_code.as_str(),
                sixth_context.run_code.as_str(),
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("recovered session summary should be queryable");
    assert_eq!(recovered_status, "FAILED");
    assert_eq!(
        recovered_error_code.as_deref(),
        Some("SESSION_SUMMARY_INTERRUPTED")
    );
    assert_eq!(
        service
            .recover_session_summaries_on_startup()
            .expect("repeated recovery should be idempotent"),
        0
    );
}

#[test]
fn goal_continuation_admission_rechecks_active_goal_state() {
    let database_url = temp_database_url("short-term-memory-goal-admission");
    let session = create_test_session(&database_url, "goal admission");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let goal = service
        .set_goal(SetGoalCommand {
            session_code: session.session_code.clone(),
            objective: "Continue only while active".into(),
            max_runs: Some(3),
        })
        .expect("goal should be set");

    assert!(
        service
            .should_admit_goal_continuation(&session.session_code, &goal.goal.goal_code)
            .expect("active goal admission should be queryable")
    );

    service
        .pause_goal(GoalSessionCommand {
            session_code: session.session_code.clone(),
            goal_code: Some(goal.goal.goal_code.clone()),
        })
        .expect("goal should pause");

    assert!(
        !service
            .should_admit_goal_continuation(&session.session_code, &goal.goal.goal_code)
            .expect("paused goal admission should be queryable")
    );
}

#[test]
fn family_context_deduplicates_sessions_across_families_and_counts_wrapper_budget() {
    let database_url = temp_database_url("short-term-memory-family-dedupe");
    let prior_session = create_test_session(&database_url, "prior family context");
    let mut config = ShortTermMemoryConfig::default();
    config.context.family_context_max_chars = 900;
    config.family_load.default_snapshot_max_chars = 500;
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config.clone(),
    );

    let mut prior_context = base_context(&prior_session.session_code);
    prior_context.execution_scope.task_code = Some("TASK-FAMILY-DEDUPE".into());
    let prior_input = service
        .record_user_input(&prior_context, "build a dedupe family snapshot")
        .expect("prior input should be recorded");
    prior_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: prior_session.session_code.clone(),
        input_code: prior_input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_run_started(&prior_context)
        .expect("prior memberships should be recorded");
    service
        .record_model_response(
            &prior_context,
            &ModelResponse::final_output(format!(
                "family dedupe marker {}",
                "large snapshot body ".repeat(80)
            )),
        )
        .expect("prior response should be recorded");
    service
        .record_run_completed(&prior_context)
        .expect("prior run completion should be recorded");
    create_stub_session_summary(
        &service,
        &prior_context,
        &format!(
            "# Session Summary\n\n## Status\nfamily dedupe marker {}",
            "large snapshot body ".repeat(80)
        ),
    );

    let current_session = create_test_session(&database_url, "current family context");
    let mut current_context = base_context(&current_session.session_code);
    current_context.run_code = "RUN-FAMILY-DEDUPE-CURRENT".into();
    current_context.execution_scope.task_code = Some("TASK-FAMILY-DEDUPE".into());
    current_context.model.context_window_tokens = Some(10_000);
    current_context.model.max_tokens = Some(0);
    let current_input = service
        .record_user_input(&current_context, "load deduped family context")
        .expect("current input should be recorded");
    current_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: current_session.session_code.clone(),
        input_code: current_input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_run_started(&current_context)
        .expect("current memberships should be recorded");

    let messages = service
        .prepare_model_context(&current_context)
        .expect("family context should prepare")
        .messages;
    let rendered = messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n");
    let family_context = messages
        .iter()
        .map(AgentMessage::text_content)
        .find(|message| message.contains("<session-family-context>"))
        .expect("family context message should be present");

    assert!(rendered.contains("<session-family-context>"));
    assert_eq!(rendered.matches(&prior_session.session_code).count(), 1);
    assert!(
        family_context.len() <= config.context.family_context_max_chars,
        "family context should count wrappers inside the configured max chars"
    );
}

#[test]
fn family_context_prefers_session_summary_and_falls_back_to_legacy_handoff() {
    let database_url = temp_database_url("short-term-memory-family-legacy-fallback");
    let prior_session = create_test_session(&database_url, "legacy handoff source");
    let current_session = create_test_session(&database_url, "legacy handoff consumer");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );

    let mut prior_context = base_context(&prior_session.session_code);
    prior_context.execution_scope.task_code = Some("TASK-LEGACY-FALLBACK".into());
    let prior_input = service
        .record_user_input(&prior_context, "legacy family source")
        .expect("prior input should be recorded");
    attach_short_term_memory(
        &mut prior_context,
        &prior_session.session_code,
        prior_input,
        vec![AgentMessage::system("system seed")],
    );
    service
        .record_run_started(&prior_context)
        .expect("prior task family should be recorded");

    let mut current_context = base_context(&current_session.session_code);
    current_context.run_code = "RUN-LEGACY-FALLBACK-CURRENT".into();
    current_context.execution_scope.task_code = Some("TASK-LEGACY-FALLBACK".into());
    let current_input = service
        .record_user_input(&current_context, "load related work")
        .expect("current input should be recorded");
    attach_short_term_memory(
        &mut current_context,
        &current_session.session_code,
        current_input,
        vec![AgentMessage::system("system seed")],
    );
    service
        .record_run_started(&current_context)
        .expect("current task family should be recorded");

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let task_family_code: String = connection
        .query_row(
            "SELECT f.family_code
             FROM agent_session_families f
             JOIN agent_session_family_members m
               ON m.family_code = f.family_code
             WHERE m.session_code = ?1
               AND f.family_kind = 'TASK'
             LIMIT 1",
            [prior_session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("shared task family should be queryable");
    connection
        .execute(
            "INSERT INTO agent_session_family_snapshots (
                id, snapshot_code, family_code, session_code, run_code,
                snapshot_kind, status, source_start_seq, source_end_seq,
                summary_markdown, token_estimate, metadata_json, created_at,
                completed_at, expires_at
             ) VALUES (
                910210100000000001,
                'AFS-LEGACY-FALLBACK', ?1, ?2, ?3,
                'RUN_HANDOFF', 'COMPLETED', 1, 2,
                '# Legacy Handoff\nLEGACY FALLBACK MARKER', 8, '{}',
                CAST(strftime('%s','now') AS INTEGER) * 1000,
                CAST(strftime('%s','now') AS INTEGER) * 1000,
                NULL
             )",
            [
                task_family_code.as_str(),
                prior_session.session_code.as_str(),
                prior_context.run_code.as_str(),
            ],
        )
        .expect("legacy handoff should be insertable");

    let fallback_context = rendered_messages(
        &service
            .prepare_model_context(&current_context)
            .expect("legacy family handoff should load")
            .messages,
    );
    assert!(fallback_context.contains("LEGACY FALLBACK MARKER"));

    let summary_context = record_summary_run(
        &service,
        &prior_session.session_code,
        "RUN-SESSION-SUMMARY-PREFERRED",
        "new session summary evidence",
        "new session summary result",
    );
    create_stub_session_summary(
        &service,
        &summary_context,
        "# Session Summary\nSESSION SUMMARY PREFERRED MARKER",
    );
    let preferred_context = rendered_messages(
        &service
            .prepare_model_context(&current_context)
            .expect("session summary should load")
            .messages,
    );
    assert!(preferred_context.contains("SESSION SUMMARY PREFERRED MARKER"));
    assert!(!preferred_context.contains("LEGACY FALLBACK MARKER"));
}

#[test]
fn schema_guard_backfills_legacy_short_term_memory_columns() {
    let database_url = temp_database_url("short-term-memory-schema-guard");
    {
        let connection =
            rusqlite::Connection::open(&database_url).expect("legacy database should open");
        connection
            .execute_batch(
                "CREATE TABLE agent_transcript_parts (
                    id INTEGER PRIMARY KEY,
                    part_code TEXT NOT NULL UNIQUE,
                    message_code TEXT NOT NULL,
                    seq INTEGER NOT NULL,
                    part_type TEXT NOT NULL,
                    status TEXT NOT NULL,
                    content_text TEXT,
                    content_json TEXT,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE agent_transcript_messages (
                    id INTEGER PRIMARY KEY,
                    message_code TEXT NOT NULL UNIQUE,
                    session_code TEXT NOT NULL,
                    seq INTEGER NOT NULL,
                    role TEXT NOT NULL,
                    message_type TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE agent_transcript_context_epochs (
                    id INTEGER PRIMARY KEY,
                    epoch_code TEXT NOT NULL UNIQUE,
                    session_code TEXT NOT NULL,
                    workspace_code TEXT NOT NULL,
                    baseline_seq INTEGER NOT NULL,
                    snapshot_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );",
            )
            .expect("legacy stm tables should be creatable");
    }

    SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("schema guard should backfill legacy stm columns");
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let part_columns = table_columns(&connection, "agent_transcript_parts");
    assert!(part_columns.contains(&"session_code".to_string()));
    assert!(part_columns.contains(&"workspace_code".to_string()));
    assert!(part_columns.contains(&"provider_metadata_json".to_string()));
    assert!(part_columns.contains(&"visibility".to_string()));
    assert!(part_columns.contains(&"completed_at".to_string()));
    let message_columns = table_columns(&connection, "agent_transcript_messages");
    assert!(message_columns.contains(&"workspace_code".to_string()));
    let epoch_columns = table_columns(&connection, "agent_transcript_context_epochs");
    assert!(epoch_columns.contains(&"replacement_seq".to_string()));
    assert!(epoch_columns.contains(&"prompt_prefix_hash".to_string()));
    assert!(epoch_columns.contains(&"revision".to_string()));
    assert!(epoch_columns.contains(&"updated_at".to_string()));
}

#[test]
fn schema_guard_resequences_legacy_goal_events_after_event_seq_backfill() {
    let database_url = temp_database_url("short-term-memory-goal-event-seq-backfill");
    let session = create_test_session(&database_url, "goal event seq backfill");
    {
        let connection =
            rusqlite::Connection::open(&database_url).expect("legacy database should open");
        connection
            .execute_batch(&format!(
                "CREATE TABLE agent_session_goals (
                    id INTEGER PRIMARY KEY,
                    goal_code TEXT NOT NULL UNIQUE,
                    session_code TEXT NOT NULL,
                    workspace_code TEXT NOT NULL,
                    status TEXT NOT NULL,
                    revision INTEGER NOT NULL DEFAULT 0,
                    last_input_code TEXT,
                    updated_at INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE agent_session_goal_events (
                    id INTEGER PRIMARY KEY,
                    event_code TEXT NOT NULL UNIQUE,
                    goal_code TEXT NOT NULL,
                    session_code TEXT NOT NULL,
                    workspace_code TEXT NOT NULL,
                    event_kind TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                INSERT INTO agent_session_goals
                (id, goal_code, session_code, workspace_code, status, revision, last_input_code, updated_at)
                VALUES
                  (1, 'GOAL-1', '{}', 'TRON_CORE', 'PAUSED', 0, NULL, 1);
                INSERT INTO agent_session_goal_events
                (id, event_code, goal_code, session_code, workspace_code, event_kind, created_at)
                VALUES
                  (1, 'GOALEVENT-1', 'GOAL-1', '{}', 'TRON_CORE', 'GOAL_SET', 1),
                  (2, 'GOALEVENT-2', 'GOAL-1', '{}', 'TRON_CORE', 'JUDGE_VERDICT', 2);",
                session.session_code, session.session_code, session.session_code
            ))
            .expect("legacy goal events should be creatable");
    }

    SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("legacy goal event_seq should be backfilled and repaired");
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let sequences = {
        let mut statement = connection
            .prepare(
                "SELECT event_seq
                 FROM agent_session_goal_events
                 WHERE goal_code = 'GOAL-1'
                 ORDER BY id ASC",
            )
            .expect("goal event seq query should prepare");
        statement
            .query_map([], |row| row.get::<_, i64>(0))
            .expect("goal event seq query should run")
            .collect::<Result<Vec<_>, _>>()
            .expect("goal event seq rows should map")
    };
    assert_eq!(sequences, vec![1, 2]);
    assert!(index_exists(
        &connection,
        "idx_agent_session_goal_events_goal_seq"
    ));
}

#[test]
fn schema_guard_repairs_legacy_transcript_ownership_and_drops_unowned_rows() {
    let database_url = temp_database_url("short-term-memory-legacy-owner-repair");
    let session = create_test_session(&database_url, "legacy owner repair");
    {
        let connection =
            rusqlite::Connection::open(&database_url).expect("legacy database should open");
        connection
            .execute_batch(&format!(
                "CREATE TABLE agent_transcript_messages (
                    id INTEGER PRIMARY KEY,
                    message_code TEXT NOT NULL UNIQUE,
                    session_code TEXT NOT NULL,
                    seq INTEGER NOT NULL,
                    role TEXT NOT NULL,
                    message_type TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE TABLE agent_transcript_parts (
                    id INTEGER PRIMARY KEY,
                    part_code TEXT NOT NULL UNIQUE,
                    message_code TEXT NOT NULL,
                    seq INTEGER NOT NULL,
                    part_type TEXT NOT NULL,
                    status TEXT NOT NULL,
                    content_text TEXT,
                    content_json TEXT,
                    created_at INTEGER NOT NULL
                );
                INSERT INTO agent_transcript_messages
                (id, message_code, session_code, seq, role, message_type, status, created_at)
                VALUES
                  (1, 'ATM-LEGACY', '{}', 1, 'user', 'normal', 'completed', 1);
                INSERT INTO agent_transcript_parts
                (id, part_code, message_code, seq, part_type, status, content_text, content_json, created_at)
                VALUES
                  (1, 'ATP-OWNED', 'ATM-LEGACY', 1, 'text', 'completed', 'owned text', NULL, 1),
                  (2, 'ATP-ORPHAN', 'ATM-MISSING', 1, 'text', 'completed', 'orphan text', NULL, 1);",
                session.session_code
            ))
            .expect("legacy transcript rows should be creatable");
    }

    SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("legacy owner fields should be repaired");
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let (part_session, part_workspace): (String, String) = connection
        .query_row(
            "SELECT session_code, workspace_code
             FROM agent_transcript_parts
             WHERE part_code = 'ATP-OWNED'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("owned legacy part should remain");
    assert_eq!(part_session, session.session_code);
    assert_eq!(part_workspace, "TRON_CORE");
    let orphan_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_parts
             WHERE part_code = 'ATP-ORPHAN'
                OR session_code = ''
                OR workspace_code = ''",
            [],
            |row| row.get(0),
        )
        .expect("orphan count should be queryable");
    assert_eq!(orphan_count, 0);
}

#[test]
fn schema_initialization_rolls_back_partial_backfills_when_validation_fails() {
    let database_url = temp_database_url("short-term-memory-init-rollback");
    {
        let connection =
            rusqlite::Connection::open(&database_url).expect("legacy database should open");
        connection
            .execute_batch(
                "CREATE TABLE agent_transcript_context_epochs (
                    id INTEGER PRIMARY KEY,
                    epoch_code TEXT NOT NULL UNIQUE,
                    session_code TEXT NOT NULL,
                    workspace_code TEXT NOT NULL,
                    baseline_seq INTEGER NOT NULL,
                    replacement_seq TEXT NOT NULL,
                    snapshot_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );",
            )
            .expect("drifted legacy table should be creatable");
    }

    let error = match SqliteShortTermMemoryRepository::connect(&database_url) {
        Ok(_) => panic!("drifted schema should fail validation"),
        Err(error) => error,
    };
    assert_eq!(error, ShortTermMemoryError::StorageFailure);
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let columns = table_columns(&connection, "agent_transcript_context_epochs");
    assert!(
        !columns.contains(&"revision".to_string()),
        "failed initialization should not leave backfilled columns behind"
    );
    assert!(
        !table_exists(&connection, "agent_transcript_inputs"),
        "failed initialization should roll back STM migration tables"
    );
}

#[test]
fn schema_guard_validates_required_indexes_and_tables() {
    let database_url = temp_database_url("short-term-memory-schema-required");
    SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("fresh short-term memory schema should validate");
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    for table in [
        "agent_transcript_inputs",
        "agent_transcript_events",
        "agent_transcript_messages",
        "agent_transcript_parts",
        "agent_transcript_context_epochs",
        "agent_session_todos",
        "agent_session_families",
        "agent_session_family_members",
        "agent_session_family_snapshots",
        "agent_session_goals",
        "agent_session_goal_criteria",
        "agent_session_goal_events",
    ] {
        assert!(table_exists(&connection, table), "{table} should exist");
    }
    for index in [
        "idx_agent_transcript_inputs_session_idempotency",
        "idx_agent_transcript_events_session_seq",
        "idx_agent_transcript_events_session_seq_unique",
        "idx_agent_transcript_messages_session_seq",
        "idx_agent_transcript_messages_session_seq_unique",
        "idx_agent_transcript_parts_message_seq",
        "idx_agent_transcript_parts_message_seq_unique",
        "idx_agent_transcript_context_epochs_session_revision",
        "idx_agent_transcript_context_epochs_session_revision_unique",
        "idx_agent_session_todos_session_position",
        "idx_agent_session_family_members_session_enabled",
        "idx_agent_session_family_snapshots_family_status",
        "idx_agent_session_family_snapshots_session_latest",
        "idx_agent_session_family_snapshots_session_run_summary",
        "idx_agent_session_goals_one_open",
        "idx_agent_session_goal_criteria_goal_position",
        "idx_agent_session_goal_events_goal_seq",
        "idx_agent_session_todos_session_todo_code",
        "idx_agent_session_run_leases_session",
        "idx_agent_session_run_leases_run",
        "idx_agent_session_family_snapshots_session_run_summary",
    ] {
        assert!(index_exists(&connection, index), "{index} should exist");
    }
    let context_epoch_index_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_agent_transcript_context_epochs_session_revision_unique'",
            [],
            |row| row.get(0),
        )
        .expect("context epoch unique index should be queryable");
    assert!(context_epoch_index_sql.starts_with("CREATE UNIQUE INDEX"));
    for index in [
        "idx_agent_transcript_events_session_seq_unique",
        "idx_agent_transcript_messages_session_seq_unique",
        "idx_agent_transcript_parts_message_seq_unique",
        "idx_agent_session_todos_session_position",
        "idx_agent_session_todos_session_todo_code",
        "idx_agent_session_goal_criteria_goal_position",
        "idx_agent_session_goal_events_goal_seq",
        "idx_agent_session_run_leases_session",
        "idx_agent_session_run_leases_run",
    ] {
        let index_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [index],
                |row| row.get(0),
            )
            .expect("transcript unique index should be queryable");
        assert!(
            index_sql.starts_with("CREATE UNIQUE INDEX"),
            "{index} should be unique"
        );
    }
    let input_idempotency_index_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_agent_transcript_inputs_session_idempotency'",
            [],
            |row| row.get(0),
        )
        .expect("input idempotency index should be queryable");
    assert!(input_idempotency_index_sql.contains("CREATE UNIQUE INDEX"));
    assert!(input_idempotency_index_sql.contains("idempotency_key IS NOT NULL"));
    let session_summary_index_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_agent_session_family_snapshots_session_run_summary'",
            [],
            |row| row.get(0),
        )
        .expect("session summary unique index should be queryable");
    assert!(session_summary_index_sql.contains("CREATE UNIQUE INDEX"));
    assert!(session_summary_index_sql.contains("snapshot_kind = 'SESSION_SUMMARY'"));
    assert!(session_summary_index_sql.contains("run_code IS NOT NULL"));
    let family_code_not_null: i64 = connection
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('agent_session_family_snapshots') WHERE name = 'family_code'",
            [],
            |row| row.get(0),
        )
        .expect("snapshot family ownership column should be queryable");
    assert_eq!(family_code_not_null, 0);
}

#[test]
fn schema_migration_makes_snapshot_family_optional_and_preserves_legacy_handoffs() {
    let database_url = temp_database_url("short-term-memory-session-owned-snapshot-migration");
    let session = create_test_session(&database_url, "legacy family snapshot migration");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("fresh short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut legacy_context = base_context(&session.session_code);
    legacy_context.execution_scope.task_code = Some("TASK-LEGACY-SNAPSHOT".into());
    let input_code = service
        .record_user_input(&legacy_context, "legacy handoff input")
        .expect("legacy input should be recorded");
    attach_short_term_memory(
        &mut legacy_context,
        &session.session_code,
        input_code,
        vec![AgentMessage::system("system seed")],
    );
    service
        .record_run_started(&legacy_context)
        .expect("legacy family membership should be created");
    service
        .record_run_completed(&legacy_context)
        .expect("legacy run should complete");
    drop(service);

    {
        let connection =
            rusqlite::Connection::open(&database_url).expect("legacy database should open");
        let family_code: String = connection
            .query_row(
                "SELECT family_code
                 FROM agent_session_family_members
                 WHERE session_code = ?1
                 ORDER BY id ASC
                 LIMIT 1",
                [session.session_code.as_str()],
                |row| row.get(0),
            )
            .expect("legacy family should be queryable");
        connection
            .execute(
                "INSERT INTO agent_session_family_snapshots (
                    id, snapshot_code, family_code, session_code, run_code,
                    snapshot_kind, status, source_start_seq, source_end_seq,
                    source_checkpoint_message_code, summary_markdown, token_estimate,
                    error_code, error_message, metadata_json, created_at,
                    completed_at, expires_at
                 ) VALUES (
                    910200000000000001, 'AFS-LEGACY-HANDOFF', ?1, ?2, ?3,
                    'RUN_HANDOFF', 'COMPLETED', 1, 2,
                    NULL, '# Legacy Handoff', 4,
                    NULL, NULL, '{}', 1, 1, NULL
                 )",
                [
                    family_code.as_str(),
                    session.session_code.as_str(),
                    legacy_context.run_code.as_str(),
                ],
            )
            .expect("legacy handoff should be insertable");
        let canonical_table_sql: String = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'agent_session_family_snapshots'",
                [],
                |row| row.get(0),
            )
            .expect("snapshot table definition should be queryable");
        let legacy_table_sql =
            canonical_table_sql.replacen("family_code TEXT,", "family_code TEXT NOT NULL,", 1);
        assert_ne!(legacy_table_sql, canonical_table_sql);
        connection
            .execute_batch(
                "DROP INDEX IF EXISTS idx_agent_session_family_snapshots_family_status;
                 DROP INDEX IF EXISTS idx_agent_session_family_snapshots_session_latest;
                 DROP INDEX IF EXISTS idx_agent_session_family_snapshots_session_run_summary;
                 ALTER TABLE agent_session_family_snapshots
                     RENAME TO agent_session_family_snapshots_nullable_backup;",
            )
            .expect("canonical table should be movable for legacy fixture setup");
        connection
            .execute_batch(&legacy_table_sql)
            .expect("legacy family-owned snapshot table should be creatable");
        connection
            .execute_batch(
                "INSERT INTO agent_session_family_snapshots
                 SELECT * FROM agent_session_family_snapshots_nullable_backup;
                 DROP TABLE agent_session_family_snapshots_nullable_backup;",
            )
            .expect("legacy snapshot rows should be copied");
    }

    let migrated_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("legacy family-owned snapshot schema should migrate"),
        ShortTermMemoryConfig::default(),
    );
    let connection =
        rusqlite::Connection::open(&database_url).expect("migrated database should open");
    let family_code_not_null: i64 = connection
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('agent_session_family_snapshots') WHERE name = 'family_code'",
            [],
            |row| row.get(0),
        )
        .expect("migrated family ownership column should be queryable");
    assert_eq!(family_code_not_null, 0);
    let legacy_summary: String = connection
        .query_row(
            "SELECT summary_markdown
             FROM agent_session_family_snapshots
             WHERE snapshot_code = 'AFS-LEGACY-HANDOFF'",
            [],
            |row| row.get(0),
        )
        .expect("legacy handoff should survive migration");
    assert_eq!(legacy_summary, "# Legacy Handoff");

    let summary_context = record_summary_run(
        &migrated_service,
        &session.session_code,
        "RUN-SESSION-SUMMARY-AFTER-MIGRATION",
        "new session-owned summary input",
        "new session-owned summary output",
    );
    migrated_service
        .prepare_session_summary_job(
            session_summary_context(&summary_context),
            completed_summary_outcome(),
        )
        .expect("session-owned summary should insert after migration")
        .expect("new summary job should be created");
    let new_family_code: Option<String> = connection
        .query_row(
            "SELECT family_code
             FROM agent_session_family_snapshots
             WHERE session_code = ?1
               AND run_code = ?2
               AND snapshot_kind = 'SESSION_SUMMARY'",
            [
                session.session_code.as_str(),
                summary_context.run_code.as_str(),
            ],
            |row| row.get(0),
        )
        .expect("session-owned summary should be queryable");
    assert!(new_family_code.is_none());
}

#[test]
fn schema_guard_rejects_drifted_required_index_definitions() {
    let database_url = temp_database_url("short-term-memory-schema-drifted-index");
    SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("fresh short-term memory schema should validate");
    {
        let connection =
            rusqlite::Connection::open(&database_url).expect("database should be queryable");
        connection
            .execute_batch(
                "DROP INDEX idx_agent_session_goals_one_open;
                 CREATE INDEX idx_agent_session_goals_one_open
                 ON agent_session_goals (session_code);",
            )
            .expect("test should be able to create drifted same-name index");
    }

    let error = match SqliteShortTermMemoryRepository::connect(&database_url) {
        Ok(_) => panic!("drifted goal open index should fail schema validation"),
        Err(error) => error,
    };

    assert_eq!(error, ShortTermMemoryError::StorageFailure);
}

#[test]
fn schema_guard_rejects_orphan_json_and_invalid_enum_rows() {
    for (label, mutation) in [
        (
            "orphan-part",
            "INSERT INTO agent_transcript_parts
             (id, part_code, session_code, workspace_code, message_code, seq, part_type, status, content_text, content_json, visibility, created_at, completed_at)
             VALUES
             (910100000000000001, 'ATP-ORPHAN', 'CHAT-1', 'TRON_CORE', 'ATM-MISSING', 1, 'text', 'completed', 'orphan', NULL, 'model', 1, 1);",
        ),
        (
            "bad-json",
            "INSERT INTO agent_transcript_messages
             (id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, created_at, completed_at)
             VALUES
             (910100000000000002, 'ATM-BAD-JSON', 'CHAT-1', 'TRON_CORE', NULL, 100, 'assistant', 'normal', 'completed', 1, 1);
             INSERT INTO agent_transcript_parts
             (id, part_code, session_code, workspace_code, message_code, seq, part_type, status, content_text, content_json, visibility, created_at, completed_at)
             VALUES
             (910100000000000003, 'ATP-BAD-JSON', 'CHAT-1', 'TRON_CORE', 'ATM-BAD-JSON', 1, 'context', 'completed', 'bad json', '{not-json', 'model', 1, 1);",
        ),
        (
            "invalid-enum",
            "INSERT INTO agent_transcript_messages
             (id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, created_at, completed_at)
             VALUES
             (910100000000000004, 'ATM-BAD-ENUM', 'CHAT-1', 'TRON_CORE', NULL, 100, 'alien', 'normal', 'completed', 1, 1);",
        ),
    ] {
        let database_url = temp_database_url(&format!("short-term-memory-schema-guard-{label}"));
        let session = create_test_session(&database_url, "schema guard invalid row");
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("fresh schema should initialize before corruption");
        {
            let connection =
                rusqlite::Connection::open(&database_url).expect("database should be queryable");
            connection
                .execute_batch(&mutation.replace("CHAT-1", &session.session_code))
                .expect("test corruption should be insertable");
        }

        let error = match SqliteShortTermMemoryRepository::connect(&database_url) {
            Ok(_) => panic!("invalid {label} row should fail schema validation"),
            Err(error) => error,
        };

        assert_eq!(error, ShortTermMemoryError::StorageFailure);
    }
}

#[test]
fn schema_guard_rejects_legacy_duplicate_transcript_sequences() {
    for (label, ddl) in [
        (
            "events",
            "CREATE TABLE agent_transcript_events (
                id INTEGER PRIMARY KEY,
                event_code TEXT NOT NULL UNIQUE,
                session_code TEXT NOT NULL,
                seq INTEGER NOT NULL,
                input_code TEXT,
                event_type TEXT NOT NULL,
                event_source TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                occurred_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL
             );
             INSERT INTO agent_transcript_events (id, event_code, session_code, seq, input_code, event_type, event_source, payload_json, occurred_at, created_at)
             VALUES
               (1, 'ATE-1', 'CHAT-1', 7, NULL, 'agent.run.started', 'RUNTIME', '{}', 1, 1),
               (2, 'ATE-2', 'CHAT-1', 7, NULL, 'agent.run.completed', 'RUNTIME', '{}', 2, 2);",
        ),
        (
            "messages",
            "CREATE TABLE agent_transcript_messages (
                id INTEGER PRIMARY KEY,
                message_code TEXT NOT NULL UNIQUE,
                session_code TEXT NOT NULL,
                seq INTEGER NOT NULL,
                role TEXT NOT NULL,
                message_type TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL
             );
             INSERT INTO agent_transcript_messages (id, message_code, session_code, seq, role, message_type, status, created_at)
             VALUES
               (1, 'ATM-1', 'CHAT-1', 9, 'assistant', 'normal', 'completed', 1),
               (2, 'ATM-2', 'CHAT-1', 9, 'assistant', 'normal', 'completed', 2);",
        ),
        (
            "parts",
            "CREATE TABLE agent_transcript_messages (
                id INTEGER PRIMARY KEY,
                message_code TEXT NOT NULL UNIQUE,
                session_code TEXT NOT NULL,
                seq INTEGER NOT NULL,
                role TEXT NOT NULL,
                message_type TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL
             );
             INSERT INTO agent_transcript_messages (id, message_code, session_code, seq, role, message_type, status, created_at)
             VALUES (10, 'ATM-1', 'CHAT-1', 8, 'assistant', 'normal', 'completed', 1);
             CREATE TABLE agent_transcript_parts (
                id INTEGER PRIMARY KEY,
                part_code TEXT NOT NULL UNIQUE,
                message_code TEXT NOT NULL,
                seq INTEGER NOT NULL,
                part_type TEXT NOT NULL,
                status TEXT NOT NULL,
                content_text TEXT,
                content_json TEXT,
                created_at INTEGER NOT NULL
             );
             INSERT INTO agent_transcript_parts (id, part_code, message_code, seq, part_type, status, content_text, content_json, created_at)
             VALUES
               (1, 'ATP-1', 'ATM-1', 1, 'text', 'completed', 'first', NULL, 1),
               (2, 'ATP-2', 'ATM-1', 1, 'text', 'completed', 'second', NULL, 2);",
        ),
    ] {
        let database_url = temp_database_url(&format!(
            "short-term-memory-legacy-duplicate-{label}"
        ));
        let session = create_test_session(&database_url, "legacy duplicate transcript");
        let ddl = ddl.replace("CHAT-1", &session.session_code);
        {
            let connection =
                rusqlite::Connection::open(&database_url).expect("legacy database should open");
            connection
                .execute_batch(&ddl)
                .expect("legacy duplicate table should be creatable");
        }

        let error = match SqliteShortTermMemoryRepository::connect(&database_url) {
            Ok(_) => panic!("legacy duplicate {label} sequences should fail schema validation"),
            Err(error) => error,
        };

        assert_eq!(error, ShortTermMemoryError::StorageFailure);
    }
}

#[test]
fn schema_guard_rejects_legacy_duplicate_working_state_uniques() {
    for (label, ddl) in [
        (
            "todos-position",
            "CREATE TABLE agent_session_todos (
                id INTEGER PRIMARY KEY,
                todo_code TEXT NOT NULL,
                session_code TEXT NOT NULL,
                workspace_code TEXT NOT NULL,
                position INTEGER NOT NULL,
                content TEXT NOT NULL,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
             );
             INSERT INTO agent_session_todos
             (id, todo_code, session_code, workspace_code, position, content, status, priority, created_at, updated_at)
             VALUES
               (1, 'TODO-1', 'CHAT-1', 'TRON_CORE', 0, 'first', 'pending', 'medium', 1, 1),
               (2, 'TODO-2', 'CHAT-1', 'TRON_CORE', 0, 'second', 'pending', 'medium', 1, 1);",
        ),
        (
            "goal-criteria-position",
            "CREATE TABLE agent_session_goal_criteria (
                id INTEGER PRIMARY KEY,
                criterion_code TEXT NOT NULL UNIQUE,
                goal_code TEXT NOT NULL,
                session_code TEXT NOT NULL,
                position INTEGER NOT NULL,
                text TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
             );
             INSERT INTO agent_session_goal_criteria
             (id, criterion_code, goal_code, session_code, position, text, status, created_at, updated_at)
             VALUES
               (1, 'CRIT-1', 'GOAL-1', 'CHAT-1', 0, 'first', 'pending', 1, 1),
               (2, 'CRIT-2', 'GOAL-1', 'CHAT-1', 0, 'second', 'pending', 1, 1);",
        ),
        (
            "goal-events-seq",
            "CREATE TABLE agent_session_goal_events (
                id INTEGER PRIMARY KEY,
                event_code TEXT NOT NULL UNIQUE,
                goal_code TEXT NOT NULL,
                session_code TEXT NOT NULL,
                workspace_code TEXT NOT NULL,
                event_kind TEXT NOT NULL,
                event_seq INTEGER NOT NULL,
                created_at INTEGER NOT NULL
             );
             INSERT INTO agent_session_goal_events
             (id, event_code, goal_code, session_code, workspace_code, event_kind, event_seq, created_at)
             VALUES
               (1, 'GE-1', 'GOAL-1', 'CHAT-1', 'TRON_CORE', 'CREATED', 1, 1),
               (2, 'GE-2', 'GOAL-1', 'CHAT-1', 'TRON_CORE', 'UPDATED', 1, 1);",
        ),
    ] {
        let database_url = temp_database_url(&format!(
            "short-term-memory-legacy-working-state-duplicate-{label}"
        ));
        let session = create_test_session(&database_url, "legacy duplicate working state");
        let ddl = ddl.replace("CHAT-1", &session.session_code);
        {
            let connection =
                rusqlite::Connection::open(&database_url).expect("legacy database should open");
            connection
                .execute_batch(&ddl)
                .expect("legacy duplicate working-state table should be creatable");
        }

        let error = match SqliteShortTermMemoryRepository::connect(&database_url) {
            Ok(_) => panic!("legacy duplicate {label} rows should fail schema validation"),
            Err(error) => error,
        };

        assert_eq!(error, ShortTermMemoryError::StorageFailure);
    }
}

#[test]
fn schema_backfill_reassigns_context_epoch_revisions_before_unique_index() {
    let database_url = temp_database_url("short-term-memory-legacy-context-revisions");
    let session = create_test_session(&database_url, "legacy context revisions");
    {
        let connection =
            rusqlite::Connection::open(&database_url).expect("legacy database should open");
        connection
            .execute_batch(&format!(
                "CREATE TABLE agent_transcript_context_epochs (
                    id INTEGER PRIMARY KEY,
                    epoch_code TEXT NOT NULL UNIQUE,
                    session_code TEXT NOT NULL,
                    workspace_code TEXT NOT NULL,
                    baseline_seq INTEGER NOT NULL,
                    snapshot_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                INSERT INTO agent_transcript_context_epochs
                (id, epoch_code, session_code, workspace_code, baseline_seq, snapshot_json, created_at)
                VALUES
                  (1, 'ATCE-OLD-1', '{}', 'TRON_CORE', 1, '{{}}', 10),
                  (2, 'ATCE-OLD-2', '{}', 'TRON_CORE', 2, '{{}}', 20);",
                session.session_code, session.session_code
            ))
            .expect("legacy context epoch rows should be creatable");
    }

    SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("legacy context epoch revisions should be repaired before validation");

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let revisions = connection
        .prepare(
            "SELECT revision
             FROM agent_transcript_context_epochs
             WHERE session_code = ?1
             ORDER BY created_at ASC, id ASC",
        )
        .expect("context epoch revisions should be selectable")
        .query_map([session.session_code.as_str()], |row| row.get::<_, i64>(0))
        .expect("context epoch revisions should be queryable")
        .collect::<Result<Vec<_>, _>>()
        .expect("context epoch revisions should be readable");
    assert_eq!(revisions, vec![1, 2]);
    let unique_index_sql: String = connection
        .query_row(
            "SELECT sql
             FROM sqlite_master
             WHERE type = 'index'
               AND name = 'idx_agent_transcript_context_epochs_session_revision_unique'",
            [],
            |row| row.get(0),
        )
        .expect("context epoch unique index should exist");
    assert!(unique_index_sql.starts_with("CREATE UNIQUE INDEX"));
}

#[test]
fn transcript_input_idempotency_reuses_existing_input() {
    let database_url = temp_database_url("short-term-memory-input-idempotency");
    let session = create_test_session(&database_url, "input idempotency");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);

    let first = service
        .record_user_input(&context, "same user input")
        .expect("first user input should be recorded");
    let second = service
        .record_user_input(&context, "same user input")
        .expect("duplicate user input should reuse the first row");
    assert_eq!(first, second);

    context.run_code = "RUN-STMEM-CONTINUATION-A".into();
    let continuation_a = service
        .record_invocation_input(
            &context,
            "Continue the active goal.",
            "CONTINUATION",
            Some(json!({"source": "GOAL_LOOP", "goalCode": "GOAL-1"})),
        )
        .expect("first continuation should be recorded");
    let continuation_a_retry = service
        .record_invocation_input(
            &context,
            "Continue the active goal.",
            "CONTINUATION",
            Some(json!({"source": "GOAL_LOOP", "goalCode": "GOAL-1"})),
        )
        .expect("same continuation run should reuse the first row");
    assert_eq!(continuation_a, continuation_a_retry);

    context.run_code = "RUN-STMEM-CONTINUATION-B".into();
    context.request_id = Some("REQ-STMEM-CONTINUATION-B".into());
    let continuation_b = service
        .record_invocation_input(
            &context,
            "Continue the active goal.",
            "CONTINUATION",
            Some(json!({"source": "GOAL_LOOP", "goalCode": "GOAL-1"})),
        )
        .expect("new continuation run should be recorded");
    assert_ne!(continuation_a, continuation_b);

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let user_created_events: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE input_code = ?1
               AND event_type = 'agent.message.user_created'",
            [first.as_str()],
            |row| row.get(0),
        )
        .expect("user created event count should be queryable");
    assert_eq!(user_created_events, 1);
    let (input_count, user_message_count, continuation_count): (i64, i64, i64) = connection
        .query_row(
            "SELECT
                (SELECT count(1) FROM agent_transcript_inputs WHERE session_code = ?1),
                (SELECT count(1) FROM agent_transcript_messages WHERE session_code = ?1 AND role = 'user'),
                (SELECT count(1) FROM agent_transcript_inputs WHERE session_code = ?1 AND input_kind = 'CONTINUATION')",
            [session.session_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("idempotent input counts should be queryable");
    assert_eq!(input_count, 3);
    assert_eq!(user_message_count, 1);
    assert_eq!(continuation_count, 2);
}

#[test]
fn repository_rejects_todo_workspace_mismatch_inside_transaction() {
    let database_url = temp_database_url("short-term-memory-todo-workspace-mismatch");
    let session = create_test_session(&database_url, "todo workspace mismatch");
    let mut repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let now = current_timestamp();
    let todo = ShortTermMemoryTodo {
        id: 0,
        todo_code: "TODO-WORKSPACE-MISMATCH".into(),
        session_code: session.session_code.clone(),
        workspace_code: "TRON_CORE".into(),
        position: 0,
        content: "must not write under the wrong workspace".into(),
        status: TodoStatus::Pending,
        priority: TodoPriority::Medium,
        created_by_run_code: None,
        updated_by_run_code: None,
        created_at: now.clone(),
        updated_at: now,
    };

    let error = repository
        .replace_todos(&session.session_code, "OTHER_WORKSPACE", &[todo])
        .expect_err("workspace mismatch should fail inside repository transaction");

    assert_eq!(error, ShortTermMemoryError::StorageFailure);
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let todo_count: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_session_todos WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("todo count should be queryable");
    assert_eq!(todo_count, 0);
}

#[test]
fn concurrent_input_writes_allocate_unique_ids_inside_repository_transactions() {
    let database_url = temp_database_url("short-term-memory-input-id-concurrent");
    let first_session = create_test_session(&database_url, "input id first");
    let second_session = create_test_session(&database_url, "input id second");
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let handles = [
        (first_session.session_code.clone(), 100_000_i64),
        (second_session.session_code.clone(), 200_000_i64),
    ]
    .into_iter()
    .map(|(session_code, id_seed)| {
        let database_url = database_url.clone();
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            let service = ShortTermMemoryService::with_id_provider(
                SqliteShortTermMemoryRepository::connect(&database_url)
                    .expect("short-term memory repository should connect"),
                ShortTermMemoryConfig::default(),
                Arc::new(FixedSnowflakeIdProvider::new(id_seed)),
            );
            let mut context = base_context(&session_code);
            context.run_code = format!("RUN-INPUT-{id_seed}");
            context.request_id = Some(format!("REQ-INPUT-{id_seed}"));
            barrier.wait();
            service
                .record_user_input(&context, &format!("concurrent input {id_seed}"))
                .expect("input should be recorded")
        })
    })
    .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("input writer should finish");
    }

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let (input_count, distinct_ids): (i64, i64) = connection
        .query_row(
            "SELECT count(1), count(DISTINCT id)
             FROM agent_transcript_inputs
             WHERE session_code IN (?1, ?2)",
            [
                first_session.session_code.as_str(),
                second_session.session_code.as_str(),
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("input ids should be queryable");
    assert_eq!(input_count, 2);
    assert_eq!(distinct_ids, 2);
}

#[test]
fn message_with_event_transaction_rolls_back_on_event_insert_failure() {
    let database_url = temp_database_url("short-term-memory-message-event-rollback");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("rollback".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "rollback input")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    connection
        .execute_batch(
            "CREATE TRIGGER fail_assistant_completed_event
             BEFORE INSERT ON agent_transcript_events
             WHEN NEW.event_type = 'agent.message.assistant_completed'
             BEGIN
               SELECT RAISE(ABORT, 'assistant event blocked');
             END;",
        )
        .expect("event failure trigger should be installable");

    let result = service.record_model_response(
        &context,
        &ModelResponse::final_output("assistant row must roll back"),
    );

    assert!(matches!(result, Err(ShortTermMemoryError::StorageFailure)));
    let assistant_messages: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND role = 'assistant'
               AND message_type = 'normal'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("assistant message count should be queryable");
    assert_eq!(assistant_messages, 0);
    let responded_events: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.model.responded'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("model responded event count should be queryable");
    assert_eq!(responded_events, 0);
}

#[test]
fn context_epoch_revision_is_unique_under_concurrent_prepare() {
    let database_url = temp_database_url("short-term-memory-context-epoch-concurrent");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("context epoch concurrent".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    context.tools.clear();
    let input_code = service
        .record_user_input(&context, "prepare concurrently")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let handles = (0..6)
        .map(|index| {
            let service = service.clone();
            let mut context = context.clone();
            context.run_code = format!("RUN-CONTEXT-EPOCH-{index}");
            std::thread::spawn(move || {
                service
                    .prepare_model_context(&context)
                    .expect("context prepare should succeed");
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("prepare thread should finish");
    }

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let (row_count, distinct_revisions): (i64, i64) = connection
        .query_row(
            "SELECT count(1), count(DISTINCT revision)
             FROM agent_transcript_context_epochs
             WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("context epoch revisions should be queryable");
    assert_eq!(row_count, 6);
    assert_eq!(distinct_revisions, 6);
}

#[test]
fn goal_storage_rejects_second_open_goal_and_preserves_existing_goal() {
    let database_url = temp_database_url("short-term-memory-goal-conflict");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("goal conflict".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );

    service
        .set_goal(SetGoalCommand {
            session_code: session.session_code.clone(),
            objective: "Keep the original goal".into(),
            max_runs: Some(3),
        })
        .expect("first goal should be created");
    let conflict = service
        .set_goal(SetGoalCommand {
            session_code: session.session_code.clone(),
            objective: "Reject this second goal".into(),
            max_runs: Some(3),
        })
        .expect_err("second open goal should conflict");

    assert_eq!(conflict, ShortTermMemoryError::GoalConflict);
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let (goal_count, objective): (i64, String) = connection
        .query_row(
            "SELECT count(1), max(objective_markdown)
             FROM agent_session_goals
             WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("goal rows should be queryable");
    assert_eq!(goal_count, 1);
    assert_eq!(objective, "Keep the original goal");
}

#[test]
fn goal_update_rejects_stale_revision_and_preserves_current_state() {
    let database_url = temp_database_url("short-term-memory-goal-revision-conflict");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("goal revision conflict".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let created = service
        .set_goal(SetGoalCommand {
            session_code: session.session_code.clone(),
            objective: "Keep revision safe".into(),
            max_runs: Some(3),
        })
        .expect("goal should be created");
    let mut repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should reconnect");
    let stale = repository
        .get_active_goal(&session.session_code)
        .expect("active goal should load")
        .expect("active goal should exist");
    let expected_revision = stale.goal.revision;

    let mut first = stale.clone();
    first.goal.status = "PAUSED".into();
    first.goal.revision = expected_revision + 1;
    first.goal.updated_at = current_timestamp();
    let first_event = test_goal_event(&first.goal, "GOAL_PAUSED");
    repository
        .update_goal(
            &first.goal,
            &first.criteria,
            &first_event,
            expected_revision,
        )
        .expect("first update should win");

    let mut second = stale;
    second.goal.status = "BLOCKED".into();
    second.goal.revision = expected_revision + 1;
    second.goal.updated_at = current_timestamp();
    let second_event = test_goal_event(&second.goal, "GOAL_BLOCKED");
    let error = repository
        .update_goal(
            &second.goal,
            &second.criteria,
            &second_event,
            expected_revision,
        )
        .expect_err("stale update must conflict");

    assert_eq!(error, ShortTermMemoryError::GoalStateConflict);
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let (status, revision): (String, i64) = connection
        .query_row(
            "SELECT status, revision FROM agent_session_goals WHERE goal_code = ?1",
            [created.goal.goal_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("goal row should be queryable");
    assert_eq!(status, "PAUSED");
    assert_eq!(revision, expected_revision + 1);
    let blocked_events: i64 = connection
        .query_row(
            "SELECT count(1) FROM agent_session_goal_events WHERE goal_code = ?1 AND event_kind = 'GOAL_BLOCKED'",
            [created.goal.goal_code.as_str()],
            |row| row.get(0),
        )
        .expect("goal event rows should be queryable");
    assert_eq!(blocked_events, 0);
}

fn test_goal_event(
    goal: &tron_backend::domain::short_term_memory::ShortTermMemoryGoal,
    event_kind: &str,
) -> ShortTermMemoryGoalEvent {
    ShortTermMemoryGoalEvent {
        id: 0,
        event_code: String::new(),
        goal_code: goal.goal_code.clone(),
        session_code: goal.session_code.clone(),
        workspace_code: goal.workspace_code.clone(),
        run_code: None,
        input_code: None,
        event_kind: event_kind.into(),
        event_seq: 0,
        summary: Some(event_kind.to_string()),
        payload_json: Some(json!({"source": "test"})),
        created_at: current_timestamp(),
    }
}

#[test]
fn assistant_response_persists_reasoning_summary_and_enforces_raw_reasoning_policy() {
    let default_database_url = temp_database_url("short-term-memory-reasoning-summary");
    let default_session = create_test_session(&default_database_url, "reasoning summary");
    let default_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&default_database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut default_context = base_context(&default_session.session_code);
    let default_input = default_service
        .record_user_input(&default_context, "reasoning default")
        .expect("user input should be recorded");
    default_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: default_session.session_code.clone(),
        input_code: default_input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    default_service
        .record_model_response(&default_context, &reasoning_response())
        .expect("assistant response should persist summary");
    let default_connection =
        rusqlite::Connection::open(&default_database_url).expect("database should be queryable");
    assert_eq!(
        part_type_count(
            &default_connection,
            &default_session.session_code,
            "reasoning_summary",
        ),
        1
    );
    assert_eq!(
        part_type_count(
            &default_connection,
            &default_session.session_code,
            "reasoning"
        ),
        0
    );

    let raw_database_url = temp_database_url("short-term-memory-reasoning-raw");
    let raw_session = create_test_session(&raw_database_url, "reasoning raw");
    let raw_config = ShortTermMemoryConfig::default();

    let raw_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&raw_database_url)
            .expect("short-term memory repository should connect"),
        raw_config,
    );
    let mut raw_context = base_context(&raw_session.session_code);
    let raw_input = raw_service
        .record_user_input(&raw_context, "reasoning raw")
        .expect("user input should be recorded");
    raw_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: raw_session.session_code.clone(),
        input_code: raw_input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    raw_service
        .record_model_response(&raw_context, &reasoning_response())
        .expect("assistant response should persist reasoning summary");
    let raw_connection =
        rusqlite::Connection::open(&raw_database_url).expect("database should be queryable");
    assert_eq!(
        part_type_count(&raw_connection, &raw_session.session_code, "reasoning"),
        0
    );
    let raw_rendered = rendered_model_context(
        &raw_service
            .prepare_model_context(&raw_context)
            .expect("raw context should prepare")
            .messages,
    );
    assert!(!raw_rendered.contains("raw reasoning should be policy gated"));
}

#[test]
fn reasoning_summary_replay_is_disabled_by_default_and_configurable() {
    let default_database_url = temp_database_url("short-term-memory-reasoning-replay-default");
    let default_session = create_test_session(&default_database_url, "reasoning replay default");
    let default_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&default_database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut default_context = base_context(&default_session.session_code);
    record_reasoning_tool_turn(
        &default_service,
        &mut default_context,
        &default_session.session_code,
        "reasoning replay default",
    );

    let default_prepared = default_service
        .prepare_model_context(&default_context)
        .expect("default context should prepare");
    let default_rendered = rendered_model_context(&default_prepared.messages);
    assert!(!default_rendered.contains("summary reasoning"));
    assert!(default_rendered.contains("visible answer before tool"));
    assert!(default_rendered.contains("tool_call:shell.exec"));
    assert!(default_rendered.contains("tool result visible to model"));
    assert!(!default_prepared.messages.iter().any(|message| {
        message
            .parts
            .iter()
            .any(|part| matches!(part, AgentMessagePart::ReasoningSummary { .. }))
    }));

    let enabled_database_url = temp_database_url("short-term-memory-reasoning-replay-enabled");
    let enabled_session = create_test_session(&enabled_database_url, "reasoning replay enabled");
    let mut enabled_config = ShortTermMemoryConfig::default();
    enabled_config.context.reasoning_summary_replay_enabled = true;
    let enabled_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&enabled_database_url)
            .expect("short-term memory repository should connect"),
        enabled_config,
    );
    let mut enabled_context = base_context(&enabled_session.session_code);
    record_reasoning_tool_turn(
        &enabled_service,
        &mut enabled_context,
        &enabled_session.session_code,
        "reasoning replay enabled",
    );

    let enabled_rendered = rendered_model_context(
        &enabled_service
            .prepare_model_context(&enabled_context)
            .expect("enabled context should prepare")
            .messages,
    );
    assert!(enabled_rendered.contains("summary reasoning"));
}

#[test]
fn compaction_input_filters_reasoning_summary_unless_replay_is_enabled() {
    let default_database_url = temp_database_url("short-term-memory-reasoning-compaction-default");
    let default_session =
        create_test_session(&default_database_url, "reasoning compaction default");
    let mut default_config = ShortTermMemoryConfig::default();
    default_config.compaction.retained_history_ratio_bps = 1;
    let default_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&default_database_url)
            .expect("short-term memory repository should connect"),
        default_config,
    );
    let mut default_context = base_context(&default_session.session_code);
    default_context.run_code = "RUN-STMEM-REASONING-DEFAULT-OLD".into();
    default_service
        .record_user_input(&default_context, "older reasoning compaction head")
        .expect("older default input should be recorded");
    default_context.run_code = "RUN-STMEM-REASONING-DEFAULT-CURRENT".into();
    record_reasoning_tool_turn(
        &default_service,
        &mut default_context,
        &default_session.session_code,
        "reasoning compaction default",
    );
    let default_input = default_service
        .prepare_automatic_model_compaction_input(
            &default_session.session_code,
            "test_reasoning_filter",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            default_context.model.context_window_tokens,
            default_context.model.max_tokens,
            None,
            None,
            None,
        )
        .expect("default compaction input should prepare");
    assert!(!default_input.prompt.contains("summary reasoning"));
    assert!(!default_input.recent.contains("summary reasoning"));
    assert!(default_input.prompt.contains("visible answer before tool"));
    assert!(
        default_input
            .prompt
            .contains("tool result visible to model")
    );

    let enabled_database_url = temp_database_url("short-term-memory-reasoning-compaction-enabled");
    let enabled_session =
        create_test_session(&enabled_database_url, "reasoning compaction enabled");
    let mut enabled_config = ShortTermMemoryConfig::default();
    enabled_config.context.reasoning_summary_replay_enabled = true;
    enabled_config.compaction.retained_history_ratio_bps = 1;
    let enabled_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&enabled_database_url)
            .expect("short-term memory repository should connect"),
        enabled_config,
    );
    let mut enabled_context = base_context(&enabled_session.session_code);
    enabled_context.run_code = "RUN-STMEM-REASONING-ENABLED-OLD".into();
    enabled_service
        .record_user_input(&enabled_context, "older reasoning compaction head")
        .expect("older enabled input should be recorded");
    enabled_context.run_code = "RUN-STMEM-REASONING-ENABLED-CURRENT".into();
    record_reasoning_tool_turn(
        &enabled_service,
        &mut enabled_context,
        &enabled_session.session_code,
        "reasoning compaction enabled",
    );
    let enabled_input = enabled_service
        .prepare_automatic_model_compaction_input(
            &enabled_session.session_code,
            "test_reasoning_enabled",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            enabled_context.model.context_window_tokens,
            enabled_context.model.max_tokens,
            None,
            None,
            None,
        )
        .expect("enabled compaction input should prepare");
    assert!(enabled_input.prompt.contains("summary reasoning"));
}

#[test]
fn model_response_usage_audit_persists_when_visible_parts_are_empty() {
    let database_url = temp_database_url("short-term-memory-empty-visible-usage-audit");
    let session = create_test_session(&database_url, "empty visible usage audit");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "usage audit prompt")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let token_usage = ModelTokenUsage {
        input_tokens: 100,
        output_tokens: 8,
        total_tokens: 108,
        cached_input_tokens: Some(32),
        reasoning_tokens: Some(6),
    };
    let usage_estimate = TokenEstimateSnapshot {
        estimated_input_tokens: 90,
        message_token_estimate: 84,
        tool_schema_token_estimate: 6,
        context_window_limit: Some(8_192),
        effective_input_budget: Some(7_000),
        output_token_reserve: Some(512),
        rebased: false,
        over_budget: false,
        compaction_trigger_reached: false,
        checkpoint_code: None,
        model_code: context.model.model_code.clone(),
        model_name: context.model.model_name.clone(),
        model_protocol: "chatCompletions".into(),
        usage_source: "local_estimate".into(),
        usage_confidence: "estimated".into(),
    };
    let response = ModelResponse::from_assistant(
        AgentMessage::new(
            "assistant",
            vec![AgentMessagePart::Reasoning {
                text: "raw reasoning is not visible by default".into(),
            }],
        ),
        ModelFinish::Final,
        Some(token_usage),
        None,
        Some("resp-empty-visible".into()),
    );

    service
        .record_model_response_with_facts(
            &context,
            &response,
            tron_backend::application::short_term_memory::ModelResponseTranscriptFacts {
                usage_audit: Some(TokenUsageAuditSnapshot {
                    request_estimate: usage_estimate,
                    provider_usage: Some(token_usage),
                    cumulative_run_usage: Some(token_usage),
                    input_token_delta: Some(10),
                    input_token_ratio: Some(1.111_111_111),
                    calibration_factor: Some(1.111_111_111),
                    response_id: Some("resp-empty-visible".into()),
                    usage_source: "provider".into(),
                    usage_confidence: "actual".into(),
                }),
            },
        )
        .expect("usage audit event should persist without visible parts");

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let assistant_messages: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND run_code = ?2
               AND role = 'assistant'",
            [session.session_code.as_str(), context.run_code.as_str()],
            |row| row.get(0),
        )
        .expect("assistant message count should be queryable");
    assert_eq!(assistant_messages, 0);
    let payload: String = connection
        .query_row(
            "SELECT payload_json
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND run_code = ?2
               AND event_type = 'agent.model.responded'
             ORDER BY seq DESC
             LIMIT 1",
            [session.session_code.as_str(), context.run_code.as_str()],
            |row| row.get(0),
        )
        .expect("responded event should be persisted");
    assert!(payload.contains("\"usageAudit\""));
    assert!(payload.contains("\"tokenUsage\""));
    assert!(payload.contains("\"cachedInputTokens\":32"));
    assert!(payload.contains("\"reasoningTokens\":6"));
    assert!(payload.contains("\"messageCode\":null"));
}

#[test]
fn prepare_model_context_enforces_part_visibility_replay_policy() {
    let database_url = temp_database_url("short-term-memory-part-visibility");
    let session = create_test_session(&database_url, "part visibility");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "visibility user prompt")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_model_response(
            &context,
            &ModelResponse::final_output("visible assistant ui text"),
        )
        .expect("assistant response should be recorded");

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let persisted_text_visibility: String = connection
        .query_row(
            "SELECT visibility
             FROM agent_transcript_parts
             WHERE session_code = ?1
               AND part_type = 'text'
               AND content_text = 'visible assistant ui text'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("assistant text visibility should be queryable");
    assert_eq!(persisted_text_visibility, "model");
    connection
        .execute(
            "INSERT INTO agent_transcript_messages
             (id, message_code, session_code, workspace_code, run_code, seq, role, message_type, status, agent_code, created_at, completed_at)
             VALUES
             (910000000000000001, 'ATM-VISIBILITY-POLICY', ?1, 'TRON_CORE', 'RUN-VISIBILITY-POLICY', 100, 'assistant', 'normal', 'completed', 'main-agent', 1, 1)",
            [session.session_code.as_str()],
        )
        .expect("visibility policy message should be insertable");
    connection
        .execute(
            "INSERT INTO agent_transcript_parts
             (id, part_code, session_code, workspace_code, message_code, seq, part_type, status, content_text, content_json, visibility, created_at, completed_at)
             VALUES
             (910000000000000002, 'ATP-UI-TEXT', ?1, 'TRON_CORE', 'ATM-VISIBILITY-POLICY', 1, 'text', 'completed', 'ui-only annotation should not replay', NULL, 'ui', 1, 1),
             (910000000000000003, 'ATP-AUDIT-TEXT', ?1, 'TRON_CORE', 'ATM-VISIBILITY-POLICY', 2, 'text', 'completed', 'audit secret should not replay', NULL, 'audit', 1, 1),
             (910000000000000004, 'ATP-INTERNAL-CONTEXT', ?1, 'TRON_CORE', 'ATM-VISIBILITY-POLICY', 3, 'context', 'completed', 'internal context should not replay', '{\"kind\":\"internal_test\"}', 'internal', 1, 1)",
            [session.session_code.as_str()],
        )
        .expect("visibility policy parts should be insertable");

    let rendered = service
        .prepare_model_context(&context)
        .expect("model context should prepare")
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");

    assert!(rendered.contains("visibility user prompt"));
    assert!(rendered.contains("visible assistant ui text"));
    assert!(!rendered.contains("ui-only annotation should not replay"));
    assert!(!rendered.contains("audit secret should not replay"));
    assert!(!rendered.contains("internal context should not replay"));
}

#[test]
fn prepare_model_context_replays_only_messages_after_v2_checkpoint_source_end_seq() {
    let database_url = temp_database_url("short-term-memory-v2-replay-boundary");
    let session = create_test_session(&database_url, "v2 replay");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "pre-checkpoint-user-unique")
        .expect("pre-checkpoint input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_model_response(
            &context,
            &ModelResponse::final_output("pre-checkpoint assistant"),
        )
        .expect("assistant response should be recorded");
    create_legacy_full_source_stub_compaction(
        &service,
        &database_url,
        CreateCompactionCommand {
            session_code: session.session_code.clone(),
            reason: Some("v2-boundary".into()),
            note: None,
            todo_enabled: Some(false),
        },
        "## Goal\nReplay boundary\n\n## Decisions\nUse v2 replay boundary.",
    );
    context.run_code = "RUN-STMEM-AFTER-CHECKPOINT".into();
    let input_code = service
        .record_user_input(&context, "post-checkpoint-user-unique")
        .expect("post-checkpoint input should be recorded");
    context.short_term_memory.as_mut().unwrap().input_code = input_code;
    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare");
    let rendered = prepared
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");
    assert!(rendered.contains("Conversation checkpoint:"));
    assert!(rendered.contains("post-checkpoint-user-unique"));
    assert_eq!(rendered.matches("pre-checkpoint-user-unique").count(), 1);
}

#[test]
fn manual_compaction_conflicts_when_session_has_active_run() {
    let database_url = temp_database_url("short-term-memory-manual-conflict");
    let session = create_test_session(&database_url, "manual conflict");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "active run blocks manual compaction")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_run_started(&context)
        .expect("run start should be recorded");

    let conflict = service
        .prepare_manual_model_compaction_input(
            CreateCompactionCommand {
                session_code: session.session_code.clone(),
                reason: Some("manual".into()),
                note: None,
                todo_enabled: Some(false),
            },
            Some(8_192),
            Some(128),
        )
        .expect_err("manual compaction should conflict with active run");
    assert_eq!(conflict, ShortTermMemoryError::CompactionConflict);

    service
        .recover_goal_state_on_startup()
        .expect("startup recovery should close stale runs");
    let mut recovered_context = base_context(&session.session_code);
    for index in 0..6 {
        recovered_context.run_code = format!("RUN-STMEM-RECOVERED-MANUAL-SEED-{index}");
        service
            .record_user_input(
                &recovered_context,
                &format!(
                    "recovered manual compaction seed {index} {}",
                    "R".repeat(2_200)
                ),
            )
            .expect("recovered compaction seed input should be recorded");
    }
    create_stub_model_compaction(
        &service,
        CreateCompactionCommand {
            session_code: session.session_code.clone(),
            reason: Some("manual-after-recovery".into()),
            note: None,
            todo_enabled: Some(false),
        },
        "## Goal\nRecovered manual compaction\n\n## Decisions\nProceed after stale run recovery.",
    );
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let recovered_events: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.run.recovered'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("recovery event count should be queryable");
    assert_eq!(recovered_events, 1);
}

#[test]
fn repository_compaction_write_rechecks_active_run_inside_transaction() {
    let database_url = temp_database_url("short-term-memory-compaction-transaction-idle-check");
    let session = create_test_session(&database_url, "compaction transaction idle check");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "active run should block repository compaction")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_run_started(&context)
        .expect("run start should be recorded");

    let now = current_timestamp();
    let checkpoint_message_code = "ATM-REPOSITORY-IDLE-CHECK";
    let compaction = ShortTermMemoryCompaction {
        session_code: session.session_code.clone(),
        checkpoint_message_code: checkpoint_message_code.into(),
        status: "completed".into(),
        source_start_seq: 1,
        source_end_seq: 1,
        tail_start_seq: 1,
        selector_version: 2,
        coverage: Some("full_source_includes_tail".into()),
        summary_token_estimate: 1,
        created_at: now.clone(),
    };
    let message = ShortTermMemoryMessage {
        id: 0,
        message_code: checkpoint_message_code.into(),
        session_code: session.session_code.clone(),
        workspace_code: "TRON_CORE".into(),
        run_code: None,
        seq: 0,
        role: "checkpoint".into(),
        message_type: "compaction".into(),
        status: "completed".into(),
        agent_code: Some("main-agent".into()),
        model_code: None,
        model_provider: None,
        model_name: None,
        model_protocol: None,
        finish_reason: None,
        error_json: None,
        input_tokens: None,
        output_tokens: None,
        reasoning_tokens: None,
        total_tokens: None,
        metadata_json: None,
        created_at: now.clone(),
        completed_at: Some(now.clone()),
    };
    let part = ShortTermMemoryPart {
        id: 0,
        part_code: String::new(),
        session_code: session.session_code.clone(),
        workspace_code: "TRON_CORE".into(),
        message_code: String::new(),
        seq: 1,
        part_type: "compaction_summary".into(),
        status: "completed".into(),
        content_text: Some("should not be written while run is active".into()),
        content_json: Some(json!({
            "sourceStartSeq": 1,
            "sourceEndSeq": 1,
            "tailStartSeq": 1,
            "selectorVersion": 2,
            "coverage": "full_source_includes_tail"
        })),
        provider_metadata_json: None,
        visibility: "model".into(),
        created_at: now.clone(),
        completed_at: Some(now.clone()),
    };
    let event = ShortTermMemoryEvent {
        id: 0,
        event_code: String::new(),
        session_code: session.session_code.clone(),
        workspace_code: "TRON_CORE".into(),
        seq: 0,
        run_code: None,
        input_code: None,
        message_code: Some(checkpoint_message_code.into()),
        part_code: None,
        event_type: "agent.compaction.completed".into(),
        event_source: "COMPACTION".into(),
        payload_json: json!({"checkpointMessageCode": checkpoint_message_code}),
        metadata_json: None,
        occurred_at: now.clone(),
        created_at: now,
    };

    let mut repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should reconnect");
    let error = repository
        .create_compaction(&compaction, &message, &[part], Some(&event), true)
        .expect_err("repository write should recheck active runs inside its transaction");
    assert_eq!(error, ShortTermMemoryError::CompactionConflict);

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let persisted_checkpoint_rows: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_messages
             WHERE message_code = ?1",
            [checkpoint_message_code],
            |row| row.get(0),
        )
        .expect("checkpoint row count should be queryable");
    assert_eq!(persisted_checkpoint_rows, 0);
}

#[test]
fn run_lifecycle_uses_sqlite_session_lease() {
    let database_url = temp_database_url("short-term-memory-run-lease");
    let session = create_test_session(&database_url, "run lease");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "lease first run")
        .expect("first input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_run_started(&context)
        .expect("first run should acquire lease");

    let mut second_context = base_context(&session.session_code);
    second_context.run_code = "RUN-STMEM-SECOND-LEASE".into();
    second_context.request_id = Some("REQ-STMEM-SECOND-LEASE".into());
    second_context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code: context
            .short_term_memory
            .as_ref()
            .expect("short-term memory context should exist")
            .input_code
            .clone(),
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let second_start_error = service
        .record_run_started(&second_context)
        .expect_err("second open run should be rejected by sqlite lease");
    assert_eq!(second_start_error, ShortTermMemoryError::RunConflict);

    service
        .record_run_completed(&context)
        .expect("run completion should release lease");
    service
        .record_run_started(&second_context)
        .expect("new run should acquire lease after completion releases it");

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let active_lease_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_session_run_leases
             WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("run lease count should be queryable");
    assert_eq!(active_lease_count, 1);
}

#[test]
fn tool_output_externalization_persists_full_metadata_and_cleans_orphan_on_db_failure() {
    let database_url = temp_database_url("short-term-memory-tool-cleanup");
    let session = create_test_session(&database_url, "tool cleanup");
    let artifact_root = temp_artifact_root("short-term-memory-tool-cleanup");
    let mut config = ShortTermMemoryConfig::default();
    config.tool_output = ToolOutputConfig {
        root: artifact_root.display().to_string(),
        externalize_above_bytes: 8,
    };
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "externalize failure cleanup")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    connection
        .execute_batch(
            "CREATE TRIGGER fail_tool_completed_event
             BEFORE INSERT ON agent_transcript_events
             WHEN NEW.event_type = 'agent.tool.completed'
             BEGIN
               SELECT RAISE(ABORT, 'tool event blocked');
             END;",
        )
        .expect("tool failure trigger should be installable");
    let observation = ToolObservation {
        tool_name: "shell.exec".into(),
        output: "line-one\nline-two\nline-three".into(),
    };
    let failed = service.record_tool_result(
        &context,
        &observation,
        Some("call-cleanup".into()),
        1,
        ToolTranscriptStatus::Completed,
    );
    assert!(matches!(failed, Err(ShortTermMemoryError::StorageFailure)));
    let expected_artifact = artifact_root
        .join("TRON_CORE")
        .join(&session.session_code)
        .join(&context.run_code)
        .join("0001-shell.exec.txt");
    assert!(!expected_artifact.exists());
    let tool_result_rows: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_parts
             WHERE session_code = ?1
               AND part_type = 'tool_result'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool result row count should be queryable");
    assert_eq!(tool_result_rows, 0);

    connection
        .execute_batch("DROP TRIGGER fail_tool_completed_event;")
        .expect("tool failure trigger should drop");
    connection
        .execute_batch(
            "CREATE TRIGGER fail_tool_result_created_event
             BEFORE INSERT ON agent_transcript_events
             WHEN NEW.event_type = 'agent.message.tool_result_created'
             BEGIN
               SELECT RAISE(ABORT, 'tool result created event blocked');
             END;",
        )
        .expect("second tool failure trigger should be installable");
    let failed_second_event = service.record_tool_result(
        &context,
        &observation,
        Some("call-cleanup-created".into()),
        2,
        ToolTranscriptStatus::Completed,
    );
    assert!(matches!(
        failed_second_event,
        Err(ShortTermMemoryError::StorageFailure)
    ));
    let second_artifact = artifact_root
        .join("TRON_CORE")
        .join(&session.session_code)
        .join(&context.run_code)
        .join("0002-shell.exec.txt");
    assert!(!second_artifact.exists());
    let tool_result_rows_after_second_failure: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_parts
             WHERE session_code = ?1
               AND part_type = 'tool_result'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("tool result row count should be queryable after second failure");
    assert_eq!(tool_result_rows_after_second_failure, 0);

    connection
        .execute_batch("DROP TRIGGER fail_tool_result_created_event;")
        .expect("second tool failure trigger should drop");
    service
        .record_tool_result(
            &context,
            &observation,
            Some("call-cleanup".into()),
            1,
            ToolTranscriptStatus::Completed,
        )
        .expect("tool result should persist after trigger drop");
    let (content_text, content_json): (String, String) = connection
        .query_row(
            "SELECT content_text, content_json
             FROM agent_transcript_parts
             WHERE session_code = ?1
               AND part_type = 'tool_result'
             ORDER BY id DESC
             LIMIT 1",
            [session.session_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("tool metadata should be queryable");
    for required in [
        "sha256",
        "bytes",
        "chars",
        "lines",
        "rootConfigKey",
        "relativePath",
    ] {
        assert!(content_text.contains(required));
        assert!(content_json.contains(required));
    }
    for required in ["callIndex", "modelCallId"] {
        assert!(content_json.contains(required));
    }
    let parsed_part_json: serde_json::Value =
        serde_json::from_str(&content_json).expect("tool result json should parse");
    assert_eq!(parsed_part_json["callIndex"], json!(1));
    assert_eq!(parsed_part_json["modelCallId"], json!("call-cleanup"));
    let (completed_payload, created_payload): (String, String) = connection
        .query_row(
            "SELECT
                max(CASE WHEN event_type = 'agent.tool.completed' THEN payload_json END),
                max(CASE WHEN event_type = 'agent.message.tool_result_created' THEN payload_json END)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND message_code IN (
                   SELECT message_code
                   FROM agent_transcript_parts
                   WHERE session_code = ?1
                     AND part_type = 'tool_result'
                     AND content_json = ?2
               )",
            (session.session_code.as_str(), content_json.as_str()),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("tool event payloads should be queryable");
    for payload in [completed_payload, created_payload] {
        let parsed_payload: serde_json::Value =
            serde_json::from_str(&payload).expect("tool event payload should parse");
        assert_eq!(parsed_payload["callIndex"], json!(1));
        assert_eq!(parsed_payload["modelCallId"], json!("call-cleanup"));
    }
    assert!(expected_artifact.exists());
}

#[test]
#[cfg(unix)]
fn tool_output_orphan_sweep_deletes_unreferenced_artifacts_and_keeps_referenced_artifacts() {
    let database_url = temp_database_url("short-term-memory-tool-orphan-sweep");
    let session = create_test_session(&database_url, "tool orphan sweep");
    let artifact_root = temp_artifact_root("short-term-memory-tool-orphan-sweep");
    let mut config = ShortTermMemoryConfig::default();
    config.tool_output = ToolOutputConfig {
        root: artifact_root.display().to_string(),
        externalize_above_bytes: 8,
    };
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "externalize orphan sweep")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    let tool_message = service
        .record_tool_result(
            &context,
            &ToolObservation {
                tool_name: "shell.exec".into(),
                output: "line-one\nline-two\nline-three".into(),
            },
            Some("call-retained".into()),
            1,
            ToolTranscriptStatus::Completed,
        )
        .expect("referenced tool result should persist");
    let referenced_relative_path = match &tool_message.parts[0] {
        AgentMessagePart::ToolResult { metadata, .. } => metadata
            .as_ref()
            .and_then(|value| value.get("relativePath"))
            .and_then(|value| value.as_str())
            .expect("referenced artifact should include relative path")
            .to_string(),
        _ => panic!("tool result message should contain a tool result part"),
    };
    let referenced_path = artifact_root.join(referenced_relative_path);
    assert!(referenced_path.exists());

    let orphan_path = artifact_root
        .join("TRON_CORE")
        .join(&session.session_code)
        .join(&context.run_code)
        .join("9999-orphan.txt");
    fs::write(&orphan_path, "orphan output").expect("orphan artifact should be writable");
    mark_file_old(&referenced_path);
    mark_file_old(&orphan_path);

    let deleted = service
        .sweep_externalized_tool_output_orphans()
        .expect("orphan sweep should complete");

    assert_eq!(deleted, 1);
    assert!(referenced_path.exists());
    assert!(!orphan_path.exists());
}

#[test]
fn working_state_budget_prioritizes_goal_before_todos() {
    let database_url = temp_database_url("short-term-memory-working-state-budget");
    let session = create_test_session(&database_url, "working state budget");
    let mut config = ShortTermMemoryConfig::default();
    config
        .context
        .blocks
        .current_session_working_state
        .max_ratio_bps = None;
    config
        .context
        .blocks
        .current_session_working_state
        .max_tokens = Some(450);
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "prepare goal under budget pressure")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });
    service
        .set_goal(SetGoalCommand {
            session_code: session.session_code.clone(),
            objective: "Preserve the active session goal under pressure".into(),
            max_runs: Some(5),
        })
        .expect("goal should be set");
    service
        .replace_todos(
            &context,
            vec![TodoUpdateItem {
                todo_code: Some("TODO-BUDGET-PRESSURE".into()),
                position: 0,
                content:
                    "TODO-SHOULD-TRUNCATE before this lower-priority active todo displaces the goal"
                        .into(),
                status: "pending".into(),
                priority: "high".into(),
            }],
        )
        .expect("todo should be recorded");

    let prepared = service
        .prepare_model_context(&context)
        .expect("prepared context should be returned");
    let rendered = prepared
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");

    assert!(rendered.contains("<active-goal"));
    assert!(rendered.contains("Preserve the active session goal under pressure"));
    // Prepared DTOs preserve the complete authorized source. The actual SDK
    // admission/reduction is exercised by provider_working_bound_preserves_goal_before_todos.
    assert!(rendered.contains("TODO-SHOULD-TRUNCATE"));
    assert!(
        prepared
            .budget_breakdown
            .as_ref()
            .unwrap()
            .working_state_tokens
            > 450
    );
    assert!(
        prepared
            .sources
            .as_ref()
            .unwrap()
            .working
            .iter()
            .any(|message| message.text_content().contains("TODO-SHOULD-TRUNCATE"))
    );
}

#[test]
fn prepared_context_hash_includes_recent_transcript_delta_for_stateful_responses() {
    let database_url = temp_database_url("short-term-memory-context-hash-delta");
    let session = create_test_session(&database_url, "context hash delta");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    context.model.protocol = AgentModelProtocol::Responses;
    context.model.responses_stateful_enabled = true;
    let input_code = service
        .record_user_input(&context, "lookup current workspace")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("stable seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let before_hash = service
        .prepare_model_context(&context)
        .expect("initial context should prepare")
        .context_hash
        .expect("context hash should be present");
    service
        .record_model_response(
            &context,
            &ModelResponse::from_tool_calls(vec![ToolCall {
                call_id: Some("call-hash".into()),
                tool_name: "workspace.lookup".into(),
                input: json!({"workspaceCode": "TRON_CORE"}),
                input_error: None,
            }]),
        )
        .expect("assistant tool call should be recorded");
    service
        .record_tool_result(
            &context,
            &ToolObservation {
                tool_name: "workspace.lookup".into(),
                output: "{\"name\":\"Tron Core\"}".into(),
            },
            Some("call-hash".into()),
            1,
            ToolTranscriptStatus::Completed,
        )
        .expect("tool result should be recorded");
    let after_hash = service
        .prepare_model_context(&context)
        .expect("context should prepare after transcript delta")
        .context_hash
        .expect("context hash should be present after delta");

    assert_ne!(before_hash, after_hash);
}

#[test]
fn history_retention_preserves_current_user_input_when_single_turn_exceeds_budget() {
    let database_url = temp_database_url("short-term-memory-current-input");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("current input pinning".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.auto_enabled = false;
    config.compaction.retained_history_ratio_bps = 2_000;
    let service = ShortTermMemoryService::new(repository, config);
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(512);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let long_input = format!(
        "current-user-input-start {} current-user-input-end",
        "preserve-this-intent ".repeat(160)
    );
    let input_code = service
        .record_user_input(&context, &long_input)
        .expect("long user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let prepared = service
        .prepare_model_context(&context)
        .expect("prepared context should be returned even when over budget");
    let rendered = prepared
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(prepared.over_budget);
    assert!(rendered.contains(&long_input));
}

#[test]
fn full_context_preserves_tool_call_result_structure_when_single_turn_exceeds_budget() {
    let database_url = temp_database_url("short-term-memory-tool-pair-budget");
    let session = create_test_session(&database_url, "tool pair budget");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.auto_enabled = false;
    config.compaction.retained_history_ratio_bps = 2_000;
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(512);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let input_code = service
        .record_user_input(&context, "use the structured tool pair")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_model_response(
            &context,
            &ModelResponse::from_tool_calls(vec![ToolCall {
                call_id: Some("call-structured".into()),
                tool_name: "shell.exec".into(),
                input: json!({"cmd": "printf structured"}),
                input_error: None,
            }]),
        )
        .expect("tool call response should be recorded");
    service
        .record_tool_result(
            &context,
            &ToolObservation {
                tool_name: "shell.exec".into(),
                output: "large tool output ".repeat(240),
            },
            Some("call-structured".into()),
            1,
            ToolTranscriptStatus::Completed,
        )
        .expect("tool result should be recorded");

    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare");
    let flattened_parts = prepared
        .messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .collect::<Vec<_>>();
    let tool_call_index = flattened_parts
        .iter()
        .position(|part| {
            matches!(
                part,
                AgentMessagePart::ToolCall {
                    call_id: Some(call_id),
                    ..
                } if call_id == "call-structured"
            )
        })
        .expect("tool call should remain structured");
    let tool_result_index = flattened_parts
        .iter()
        .position(|part| {
            matches!(
                part,
                AgentMessagePart::ToolResult {
                    call_id: Some(call_id),
                    output,
                    ..
                } if call_id == "call-structured"
                    && output.contains("large tool output")
            )
        })
        .expect("tool result should remain structured");

    assert!(prepared.over_budget);
    assert!(!prepared.compaction_required);
    assert!(
        tool_call_index < tool_result_index,
        "tool result should remain after its tool call"
    );
}

#[test]
fn model_compaction_keeps_current_tool_pair_as_raw_tail() {
    let database_url = temp_database_url("short-term-memory-current-tool-pair-pinned");
    let session = create_test_session(&database_url, "current tool pair pinned");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.retained_history_ratio_bps = 1;

    let service = ShortTermMemoryService::with_id_provider(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
        Arc::new(FixedSnowflakeIdProvider::new(803_000_000_000_000_000)),
    );
    let mut old_context = base_context(&session.session_code);
    old_context.run_code = "RUN-STMEM-TOOL-OLD".into();
    service
        .record_user_input(&old_context, "old context before current tool")
        .expect("old input should be recorded");

    let mut context = base_context(&session.session_code);
    context.run_code = "RUN-STMEM-TOOL-CURRENT".into();
    let input_code = service
        .record_user_input(&context, "current run uses structured tool")
        .expect("current input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_model_response(
            &context,
            &ModelResponse::from_tool_calls(vec![ToolCall {
                call_id: Some("call-pinned-tool".into()),
                tool_name: "shell.exec".into(),
                input: json!({"cmd": "printf pinned"}),
                input_error: None,
            }]),
        )
        .expect("tool call should be recorded");
    service
        .record_tool_result(
            &context,
            &ToolObservation {
                tool_name: "shell.exec".into(),
                output: "pinned tool result".into(),
            },
            Some("call-pinned-tool".into()),
            1,
            ToolTranscriptStatus::Completed,
        )
        .expect("tool result should be recorded");

    let compaction_input = service
        .prepare_automatic_model_compaction_input(
            &session.session_code,
            "auto_compaction_trigger",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            Some(8_192),
            context.model.max_tokens,
            Some(&context.run_code),
            None,
            None,
        )
        .expect("model compaction input should prepare");
    assert_eq!(
        compaction_input.run_code.as_deref(),
        Some(context.run_code.as_str()),
        "automatic compaction events must stay attached to the active AG-UI run",
    );
    assert!(
        compaction_input
            .pinned_start_seq
            .is_some_and(|pinned_seq| compaction_input.source_end_seq < pinned_seq),
        "latest tool pair must stay outside checkpoint coverage"
    );
    let model = service.compaction_model_config(&context.model);
    service
        .commit_model_compaction(
            compaction_input,
            ModelCompactionOutput {
                summary: "## Goal\nOld context compacted\n\n## Decisions\nKeep current tool raw."
                    .into(),
                token_usage: None,
                provider_metadata: None,
            },
            &model,
        )
        .expect("model compaction should commit");

    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare after compaction");
    let flattened_parts = prepared
        .messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .collect::<Vec<_>>();
    let tool_call_index = flattened_parts
        .iter()
        .position(|part| {
            matches!(
                part,
                AgentMessagePart::ToolCall {
                    call_id: Some(call_id),
                    ..
                } if call_id == "call-pinned-tool"
            )
        })
        .expect("current tool call should remain structured");
    let tool_result_index = flattened_parts
        .iter()
        .position(|part| {
            matches!(
                part,
                AgentMessagePart::ToolResult {
                    call_id: Some(call_id),
                    output,
                    ..
                } if call_id == "call-pinned-tool" && output.contains("pinned tool result")
            )
        })
        .expect("current tool result should remain structured");
    assert!(tool_call_index < tool_result_index);
}

#[test]
fn auto_compaction_preserves_complete_same_run_tool_chain_and_refuses_to_split_latest_turn() {
    let database_url = temp_database_url("short-term-memory-same-run-tool-chain");
    let session = create_test_session(&database_url, "same run tool chain");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.trigger_ratio_bps = 3_000;
    config.compaction.retained_history_ratio_bps = 1;
    let service = ShortTermMemoryService::with_id_provider(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
        Arc::new(FixedSnowflakeIdProvider::new(806_000_000_000_000_000)),
    );

    let mut context = base_context(&session.session_code);
    context.run_code = "RUN-STMEM-SAME-RUN-LONG-TOOLS".into();
    context.model.context_window_tokens = Some(100_000);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let input_code = service
        .record_user_input(&context, "research task with a long tool chain")
        .expect("user input should be recorded");
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        input_code,
        vec![AgentMessage::system("system seed")],
    );

    for index in 0..120 {
        let call_id = format!("call-same-run-{index}");
        service
            .record_model_response(
                &context,
                &ModelResponse::from_tool_calls(vec![ToolCall {
                    call_id: Some(call_id.clone()),
                    tool_name: "bash_exec".into(),
                    input: json!({"command": format!("bad shell pipeline {index}")}),
                    input_error: None,
                }]),
            )
            .expect("tool call should be recorded");
        let output = if index == 0 {
            "early-tool-failure-0 command failed to start".to_string()
        } else if index == 119 {
            "latest-tool-result-119 source evidence".to_string()
        } else {
            format!("tool-result-{index} command failed")
        };
        service
            .record_tool_result(
                &context,
                &ToolObservation {
                    tool_name: "bash_exec".into(),
                    output,
                },
                Some(call_id),
                index + 1,
                ToolTranscriptStatus::Failed,
            )
            .expect("tool result should be recorded");
    }

    let baseline = service
        .prepare_model_context(&context)
        .expect("baseline context should prepare");
    context.model.context_window_tokens = Some(context_window_for_estimate_ratio(
        baseline.estimated_tokens,
        5_000,
    ));

    let before = service
        .prepare_model_context(&context)
        .expect("same-run tool chain context should prepare");

    assert!(before.compaction_trigger_reached);
    assert!(!before.compaction_required);
    assert!(!before.over_budget);
    let breakdown = before.budget_breakdown.as_ref().unwrap();
    assert_eq!(breakdown.compressed_history_message_count, Some(0));
    assert_eq!(breakdown.raw_tail_message_count, Some(241));
    let rendered = rendered_messages(&before.messages);
    assert!(rendered.contains("early-tool-failure-0"));
    assert!(rendered.contains("latest-tool-result-119"));
    let flattened = before
        .messages
        .iter()
        .flat_map(|message| &message.parts)
        .collect::<Vec<_>>();
    for index in 0..120 {
        let id = format!("call-same-run-{index}");
        let call=flattened.iter().position(|part|matches!(part,AgentMessagePart::ToolCall{call_id:Some(call_id),..} if call_id==&id)).unwrap();
        let result=flattened.iter().position(|part|matches!(part,AgentMessagePart::ToolResult{call_id:Some(call_id),..} if call_id==&id)).unwrap();
        assert!(call < result);
    }
    assert!(matches!(
        service.prepare_automatic_model_compaction_input(
            &session.session_code,
            "auto_compaction_trigger",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false
            },
            context.model.context_window_tokens,
            context.model.max_tokens,
            Some(&context.run_code),
            None,
            None
        ),
        Err(ShortTermMemoryError::ContextBudgetExceeded)
    ));
    context.model.context_window_tokens = Some(baseline.estimated_tokens / 2);
    let oversized = service.prepare_model_context(&context).unwrap();
    assert!(oversized.over_budget);
    assert!(!oversized.compaction_required);
    assert!(rendered_messages(&oversized.messages).contains("early-tool-failure-0"));
    assert!(
        service
            .latest_compaction(&session.session_code)
            .unwrap()
            .is_none()
    );
}

#[test]
fn prepare_model_context_keeps_full_history_when_recent_under_raw_retain_policy() {
    let database_url = temp_database_url("short-term-memory-full-history-under-retain-policy");
    let session = create_test_session(&database_url, "full history under retain policy");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.retained_history_ratio_bps = 2_000;
    config.compaction.retained_history_ratio_bps = 2_000;

    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(20_000);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let mut latest_input_code = String::new();
    for label in ["A", "B", "C", "D", "E", "F", "G"] {
        context.run_code = format!("RUN-STMEM-{label}");
        latest_input_code = service
            .record_user_input(&context, &format!("history-{label}-unique"))
            .expect("history input should be recorded");
    }
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        latest_input_code,
        vec![AgentMessage::system("system seed")],
    );

    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare");
    let rendered = rendered_messages(&prepared.messages);

    assert!(!prepared.over_budget);
    assert!(!prepared.compaction_required);
    let breakdown = prepared
        .budget_breakdown
        .as_ref()
        .expect("budget breakdown should be present");
    assert_eq!(breakdown.compressed_history_message_count, Some(0));
    assert!(
        breakdown.recent_transcript_tokens
            < breakdown
                .effective_retained_history_tokens
                .expect("retained history token budget should be recorded")
    );
    for label in ["A", "B", "C", "D", "E", "F", "G"] {
        assert!(rendered.contains(&format!("history-{label}-unique")));
    }
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let requested_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.context.compaction_requested'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("compaction request count should be queryable");
    assert_eq!(requested_count, 0);
}

#[test]
fn prepare_model_context_does_not_compact_or_fail_below_trigger_threshold() {
    let database_url = temp_database_url("short-term-memory-trigger-79");
    let session = create_test_session(&database_url, "trigger 79");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(100_000);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let mut latest_input_code = String::new();
    for label in ["A", "B", "C", "D", "E", "F"] {
        context.run_code = format!("RUN-STMEM-TRIGGER-79-{label}");
        latest_input_code = service
            .record_user_input(
                &context,
                &format!("below-trigger-{label}-unique {}", "x".repeat(240)),
            )
            .expect("history input should be recorded");
    }
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        latest_input_code,
        vec![AgentMessage::system("system seed")],
    );
    let baseline = service
        .prepare_model_context(&context)
        .expect("baseline context should prepare");
    context.model.context_window_tokens = Some(context_window_for_estimate_ratio(
        baseline.estimated_tokens,
        7_900,
    ));

    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare");

    assert!(!prepared.compaction_trigger_reached);
    assert!(!prepared.compaction_required);
    assert!(!prepared.over_budget);
    let breakdown = prepared
        .budget_breakdown
        .as_ref()
        .expect("budget breakdown should be present");
    assert!(!breakdown.compaction_trigger_reached);
    assert!(!breakdown.input_budget_exceeded);
}

#[test]
fn prepare_model_context_requests_compaction_when_estimated_input_reaches_trigger_threshold() {
    let database_url = temp_database_url("short-term-memory-recent-retain-compaction");
    let session = create_test_session(&database_url, "recent retain compaction");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.retained_history_ratio_bps = 2_000;

    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(100_000);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let mut latest_input_code = String::new();
    for label in ["A", "B", "C", "D", "E", "F", "G"] {
        context.run_code = format!("RUN-STMEM-RETAIN-{label}");
        latest_input_code = service
            .record_user_input(
                &context,
                &format!("retain-window-{label}-unique {}", "x".repeat(360)),
            )
            .expect("history input should be recorded");
    }
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        latest_input_code,
        vec![AgentMessage::system("system seed")],
    );
    let baseline = service
        .prepare_model_context(&context)
        .expect("baseline context should prepare");
    context.model.context_window_tokens = Some(context_window_for_estimate_ratio(
        baseline.estimated_tokens,
        8_000,
    ));

    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare");
    let rendered = rendered_messages(&prepared.messages);

    assert!(prepared.compaction_trigger_reached);
    assert!(!prepared.over_budget);
    assert!(prepared.compaction_required);
    assert!(!rendered.contains("retain-window-A-unique"));
    assert!(rendered.contains("retain-window-G-unique"));
    let breakdown = prepared
        .budget_breakdown
        .as_ref()
        .expect("budget breakdown should be present");
    assert!(
        breakdown
            .compressed_history_message_count
            .expect("compressed history message count should be recorded")
            > 0
    );
    assert!(
        breakdown.recent_transcript_tokens
            <= breakdown
                .effective_retained_history_tokens
                .expect("retained history token budget should be recorded")
    );

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let payload_json: String = connection
        .query_row(
            "SELECT payload_json
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.context.compaction_requested'
             ORDER BY id DESC
             LIMIT 1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("compaction request should be recorded");
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).expect("payload should be JSON");
    assert_eq!(
        payload["compressedHistoryMessageCount"]
            .as_i64()
            .unwrap_or_default()
            > 0,
        true
    );
    assert!(payload["budgetBreakdown"].is_object());
}

#[test]
fn prepare_model_context_keeps_pinned_current_run_when_triggered_but_under_budget() {
    let database_url = temp_database_url("short-term-memory-trigger-85-no-head");
    let session = create_test_session(&database_url, "trigger 85 no head");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(100_000);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    context.run_code = "RUN-STMEM-TRIGGER-85-CURRENT".into();
    let current_prompt = format!("current-only-triggered {}", "C".repeat(360));
    let input_code = service
        .record_user_input(&context, &current_prompt)
        .expect("current input should be recorded");
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        input_code,
        vec![AgentMessage::system("system seed")],
    );
    let baseline = service
        .prepare_model_context(&context)
        .expect("baseline context should prepare");
    context.model.context_window_tokens = Some(context_window_for_estimate_ratio(
        baseline.estimated_tokens,
        8_500,
    ));

    let prepared = service
        .prepare_model_context(&context)
        .expect("triggered context should still prepare");

    assert!(prepared.compaction_trigger_reached);
    assert!(!prepared.compaction_required);
    assert!(!prepared.over_budget);
    assert!(rendered_messages(&prepared.messages).contains(&current_prompt));
    let breakdown = prepared
        .budget_breakdown
        .as_ref()
        .expect("budget breakdown should be present");
    assert_eq!(breakdown.compressed_history_message_count, Some(0));
    assert!(!breakdown.input_budget_exceeded);
}

#[test]
fn automatic_compaction_tail_limit_uses_history_budget_ratio() {
    let mut config = ShortTermMemoryConfig::default();
    let plan = config.model_context_budget_plan(Some(2_000), Some(0));
    assert_eq!(plan.effective_input_budget, 2_000);

    let database_url = temp_database_url("short-term-memory-ratio-tail-limit");
    let session = create_test_session(&database_url, "ratio tail limit");
    config.compaction.retained_history_ratio_bps = 2_000;
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    for index in 0..30 {
        context.run_code = format!("RUN-STMEM-RATIO-{index}");
        service
            .record_user_input(&context, &format!("ratio-tail-{index} {}", "R".repeat(80)))
            .expect("ratio input should be recorded");
    }

    let input = service
        .prepare_automatic_model_compaction_input(
            &session.session_code,
            "ratio-tail-limit",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            Some(2_000),
            Some(0),
            None,
            None,
            None,
        )
        .expect("ratio compaction input should prepare");

    assert_eq!(input.retained_history_ratio_bps, 2_000);
    assert_eq!(input.effective_retained_history_tokens, 400);
    assert!(input.source_end_seq < input.selection_source_end_seq);
    assert!(input.source_message_count < input.selection_source_message_count);
    assert!(!input.recent.contains("ratio-tail-0"));
    assert!(input.recent.contains("ratio-tail-29"));
}

#[test]
fn model_compaction_input_budget_reserves_compaction_output_tokens() {
    let database_url = temp_database_url("short-term-memory-compaction-input-budget");
    let session = create_test_session(&database_url, "compaction input budget");
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        ShortTermMemoryConfig::default(),
    );
    let mut context = base_context(&session.session_code);
    for index in 0..3 {
        context.run_code = format!("RUN-STMEM-COMPACTION-BUDGET-{index}");
        service
            .record_user_input(
                &context,
                &format!("budget-input-{index} {}", "B".repeat(2_200)),
            )
            .expect("history input should be recorded");
    }

    let default_reserve = service
        .prepare_manual_model_compaction_input(
            CreateCompactionCommand {
                session_code: session.session_code.clone(),
                reason: Some("manual-budget-default".into()),
                note: None,
                todo_enabled: Some(false),
            },
            Some(8_192),
            None,
        )
        .expect("manual compaction input should prepare with default reserve");
    let explicit_reserve = service
        .prepare_manual_model_compaction_input(
            CreateCompactionCommand {
                session_code: session.session_code.clone(),
                reason: Some("manual-budget-explicit".into()),
                note: None,
                todo_enabled: Some(false),
            },
            Some(8_192),
            Some(1_024),
        )
        .expect("manual compaction input should prepare with explicit reserve");

    assert_eq!(default_reserve.max_input_tokens, 4_096);
    assert_eq!(explicit_reserve.max_input_tokens, 7_168);
}

#[test]
fn auto_compaction_summarizes_head_and_replays_raw_tail_after_checkpoint() {
    let database_url = temp_database_url("short-term-memory-head-only-raw-tail");
    let session = create_test_session(&database_url, "head only raw tail");
    let mut config = ShortTermMemoryConfig::default();
    config
        .context
        .blocks
        .current_session_checkpoint
        .max_ratio_bps = None;
    config.context.blocks.current_session_checkpoint.max_tokens = Some(250);

    config.compaction.retained_history_ratio_bps = 8_000;
    let service = ShortTermMemoryService::with_id_provider(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
        Arc::new(FixedSnowflakeIdProvider::new(804_000_000_000_000_000)),
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(5_200);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let messages = [
        ("A", "RUN-STMEM-A", "head-A-old"),
        ("B", "RUN-STMEM-B", "head-B-old"),
        ("C", "RUN-STMEM-C", "head-C-old"),
        ("D", "RUN-STMEM-D", "head-D-old"),
        ("E", "RUN-STMEM-E", "tail-E-recent"),
        ("F", "RUN-STMEM-F", "tail-F-recent"),
        ("G", "RUN-STMEM-G", "tail-G-recent"),
        ("H", "RUN-STMEM-H", "tail-H-recent"),
    ];
    let mut latest_input_code = String::new();
    for (_, run_code, text) in messages {
        context.run_code = run_code.into();
        latest_input_code = service
            .record_user_input(&context, &format!("{text} {}", "x".repeat(360)))
            .expect("history input should be recorded");
    }
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        latest_input_code,
        vec![AgentMessage::system(format!(
            "large stable prefix {}",
            "P".repeat(1_800)
        ))],
    );

    let before = service
        .prepare_model_context(&context)
        .expect("over-budget context should prepare");
    assert!(before.compaction_trigger_reached);
    assert!(!before.over_budget);
    assert!(before.compaction_required);
    let before_rendered = rendered_messages(&before.messages);
    assert!(!before_rendered.contains("head-A-old"));
    assert!(before_rendered.contains("tail-H-recent"));

    let compaction_input = service
        .prepare_automatic_model_compaction_input(
            &session.session_code,
            "auto_compaction_trigger",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            Some(16_000),
            context.model.max_tokens,
            Some(&context.run_code),
            before.compaction_tail_start_seq,
            before.effective_retained_history_tokens,
        )
        .expect("auto compaction input should prepare");
    assert_eq!(compaction_input.coverage, "head_only_tail_replayed");
    assert_eq!(compaction_input.selector_version, 4);
    assert_eq!(compaction_input.retained_history_ratio_bps, 8_000);
    assert!(compaction_input.source_message_count > 0);
    assert!(
        compaction_input.source_message_count < compaction_input.selection_source_message_count
    );
    assert_eq!(compaction_input.selection_source_message_count, 8);
    assert!(compaction_input.prompt.contains("head-A-old"));
    assert!(compaction_input.prompt.contains("tail-H-recent"));
    assert_eq!(
        compaction_input.source_end_seq,
        compaction_input.tail_start_seq.saturating_sub(1)
    );
    assert!(compaction_input.selection_source_end_seq >= compaction_input.tail_start_seq);

    let model = service.compaction_model_config(&context.model);
    let checkpoint = service
        .commit_model_compaction(
            compaction_input,
            ModelCompactionOutput {
                summary: "## Goal\nNEW CHECKPOINT HEAD ONLY\n\n## Decisions\nOlder history was compacted."
                    .into(),
                token_usage: None,
                provider_metadata: None,
            },
            &model,
        )
        .expect("model compaction should commit");
    assert_eq!(
        checkpoint.coverage.as_deref(),
        Some("head_only_tail_replayed")
    );
    assert_eq!(checkpoint.selector_version, 4);
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let metadata: String = connection
        .query_row(
            "SELECT metadata_json FROM agent_transcript_messages WHERE message_code = ?1",
            [checkpoint.checkpoint_message_code.as_str()],
            |row| row.get(0),
        )
        .expect("checkpoint metadata should be queryable");
    assert!(metadata.contains("\"retainedHistoryRatioBps\":8000"));
    assert!(metadata.contains("\"effectiveRetainedHistoryTokens\""));
    assert!(metadata.contains("\"selectionSourceMessageCount\":8"));

    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare after checkpoint");
    let rendered = rendered_messages(&prepared.messages);
    assert!(rendered.contains("NEW CHECKPOINT HEAD ONLY"));
    assert!(!rendered.contains("Recent retained context"));
    assert!(!rendered.contains("head-A-old"));
    assert!(!rendered.contains("head-A-old"));
    assert!(rendered.contains("tail-H-recent"));
}

#[test]
fn manual_compaction_summarizes_head_and_replays_raw_tail_after_checkpoint() {
    let database_url = temp_database_url("short-term-memory-manual-head-only-raw-tail");
    let session = create_test_session(&database_url, "manual head only raw tail");
    let mut config = ShortTermMemoryConfig::default();
    config
        .context
        .blocks
        .current_session_checkpoint
        .max_ratio_bps = None;
    config.context.blocks.current_session_checkpoint.max_tokens = Some(250);

    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(8_192);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let messages = [
        ("A", "RUN-STMEM-MANUAL-A", "manual-head-A-old"),
        ("B", "RUN-STMEM-MANUAL-B", "manual-head-B-old"),
        ("C", "RUN-STMEM-MANUAL-C", "manual-head-C-old"),
        ("D", "RUN-STMEM-MANUAL-D", "manual-head-D-old"),
        ("E", "RUN-STMEM-MANUAL-E", "manual-tail-E-recent"),
        ("F", "RUN-STMEM-MANUAL-F", "manual-tail-F-recent"),
        ("G", "RUN-STMEM-MANUAL-G", "manual-tail-G-recent"),
        ("H", "RUN-STMEM-MANUAL-H", "manual-tail-H-recent"),
    ];
    let mut latest_input_code = String::new();
    for (_, run_code, text) in messages {
        context.run_code = run_code.into();
        latest_input_code = service
            .record_user_input(&context, &format!("{text} {}", "M".repeat(2_200)))
            .expect("history input should be recorded");
    }
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        latest_input_code,
        vec![AgentMessage::system("system seed")],
    );

    let compaction_input = service
        .prepare_manual_model_compaction_input(
            CreateCompactionCommand {
                session_code: session.session_code.clone(),
                reason: Some("manual_compaction_trigger".into()),
                note: None,
                todo_enabled: Some(false),
            },
            context.model.context_window_tokens,
            context.model.max_tokens,
        )
        .expect("manual compaction input should prepare");
    assert_eq!(compaction_input.strategy, "MANUAL_CHECKPOINT");
    assert_eq!(compaction_input.coverage, "head_only_tail_replayed");
    assert_eq!(compaction_input.selector_version, 4);
    assert!(compaction_input.source_message_count > 0);
    assert!(
        compaction_input.source_message_count < compaction_input.selection_source_message_count
    );
    assert!(compaction_input.prompt.contains("manual-head-A-old"));
    assert!(compaction_input.prompt.contains("manual-tail-H-recent"));
    assert_eq!(
        compaction_input.source_end_seq,
        compaction_input.tail_start_seq.saturating_sub(1)
    );

    let model = service.compaction_model_config(&context.model);
    let checkpoint = service
        .commit_model_compaction(
            compaction_input,
            ModelCompactionOutput {
                summary:
                    "## Goal\nMANUAL CHECKPOINT HEAD ONLY\n\n## Decisions\nOlder history was compacted."
                        .into(),
                token_usage: None,
                provider_metadata: None,
            },
            &model,
        )
        .expect("manual model compaction should commit");
    assert_eq!(
        checkpoint.coverage.as_deref(),
        Some("head_only_tail_replayed")
    );
    assert_eq!(checkpoint.selector_version, 4);

    let prepared = service
        .prepare_model_context(&context)
        .expect("model context should prepare after manual checkpoint");
    let rendered = rendered_messages(&prepared.messages);
    assert!(rendered.contains("MANUAL CHECKPOINT HEAD ONLY"));
    assert!(!rendered.contains("Recent retained context"));
    assert!(!rendered.contains("manual-head-A-old"));
    assert!(rendered.contains("manual-tail-H-recent"));
    let breakdown = prepared
        .budget_breakdown
        .as_ref()
        .expect("budget breakdown should be present");
    assert!(
        breakdown.raw_tail_message_count.unwrap_or_default() > 0,
        "manual compaction should leave raw tail messages for replay"
    );
}

#[test]
fn checkpoint_replay_boundary_depends_on_coverage() {
    let database_url = temp_database_url("short-term-memory-coverage-boundary");
    let legacy_session = create_test_session(&database_url, "legacy boundary");
    let head_only_session = create_test_session(&database_url, "head only boundary");
    let mut config = ShortTermMemoryConfig::default();

    config.compaction.retained_history_ratio_bps = 1;
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );

    let mut legacy_context = base_context(&legacy_session.session_code);
    legacy_context.run_code = "RUN-STMEM-LEGACY-PRE".into();
    service
        .record_user_input(&legacy_context, "legacy-pre-unique")
        .expect("legacy pre input should be recorded");
    create_legacy_full_source_stub_compaction(
        &service,
        &database_url,
        CreateCompactionCommand {
            session_code: legacy_session.session_code.clone(),
            reason: Some("legacy-boundary".into()),
            note: None,
            todo_enabled: Some(false),
        },
        "LEGACY CHECKPOINT",
    );
    legacy_context.run_code = "RUN-STMEM-LEGACY-POST".into();
    let input_code = service
        .record_user_input(&legacy_context, "legacy-post-unique")
        .expect("post-checkpoint input should be recorded");
    attach_short_term_memory(
        &mut legacy_context,
        &legacy_session.session_code,
        input_code,
        vec![AgentMessage::system("system seed")],
    );
    let legacy_rendered = rendered_messages(
        &service
            .prepare_model_context(&legacy_context)
            .expect("legacy context should prepare")
            .messages,
    );
    assert!(legacy_rendered.contains("LEGACY CHECKPOINT"));
    assert!(legacy_rendered.contains("Recent retained context"));
    assert!(legacy_rendered.contains("legacy-post-unique"));
    assert_eq!(legacy_rendered.matches("legacy-pre-unique").count(), 1);

    let mut head_only_context = base_context(&head_only_session.session_code);
    for (run_code, text) in [
        ("RUN-STMEM-HEAD-A", "head-only-head-A"),
        ("RUN-STMEM-HEAD-B", "head-only-head-B"),
        ("RUN-STMEM-TAIL-C", "head-only-tail-C"),
        ("RUN-STMEM-TAIL-D", "head-only-tail-D"),
    ] {
        head_only_context.run_code = run_code.into();
        service
            .record_user_input(&head_only_context, text)
            .expect("head-only input should be recorded");
    }
    let input = service
        .prepare_automatic_model_compaction_input(
            &head_only_session.session_code,
            "head-only-boundary",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            Some(8_192),
            Some(0),
            None,
            None,
            None,
        )
        .expect("head-only compaction input should prepare");
    service
        .commit_model_compaction(
            input,
            ModelCompactionOutput {
                summary: "HEAD ONLY CHECKPOINT".into(),
                token_usage: None,
                provider_metadata: None,
            },
            &service.compaction_model_config(&head_only_context.model),
        )
        .expect("head-only checkpoint should commit");
    head_only_context.run_code = "RUN-STMEM-HEAD-POST".into();
    let post_code = service
        .record_user_input(&head_only_context, "head-only-post-unique")
        .expect("post input should be recorded");
    attach_short_term_memory(
        &mut head_only_context,
        &head_only_session.session_code,
        post_code,
        vec![AgentMessage::system("system seed")],
    );
    let mut replay_config = ShortTermMemoryConfig::default();
    replay_config.compaction.retained_history_ratio_bps = 1;
    let replay_service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        replay_config,
    );
    let head_only_rendered = rendered_messages(
        &replay_service
            .prepare_model_context(&head_only_context)
            .expect("head-only context should prepare")
            .messages,
    );
    assert!(head_only_rendered.contains("HEAD ONLY CHECKPOINT"));
    assert!(!head_only_rendered.contains("Recent retained context"));
    assert!(!head_only_rendered.contains("head-only-head-A"));
    assert!(!head_only_rendered.contains("head-only-head-B"));
    assert!(!head_only_rendered.contains("head-only-tail-C"));
    assert!(head_only_rendered.contains("head-only-tail-D"));
    assert!(head_only_rendered.contains("head-only-post-unique"));
}

#[test]
fn oversized_current_run_without_compressible_head_does_not_request_compaction() {
    let database_url = temp_database_url("short-term-memory-no-compressible-head");
    let session = create_test_session(&database_url, "no compressible head");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.retained_history_ratio_bps = 2_000;
    let service = ShortTermMemoryService::new(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
    );
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(512);
    context.model.max_tokens = Some(0);
    context.tools.clear();
    let huge_current = format!("only-current-run {}", "C".repeat(4_000));
    let input_code = service
        .record_user_input(&context, &huge_current)
        .expect("current input should be recorded");
    attach_short_term_memory(
        &mut context,
        &session.session_code,
        input_code,
        vec![AgentMessage::system("system seed")],
    );

    let prepared = service
        .prepare_model_context(&context)
        .expect("oversized context should prepare");

    assert!(prepared.over_budget);
    assert!(!prepared.compaction_required);
    assert!(rendered_messages(&prepared.messages).contains("only-current-run"));
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let requested_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.context.compaction_requested'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("compaction request count should be queryable");
    assert_eq!(requested_count, 0);
}

#[test]
fn next_head_only_compaction_does_not_duplicate_previous_retained_recent() {
    let database_url = temp_database_url("short-term-memory-no-duplicate-retained-recent");
    let session = create_test_session(&database_url, "no duplicate retained recent");
    let mut config = ShortTermMemoryConfig::default();
    config.compaction.retained_history_ratio_bps = 1;

    let service = ShortTermMemoryService::with_id_provider(
        SqliteShortTermMemoryRepository::connect(&database_url)
            .expect("short-term memory repository should connect"),
        config,
        Arc::new(FixedSnowflakeIdProvider::new(805_000_000_000_000_000)),
    );
    let mut context = base_context(&session.session_code);
    for (run_code, text) in [
        ("RUN-STMEM-FIRST-A", "first-head-A"),
        ("RUN-STMEM-FIRST-B", "first-head-B"),
        ("RUN-STMEM-FIRST-C", "first-tail-C"),
        ("RUN-STMEM-FIRST-D", "first-tail-D"),
    ] {
        context.run_code = run_code.into();
        service
            .record_user_input(&context, text)
            .expect("first window input should be recorded");
    }
    let first_input = service
        .prepare_automatic_model_compaction_input(
            &session.session_code,
            "first_auto",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            Some(8_192),
            Some(0),
            None,
            None,
            None,
        )
        .expect("first auto compaction input should prepare");
    assert!(first_input.prompt.contains("first-tail-C"));
    service
        .commit_model_compaction(
            first_input,
            ModelCompactionOutput {
                summary: "## Goal\nFIRST HEAD CHECKPOINT\n\n## Decisions\nFirst head compacted."
                    .into(),
                token_usage: None,
                provider_metadata: None,
            },
            &service.compaction_model_config(&context.model),
        )
        .expect("first compaction should commit");

    for (run_code, text) in [
        ("RUN-STMEM-SECOND-E", "second-head-E"),
        ("RUN-STMEM-SECOND-F", "second-head-F"),
        ("RUN-STMEM-SECOND-G", "second-tail-G"),
        ("RUN-STMEM-SECOND-H", "second-tail-H"),
    ] {
        context.run_code = run_code.into();
        service
            .record_user_input(&context, text)
            .expect("second window input should be recorded");
    }
    let second_input = service
        .prepare_automatic_model_compaction_input(
            &session.session_code,
            "second_auto",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            Some(8_192),
            Some(0),
            None,
            None,
            None,
        )
        .expect("second auto compaction input should prepare");

    assert!(second_input.prompt.contains("FIRST HEAD CHECKPOINT"));
    assert!(
        !second_input
            .prompt
            .contains("Previous retained recent transcript")
    );
    assert!(second_input.prompt.matches("first-tail-C").count() <= 1);
    assert!(second_input.prompt.matches("first-tail-D").count() <= 1);
    assert!(second_input.prompt.contains("second-tail-H"));
}

#[test]
fn tool_schema_tokens_participate_in_context_budget() {
    let database_url = temp_database_url("short-term-memory-tool-schema-budget");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("tool schema budget".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let config = ShortTermMemoryConfig::default();
    let service = ShortTermMemoryService::new(repository, config);
    let mut context = base_context(&session.session_code);
    context.model.context_window_tokens = Some(900);
    context.model.max_tokens = Some(0);
    context.tools = vec![EffectiveTool {
        tool_name: "large.schema".into(),
        domain_code: Some("memory".into()),
        description: "Large schema".repeat(400),
        input_schema: json!({
            "type": "object",
            "properties": {
                "payload": {
                    "type": "string",
                    "description": "x".repeat(6000)
                }
            }
        }),
        skill_name: None,
        required_variable_keys: Vec::new(),
    }];
    let input_code = service
        .record_user_input(&context, "small prompt")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let prepared = service
        .prepare_model_context(&context)
        .expect("prepared context should include tool schema budget");

    assert!(prepared.over_budget);
    assert!(!prepared.compaction_required);
    assert!(prepared.estimated_tokens > 300);
    assert_eq!(
        prepared.over_budget_reason.as_deref(),
        Some("TOOL_SCHEMA_OVER_BUDGET")
    );
    let breakdown = prepared
        .budget_breakdown
        .as_ref()
        .expect("budget breakdown should be present");
    assert_eq!(breakdown.tool_schema_budget_tokens, Some(900));
    assert_eq!(
        breakdown.compressed_history_message_count, None,
        "schema overflow is rejected before history selection"
    );
    assert!(breakdown.tool_schema_tokens > breakdown.recent_transcript_tokens);
}

#[test]
fn model_compaction_preserves_current_user_input_after_auto_retry() {
    let database_url = temp_database_url("short-term-memory-auto-compaction");
    let session = create_test_session(&database_url, "auto compaction");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let mut config = ShortTermMemoryConfig::default();
    config
        .context
        .blocks
        .current_session_checkpoint
        .max_ratio_bps = None;
    config.context.blocks.current_session_checkpoint.max_tokens = Some(300);
    config.compaction.retained_history_ratio_bps = 2_000;

    let service = ShortTermMemoryService::new(repository, config);

    let mut old_context = base_context(&session.session_code);
    old_context.run_code = "RUN-STMEM-OLD-CONTEXT".into();
    service
        .record_user_input(
            &old_context,
            &format!("older context {}", "A".repeat(8_000)),
        )
        .expect("older input should be recorded");
    let mut context = base_context(&session.session_code);
    context.run_code = "RUN-STMEM-CURRENT-CONTEXT".into();
    let current_prompt = format!("current user input must stay raw {}", "B".repeat(200));
    let latest_input = service
        .record_user_input(&context, &current_prompt)
        .expect("latest input should be recorded");
    context.model.context_window_tokens = Some(1_800);
    context.model.max_tokens = Some(0);
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code: latest_input,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let before = service
        .prepare_model_context(&context)
        .expect("over-budget context should be detected");

    assert!(
        before.compaction_required,
        "initial full context should request model compaction before retry"
    );
    assert!(
        before.checkpoint_code.is_none(),
        "prepare should not write a deterministic checkpoint"
    );
    let compaction_input = service
        .prepare_automatic_model_compaction_input(
            &session.session_code,
            "auto_compaction_trigger",
            tron_backend::application::short_term_memory::CompactionContextPolicy {
                todo_enabled: false,
            },
            context.model.context_window_tokens,
            context.model.max_tokens,
            Some(&context.run_code),
            before.compaction_tail_start_seq,
            before.effective_retained_history_tokens,
        )
        .expect("auto compaction input should prepare");
    assert!(
        compaction_input
            .pinned_start_seq
            .is_some_and(|pinned_seq| compaction_input.source_end_seq < pinned_seq),
        "current run transcript should remain outside checkpoint coverage"
    );
    let model = service.compaction_model_config(&context.model);
    service
        .commit_model_compaction(
            compaction_input,
            ModelCompactionOutput {
                summary:
                    "## Goal\nMODEL SUMMARY UNIQUE\n\n## Decisions\nOld context compacted by model."
                        .into(),
                token_usage: Some(ModelTokenUsage {
                    input_tokens: 100,
                    output_tokens: 20,
                    total_tokens: 120,
                    cached_input_tokens: None,
                    reasoning_tokens: None,
                }),
                provider_metadata: None,
            },
            &model,
        )
        .expect("model compaction should commit");
    let prepared = service
        .prepare_model_context(&context)
        .expect("retry after model compaction should prepare context");

    assert!(
        !prepared.over_budget,
        "retry after model compaction should fit the budget"
    );
    assert!(
        prepared.checkpoint_code.is_some(),
        "model compaction should create a checkpoint"
    );
    let rendered = prepared
        .messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n");
    assert!(rendered.contains("MODEL SUMMARY UNIQUE"));
    assert!(rendered.contains(&current_prompt));
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let checkpoint_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND role = 'checkpoint'
               AND message_type = 'compaction'
               AND metadata_json LIKE '%auto_compaction_trigger%'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("checkpoint count should be queryable");
    assert_eq!(checkpoint_count, 1);
    let requested_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.context.compaction_requested'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("compaction request count should be queryable");
    assert_eq!(requested_count, 1);
    let assembled_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.context.assembled'",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("assembled count should be queryable");
    assert_eq!(assembled_count, 1);
}

#[test]
fn delete_session_removes_short_term_memory_rows() {
    let database_url = temp_database_url("short-term-memory-delete-session");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("delete stm".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let service = ShortTermMemoryService::new(repository, ShortTermMemoryConfig::default());
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "delete this transcript")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });
    service
        .record_run_started(&context)
        .expect("run lease should be recorded");
    service
        .replace_todos(
            &context,
            vec![TodoUpdateItem {
                todo_code: Some("TODO-DELETE".into()),
                position: 0,
                content: "remove me".into(),
                status: "pending".into(),
                priority: "medium".into(),
            }],
        )
        .expect("todo should be recorded");
    service
        .set_goal(SetGoalCommand {
            session_code: session.session_code.clone(),
            objective: "Delete this goal with its session".into(),
            max_runs: Some(2),
        })
        .expect("goal should be recorded");
    service
        .prepare_model_context(&context)
        .expect("context epoch should be recorded");
    service
        .record_model_response(&context, &ModelResponse::final_output("done"))
        .expect("assistant response should be recorded");

    chats
        .delete_session(&session.session_code)
        .expect("session should delete");
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    for table in [
        "agent_transcript_inputs",
        "agent_transcript_events",
        "agent_transcript_messages",
        "agent_transcript_parts",
        "agent_session_todos",
        "agent_transcript_context_epochs",
        "agent_session_goals",
        "agent_session_goal_criteria",
        "agent_session_goal_events",
        "agent_session_family_members",
        "agent_session_family_snapshots",
        "agent_session_run_leases",
    ] {
        let count: i64 = connection
            .query_row(
                &format!("SELECT count(1) FROM {table} WHERE session_code = ?1"),
                [session.session_code.as_str()],
                |row| row.get(0),
            )
            .expect("stm table should be queryable");
        assert_eq!(count, 0, "{table} should not retain deleted session rows");
    }
}

#[test]
fn event_log_payload_is_bounded_before_persistence() {
    let database_url = temp_database_url("short-term-memory-event-payload-bound");
    let session = create_test_session(&database_url, "bounded event payload");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let mut config = ShortTermMemoryConfig::default();
    config.event_log.max_payload_bytes = 512;
    let service = ShortTermMemoryService::new(repository, config);
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "payload bound")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    service
        .record_model_requested(
            &context,
            ModelRequestTranscriptFacts {
                estimated_tokens: 1,
                message_hashes: vec!["x".repeat(2_000)],
                tool_schema_token_estimate: 1,
                protocol: "chatCompletions".into(),
                previous_response_id_present: false,
                context_window_estimate: Some(32_000),
                checkpoint_code: None,
                budget_breakdown: None,
                over_budget_reason: None,
                usage_estimate: None,
            },
        )
        .expect("oversized request event should be recorded");

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let payload_json: String = connection
        .query_row(
            "SELECT payload_json
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type = 'agent.model.requested'
             ORDER BY id DESC
             LIMIT 1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("model request event should be queryable");
    assert!(payload_json.len() <= 512);
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).expect("payload should be JSON");
    assert_eq!(
        payload.get("truncated").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert!(
        payload
            .get("originalBytes")
            .and_then(|value| value.as_u64())
            .unwrap_or_default()
            > 512
    );
    assert!(
        payload
            .get("sha256")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .starts_with("sha256:")
    );
    assert!(
        payload
            .get("preview")
            .and_then(|value| value.as_str())
            .is_some()
    );
}

#[test]
fn lifecycle_only_event_log_keeps_projections_and_run_events() {
    let database_url = temp_database_url("short-term-memory-event-lifecycle-only");
    let session = create_test_session(&database_url, "lifecycle only event log");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let mut config = ShortTermMemoryConfig::default();
    config.event_log.mode = "lifecycleOnly".into();
    let service = ShortTermMemoryService::new(repository, config);
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "lifecycle only input")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: true,
    });

    service
        .record_run_started(&context)
        .expect("run start should be recorded");
    service
        .replace_todos(
            &context,
            vec![TodoUpdateItem {
                todo_code: Some("TODO-LIFE".into()),
                position: 0,
                content: "Projection should survive".into(),
                status: "in_progress".into(),
                priority: "medium".into(),
            }],
        )
        .expect("todo projection should be recorded without audit event");
    service
        .prepare_model_context(&context)
        .expect("context epoch projection should be recorded without audit event");
    service
        .record_model_requested(
            &context,
            ModelRequestTranscriptFacts {
                estimated_tokens: 1,
                message_hashes: vec!["hash".into()],
                tool_schema_token_estimate: 1,
                protocol: "chatCompletions".into(),
                previous_response_id_present: false,
                context_window_estimate: Some(32_000),
                checkpoint_code: None,
                budget_breakdown: None,
                over_budget_reason: None,
                usage_estimate: None,
            },
        )
        .expect("ordinary audit event should be skipped");
    service
        .record_model_response(&context, &ModelResponse::final_output("projection ok"))
        .expect("assistant projection should be recorded");
    service
        .record_run_completed(&context)
        .expect("run completion should be recorded");
    create_legacy_full_source_stub_compaction(
        &service,
        &database_url,
        CreateCompactionCommand {
            session_code: session.session_code.clone(),
            reason: Some("lifecycle only".into()),
            note: None,
            todo_enabled: None,
        },
        "## Goal\nProjection should survive\n\n## Current State\nLifecycle mode keeps checkpoint rows.",
    );

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let run_events: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type IN ('agent.run.started', 'agent.run.completed')",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("run events should be queryable");
    assert_eq!(run_events, 2);
    let ordinary_events: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE session_code = ?1
               AND event_type NOT IN ('agent.run.started', 'agent.run.completed', 'agent.run.failed', 'agent.run.recovered')",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("ordinary events should be queryable");
    assert_eq!(ordinary_events, 0);
    let message_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_messages
             WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("messages should be queryable");
    assert!(message_count >= 3);
    let todo_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_session_todos
             WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("todos should be queryable");
    assert_eq!(todo_count, 1);
    let epoch_count: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_context_epochs
             WHERE session_code = ?1",
            [session.session_code.as_str()],
            |row| row.get(0),
        )
        .expect("context epochs should be queryable");
    assert!(epoch_count >= 1);
}

#[test]
fn event_log_cleanup_removes_old_closed_events_in_bounded_batches() {
    let database_url = temp_database_url("short-term-memory-event-cleanup");
    let session = create_test_session(&database_url, "event cleanup");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let mut config = ShortTermMemoryConfig::default();
    config.event_log.retention_days = 5;
    config.event_log.cleanup_batch_size = 2;
    config.event_log.cleanup_max_rows_per_startup = 3;
    let service = ShortTermMemoryService::new(repository, config);
    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let old_ms = 1_000_i64;
    let new_ms = chrono::Utc::now().timestamp_millis();
    for id in 1..=5_i64 {
        connection
            .execute(
                "INSERT INTO agent_transcript_events
                 (id, event_code, session_code, workspace_code, seq, run_code, input_code, message_code, part_code, event_type, event_source, payload_json, metadata_json, occurred_at, created_at)
                 VALUES (?1, ?2, ?3, 'TRON_CORE', ?4, NULL, NULL, NULL, NULL, 'agent.model.requested', 'TEST', '{}', NULL, ?5, ?5)",
                rusqlite::params![
                    id,
                    format!("ATE-OLD-{id}"),
                    session.session_code,
                    id,
                    old_ms,
                ],
            )
            .expect("old event should insert");
    }
    connection
        .execute(
            "INSERT INTO agent_transcript_events
             (id, event_code, session_code, workspace_code, seq, run_code, input_code, message_code, part_code, event_type, event_source, payload_json, metadata_json, occurred_at, created_at)
             VALUES (10, 'ATE-NEW', ?1, 'TRON_CORE', 10, NULL, NULL, NULL, NULL, 'agent.model.requested', 'TEST', '{}', NULL, ?2, ?2)",
            rusqlite::params![session.session_code, new_ms],
        )
        .expect("new event should insert");
    connection
        .execute(
            "INSERT INTO agent_transcript_events
             (id, event_code, session_code, workspace_code, seq, run_code, input_code, message_code, part_code, event_type, event_source, payload_json, metadata_json, occurred_at, created_at)
             VALUES (20, 'ATE-ACTIVE', ?1, 'TRON_CORE', 20, 'RUN-ACTIVE', NULL, NULL, NULL, 'agent.run.started', 'TEST', '{}', NULL, ?2, ?2)",
            rusqlite::params![session.session_code, old_ms],
        )
        .expect("active run event should insert");
    connection
        .execute(
            "INSERT INTO agent_session_run_leases
             (id, session_code, workspace_code, run_code, input_code, started_at, updated_at)
             VALUES (30, ?1, 'TRON_CORE', 'RUN-ACTIVE', NULL, ?2, ?2)",
            rusqlite::params![session.session_code, old_ms],
        )
        .expect("active run lease should insert");

    let deleted = service
        .cleanup_event_log_on_startup()
        .expect("event cleanup should run");
    assert_eq!(deleted, 3);
    let old_remaining: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE event_code LIKE 'ATE-OLD-%'",
            [],
            |row| row.get(0),
        )
        .expect("old events should be queryable");
    assert_eq!(old_remaining, 2);
    let new_remaining: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE event_code = 'ATE-NEW'",
            [],
            |row| row.get(0),
        )
        .expect("new event should be queryable");
    assert_eq!(new_remaining, 1);
    let active_remaining: i64 = connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_events
             WHERE event_code = 'ATE-ACTIVE'",
            [],
            |row| row.get(0),
        )
        .expect("active event should be queryable");
    assert_eq!(active_remaining, 1);
}

#[test]
fn concurrent_append_allocates_unique_session_sequence_values() {
    let database_url = temp_database_url("short-term-memory-concurrent-seq");
    let chats = ChatService::new(
        SqliteChatRepository::connect(&database_url).expect("chat repository should connect"),
    );
    let session = chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some("concurrent seq".into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created");
    let repository = SqliteShortTermMemoryRepository::connect(&database_url)
        .expect("short-term memory repository should connect");
    let service = ShortTermMemoryService::new(repository, ShortTermMemoryConfig::default());
    let mut context = base_context(&session.session_code);
    let input_code = service
        .record_user_input(&context, "parallel append")
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session.session_code.clone(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });

    let handles = (0..8)
        .map(|index| {
            let service = service.clone();
            let mut context = context.clone();
            context.run_code = format!("RUN-CONCURRENT-{index}");
            std::thread::spawn(move || {
                service
                    .record_model_response(
                        &context,
                        &ModelResponse::final_output(format!("parallel output {index}")),
                    )
                    .expect("parallel model response should be recorded");
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().expect("thread should finish");
    }

    let connection =
        rusqlite::Connection::open(&database_url).expect("database should be queryable");
    let (row_count, distinct_seq_count): (i64, i64) = connection
        .query_row(
            "SELECT count(1), count(DISTINCT seq)
             FROM (
                 SELECT seq FROM agent_transcript_messages WHERE session_code = ?1
                 UNION ALL
                 SELECT seq FROM agent_transcript_events WHERE session_code = ?1
             )",
            [session.session_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("transcript seq values should be queryable");
    assert_eq!(row_count, distinct_seq_count);
}

fn base_context(session_code: &str) -> AgentRunContext {
    AgentRunContext {
        conversation_code: Some(session_code.into()),
        run_code: "RUN-STMEM-1".into(),
        workspace_code: "TRON_CORE".into(),
        user_code: Some("USER-1".into()),
        locale: "en-US".into(),
        request_id: Some("REQ-STMEM-1".into()),
        stream_key: Some(session_code.into()),
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
        variables: Vec::<EffectiveVariable>::new(),
        skills: Vec::<EffectiveSkill>::new(),
        tools: vec![EffectiveTool {
            tool_name: "todo_update".into(),
            domain_code: Some("memory".into()),
            description: "Update session todos".into(),
            input_schema: json!({}),
            skill_name: None,
            required_variable_keys: Vec::new(),
        }],
        effective_tools: Vec::new(),
        tool_exposure: Default::default(),
        execution_scope: AgentExecutionScope {
            applications: Vec::new(),
            project_code: None,
            app_code: None,
            repository_url: None,
            local_directory: None,
            branch_name: None,
            task_code: None,
            allowed_local_directories: Vec::new(),
        },
        limits: AgentRunLimits::default(),
        messages: vec![AgentMessage::user("Track the implementation state")],
        observations: Vec::new(),
        short_term_memory: None,
        started_at: "2026-06-15T00:00:00Z".into(),
        auto_model_route: None,
    }
}

fn session_summary_context(context: &AgentRunContext) -> SessionSummaryJobContext {
    SessionSummaryJobContext {
        session_code: context
            .short_term_memory
            .as_ref()
            .map(|memory| memory.session_code.clone())
            .or_else(|| context.conversation_code.clone())
            .expect("summary test context should have a session"),
        workspace_code: context.workspace_code.clone(),
        run_code: context.run_code.clone(),
        locale: context.locale.clone(),
        model: context.model.clone(),
    }
}

fn record_summary_run(
    service: &ShortTermMemoryService,
    session_code: &str,
    run_code: &str,
    user_text: &str,
    assistant_text: &str,
) -> AgentRunContext {
    let mut context = base_context(session_code);
    context.run_code = run_code.into();
    let input_code = service
        .record_user_input(&context, user_text)
        .expect("summary test input should be recorded");
    attach_short_term_memory(
        &mut context,
        session_code,
        input_code,
        vec![AgentMessage::system("summary test system seed")],
    );
    service
        .record_model_response(&context, &ModelResponse::final_output(assistant_text))
        .expect("summary test assistant output should be recorded");
    context
}

fn completed_summary_outcome() -> SessionSummaryRunOutcome {
    SessionSummaryRunOutcome {
        status: "COMPLETED".into(),
        error_code: None,
        error_message: None,
    }
}

fn create_stub_session_summary(
    service: &ShortTermMemoryService,
    context: &AgentRunContext,
    markdown: &str,
) {
    let job = service
        .prepare_session_summary_job(
            session_summary_context(context),
            completed_summary_outcome(),
        )
        .expect("session summary job should prepare")
        .expect("session summary should not already exist");
    let input = service
        .session_summary_model_request(job, "Summarize the evidence as Markdown.".into(), 1)
        .expect("session summary input should prepare")
        .expect("session summary should not be superseded");
    service
        .commit_session_summary(
            &input,
            ModelSessionSummaryOutput {
                markdown: markdown.into(),
                token_usage: None,
                provider_metadata: None,
            },
            &service.session_summary_model_config(&context.model),
        )
        .expect("session summary should commit");
}

fn attach_short_term_memory(
    context: &mut AgentRunContext,
    session_code: &str,
    input_code: String,
    prompt_prefix: Vec<AgentMessage>,
) {
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session_code.to_string(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix,
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
}

fn rendered_messages(messages: &[AgentMessage]) -> String {
    messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n")
}

fn context_window_for_estimate_ratio(estimated_tokens: u32, ratio_bps: u32) -> u32 {
    estimated_tokens
        .saturating_mul(10_000)
        .saturating_add(ratio_bps.saturating_sub(1))
        / ratio_bps.max(1)
}

fn create_stub_model_compaction(
    service: &ShortTermMemoryService,
    command: CreateCompactionCommand,
    summary: &str,
) -> ShortTermMemoryCompaction {
    let base_model = compaction_test_model();
    let model = service.compaction_model_config(&base_model);
    let input = service
        .prepare_manual_model_compaction_input(command, Some(8_192), Some(0))
        .expect("model compaction input should prepare");
    service
        .commit_model_compaction(
            input,
            ModelCompactionOutput {
                summary: summary.to_string(),
                token_usage: Some(ModelTokenUsage {
                    input_tokens: 42,
                    output_tokens: 12,
                    total_tokens: 54,
                    cached_input_tokens: None,
                    reasoning_tokens: None,
                }),
                provider_metadata: None,
            },
            &model,
        )
        .expect("model compaction should commit")
}

fn create_legacy_full_source_stub_compaction(
    service: &ShortTermMemoryService,
    database_url: &str,
    command: CreateCompactionCommand,
    summary: &str,
) -> ShortTermMemoryCompaction {
    let session_code = command.session_code.trim().to_string();
    let workspace_code = service
        .session_workspace_for_compaction(&session_code)
        .expect("session workspace should load");
    let connection =
        rusqlite::Connection::open(database_url).expect("database should be queryable");
    let (source_start_seq, source_end_seq, source_message_count): (i64, i64, i64) = connection
        .query_row(
            "SELECT COALESCE(min(seq), 0), COALESCE(max(seq), 0), count(1)
             FROM agent_transcript_messages
             WHERE session_code = ?1
               AND status = 'completed'
               AND message_type <> 'compaction'",
            [session_code.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("transcript message range should be queryable");
    let previous_checkpoint = service
        .latest_compaction(&session_code)
        .expect("latest checkpoint should load");
    let recent = {
        let mut statement = connection
            .prepare(
                "SELECT COALESCE(p.content_text, '')
                 FROM agent_transcript_messages m
                 JOIN agent_transcript_parts p ON p.message_code = m.message_code
                 WHERE m.session_code = ?1
                   AND m.status = 'completed'
                   AND m.message_type <> 'compaction'
                   AND m.seq BETWEEN ?2 AND ?3
                   AND p.visibility = 'model'
                 ORDER BY m.seq, p.seq",
            )
            .expect("legacy recent transcript statement should prepare");
        let fragments = statement
            .query_map(
                rusqlite::params![session_code.as_str(), source_start_seq, source_end_seq],
                |row| row.get::<_, String>(0),
            )
            .expect("legacy recent transcript should query")
            .collect::<Result<Vec<_>, _>>()
            .expect("legacy recent transcript should map")
            .into_iter()
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>();
        if fragments.is_empty() {
            "Legacy retained recent transcript".into()
        } else {
            fragments.join("\n")
        }
    };
    let model = service.compaction_model_config(&compaction_test_model());
    let input = PreparedCompactionInput {
        session_code,
        workspace_code,
        run_code: None,
        reason: command.reason,
        note: command.note,
        todo_enabled: command.todo_enabled.unwrap_or(true),
        enforce_idle: true,
        strategy: "MANUAL_CHECKPOINT".into(),
        triggered_by: "user".into(),
        source: "chat_page".into(),
        previous_checkpoint_message_code: previous_checkpoint
            .as_ref()
            .map(|item| item.checkpoint_message_code.clone()),
        previous_checkpoint_included_recent: previous_checkpoint.is_some(),
        coverage: "full_source_includes_tail".into(),
        selector_version: 2,
        selection_source_end_seq: source_end_seq,
        trigger_ratio_bps: 8_000,
        retained_history_ratio_bps: 2_000,
        effective_retained_history_tokens: 0,
        source_start_seq,
        source_end_seq,
        tail_start_seq: source_start_seq,
        source_message_count: source_message_count as usize,
        selection_source_message_count: source_message_count as usize,
        source_token_estimate: 0,
        tail_token_estimate: 0,
        compaction_input_token_estimate: 0,
        max_input_tokens: 8_192,
        recent,
        prompt: "legacy full-source checkpoint test input".into(),
        prompt_hash: "legacy-full-source-stub".into(),
        pinned_start_seq: None,
        history: vec![],
        previous_summary: None,
    };
    service
        .commit_model_compaction(
            input,
            ModelCompactionOutput {
                summary: summary.to_string(),
                token_usage: None,
                provider_metadata: None,
            },
            &model,
        )
        .expect("legacy full-source compaction should commit")
}

fn compaction_test_model() -> AgentModelConfig {
    AgentModelConfig {
        model_code: "MODEL-COMPACTION".into(),
        protocol: AgentModelProtocol::ChatCompletions,
        base_url: "http://127.0.0.1:1/v1".into(),
        api_path: None,
        api_key: "sk-test".into(),
        model_name: "gpt-compaction-test".into(),
        thinking_enabled: false,
        reasoning_effort: None,
        responses_stateful_enabled: false,
        temperature: Some(0.2),
        max_tokens: Some(4_096),
        context_window_tokens: Some(200_000),
        top_p: Some(1.0),
        max_retries: Some(0),
    }
}

fn table_columns(connection: &rusqlite::Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("pragma should prepare");
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("pragma should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("columns should map")
}

fn table_exists(connection: &rusqlite::Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT count(1) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .expect("table metadata should be queryable")
        > 0
}

fn index_exists(connection: &rusqlite::Connection, index: &str) -> bool {
    connection
        .query_row(
            "SELECT count(1) FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [index],
            |row| row.get::<_, i64>(0),
        )
        .expect("index metadata should be queryable")
        > 0
}

fn create_test_session(database_url: &str, title: &str) -> ChatSession {
    let chats = ChatService::new(
        SqliteChatRepository::connect(database_url).expect("chat repository should connect"),
    );
    chats
        .create_session(
            CreateChatSessionCommand {
                workspace_code: Some("TRON_CORE".into()),
                project_code: None,
                title: Some(title.into()),
                app_code: None,
                task_code: None,
                source_type: None,
                source_code: None,
                agent_code: Some("main-agent".into()),
            },
            Some("USER-1".into()),
        )
        .expect("session should be created")
}

fn reasoning_response() -> ModelResponse {
    ModelResponse::from_assistant(
        AgentMessage::new(
            "assistant",
            vec![
                AgentMessagePart::ReasoningSummary {
                    text: "summary reasoning".into(),
                },
                AgentMessagePart::Reasoning {
                    text: "raw reasoning should be policy gated".into(),
                },
                AgentMessagePart::Text {
                    text: "visible answer".into(),
                },
            ],
        ),
        ModelFinish::Final,
        None,
        None,
        None,
    )
}

fn reasoning_tool_response() -> ModelResponse {
    ModelResponse::from_assistant(
        AgentMessage::new(
            "assistant",
            vec![
                AgentMessagePart::ReasoningSummary {
                    text: "summary reasoning".into(),
                },
                AgentMessagePart::Text {
                    text: "visible answer before tool".into(),
                },
                AgentMessagePart::ToolCall {
                    call_id: Some("call-reasoning-summary".into()),
                    tool_name: "shell.exec".into(),
                    input: json!({"cmd": "printf replay"}),
                    input_error: None,
                },
            ],
        ),
        ModelFinish::ToolCalls,
        None,
        None,
        None,
    )
}

fn record_reasoning_tool_turn(
    service: &ShortTermMemoryService,
    context: &mut AgentRunContext,
    session_code: &str,
    user_text: &str,
) {
    let input_code = service
        .record_user_input(context, user_text)
        .expect("user input should be recorded");
    context.short_term_memory = Some(AgentShortTermMemoryRunContext {
        session_code: session_code.to_string(),
        input_code,
        input_kind: "USER_PROMPT".into(),
        input_metadata: None,
        prompt_prefix_sources: vec![],
        prompt_prefix: vec![AgentMessage::system("system seed")],
        current_input_message: None,
        agent_code: Some("main-agent".into()),
        todo_enabled: false,
    });
    service
        .record_model_response(context, &reasoning_tool_response())
        .expect("reasoning tool response should be recorded");
    service
        .record_tool_result(
            context,
            &ToolObservation {
                tool_name: "shell.exec".into(),
                output: "tool result visible to model".into(),
            },
            Some("call-reasoning-summary".into()),
            1,
            ToolTranscriptStatus::Completed,
        )
        .expect("tool result should be recorded");
}

fn rendered_model_context(messages: &[AgentMessage]) -> String {
    messages
        .iter()
        .map(AgentMessage::text_content)
        .collect::<Vec<_>>()
        .join("\n---\n")
}

fn part_type_count(connection: &rusqlite::Connection, session_code: &str, part_type: &str) -> i64 {
    connection
        .query_row(
            "SELECT count(1)
             FROM agent_transcript_parts
             WHERE session_code = ?1
               AND part_type = ?2",
            [session_code, part_type],
            |row| row.get(0),
        )
        .expect("part type count should be queryable")
}

#[cfg(unix)]
fn mark_file_old(path: &Path) {
    let status = Command::new("touch")
        .arg("-t")
        .arg("197001010000.00")
        .arg(path)
        .status()
        .expect("touch should be available for unix mtime setup");
    assert!(status.success());
}

fn temp_database_url(label: &str) -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();
    std::env::temp_dir()
        .join(format!("{label}-{}-{millis}.sqlite3", std::process::id()))
        .display()
        .to_string()
}

fn temp_artifact_root(label: &str) -> std::path::PathBuf {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis();
    std::env::temp_dir().join(format!("{label}-{}-{millis}", std::process::id()))
}

#[test]
fn memory_projection_cas_rejects_two_connection_late_results_and_preserves_complete_history() {
    for scenario in ["checkpoint", "append", "scope"] {
        let database_url = temp_database_url(&format!("memory-cas-{scenario}"));
        let session = create_test_session(&database_url, "memory CAS");
        let mut config = ShortTermMemoryConfig::default();
        config.compaction.retained_history_ratio_bps = 1;
        let first = ShortTermMemoryService::new(
            SqliteShortTermMemoryRepository::connect(&database_url).unwrap(),
            config.clone(),
        );
        let second = ShortTermMemoryService::new(
            SqliteShortTermMemoryRepository::connect(&database_url).unwrap(),
            config,
        );
        let mut context = base_context(&session.session_code);
        context.run_code = "older-memory-turn".into();
        first
            .record_user_input(&context, &"older complete turn ".repeat(200))
            .unwrap();
        context.run_code = "latest-memory-turn".into();
        first
            .record_user_input(&context, "latest complete user turn")
            .unwrap();
        let command = || CreateCompactionCommand {
            session_code: session.session_code.clone(),
            reason: Some("CAS test".into()),
            note: Some("Preserve identifier CASE-42".into()),
            todo_enabled: Some(false),
        };
        let mut input = first
            .prepare_manual_model_compaction_input(command(), Some(8_192), Some(512))
            .unwrap();
        assert_eq!(input.coverage, "head_only_tail_replayed");
        assert!(
            input
                .history
                .first()
                .unwrap()
                .contains("Preserve identifier CASE-42")
        );
        assert!(
            input
                .history
                .first()
                .unwrap()
                .contains("latest complete user turn")
        );
        assert!(input.selection_source_end_seq > input.source_end_seq);
        let connection = rusqlite::Connection::open(&database_url).unwrap();
        let before:i64=connection.query_row("SELECT count(1) FROM agent_transcript_messages WHERE session_code=?1 AND message_type!='compaction'",[&session.session_code],|r|r.get(0)).unwrap();
        let model = first.compaction_model_config(&compaction_test_model());
        let output = || ModelCompactionOutput {
            summary: "SDK validated summary".into(),
            token_usage: None,
            provider_metadata: None,
        };
        if scenario == "checkpoint" {
            let newer = second
                .prepare_manual_model_compaction_input(command(), Some(8_192), Some(512))
                .unwrap();
            second
                .commit_model_compaction(newer, output(), &model)
                .unwrap();
        }
        if scenario == "append" {
            context.run_code = "appended-during-compression".into();
            second
                .record_user_input(&context, "new history must remain readable")
                .unwrap();
        }
        if scenario == "scope" {
            input.workspace_code = "another-user-workspace".into();
        }
        assert!(matches!(
            first.commit_model_compaction(input, output(), &model),
            Err(ShortTermMemoryError::CompactionConflict)
        ));
        let after:i64=connection.query_row("SELECT count(1) FROM agent_transcript_messages WHERE session_code=?1 AND message_type!='compaction'",[&session.session_code],|r|r.get(0)).unwrap();
        assert_eq!(after, before + if scenario == "append" { 1 } else { 0 });
        let checkpoints:i64=connection.query_row("SELECT count(1) FROM agent_transcript_messages WHERE session_code=?1 AND message_type='compaction'",[&session.session_code],|r|r.get(0)).unwrap();
        assert_eq!(checkpoints, if scenario == "checkpoint" { 1 } else { 0 });
        if scenario == "checkpoint" {
            assert!(
                second
                    .latest_compaction(&session.session_code)
                    .unwrap()
                    .is_some()
            );
        }
        let mut repository = SqliteShortTermMemoryRepository::connect(&database_url).unwrap();
        let rows = repository
            .list_messages_after_seq(&session.session_code, 0)
            .unwrap();
        assert!(
            rows.iter()
                .flat_map(|(_, parts)| parts)
                .any(|part| part.content_text.as_deref() == Some("latest complete user turn"))
        );
    }
}

#[test]
fn compaction_unknown_provider_usage_persists_unavailable_audit_without_ledger_estimates() {
    for reported in [false, true] {
        let database_url = temp_database_url("memory-usage-audit");
        let session = create_test_session(&database_url, "memory usage audit");
        let mut config = ShortTermMemoryConfig::default();
        config.compaction.retained_history_ratio_bps = 1;
        let service = ShortTermMemoryService::new(
            SqliteShortTermMemoryRepository::connect(&database_url).unwrap(),
            config,
        );
        let mut context = base_context(&session.session_code);
        context.run_code = "older-usage-turn".into();
        service
            .record_user_input(&context, &"authorized historical facts ".repeat(100))
            .unwrap();
        context.run_code = "latest-usage-turn".into();
        service
            .record_user_input(&context, "latest user turn replayed separately")
            .unwrap();
        let input = service
            .prepare_manual_model_compaction_input(
                CreateCompactionCommand {
                    session_code: session.session_code.clone(),
                    reason: None,
                    note: None,
                    todo_enabled: Some(false),
                },
                Some(8192),
                Some(0),
            )
            .unwrap();
        let result = eliteforge_agent_memory::MemoryCompactionResult {
            summary: "SDK summary".into(),
            usage: eliteforge_agent_framework::Usage {
                input_tokens: 8000,
                output_tokens: 1000,
            },
            provider_metrics: reported.then_some(eliteforge_agent_framework::ProviderMetrics {
                cached_input_tokens: Some(16),
                reasoning_tokens: Some(2),
            }),
            provider_usage: reported.then_some(eliteforge_agent_framework::Usage {
                input_tokens: 24,
                output_tokens: 12,
            }),
        };
        let result = eliteforge_agent_memory::MemoryCompactionResult::from_chat_response(
            result.into_chat_response(),
        )
        .unwrap();
        let output = ModelCompactionOutput {
            summary: result.summary.clone(),
            token_usage: result.provider_usage.map(|usage| ModelTokenUsage {
                input_tokens: usage.input_tokens as u32,
                output_tokens: usage.output_tokens as u32,
                total_tokens: (usage.input_tokens + usage.output_tokens) as u32,
                cached_input_tokens: result
                    .provider_metrics
                    .as_ref()
                    .and_then(|m| m.cached_input_tokens)
                    .map(|v| v as u32),
                reasoning_tokens: result
                    .provider_metrics
                    .as_ref()
                    .and_then(|m| m.reasoning_tokens)
                    .map(|v| v as u32),
            }),
            provider_metadata: None,
        };
        let model = service.compaction_model_config(&compaction_test_model());
        let checkpoint = service
            .commit_model_compaction(input, output, &model)
            .unwrap();
        let connection = rusqlite::Connection::open(&database_url).unwrap();
        let (input_tokens, output_tokens, metadata_json): (Option<u32>, Option<u32>, String) = connection.query_row(
            "SELECT input_tokens,output_tokens,metadata_json FROM agent_transcript_messages WHERE message_code=?1",
            [&checkpoint.checkpoint_message_code], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(input_tokens, reported.then_some(24));
        assert_eq!(output_tokens, reported.then_some(12));
        let metadata: serde_json::Value = serde_json::from_str(&metadata_json).unwrap();
        let audit = &metadata["usageAudit"];
        assert_eq!(
            audit["usageSource"],
            if reported { "provider" } else { "unavailable" }
        );
        assert_eq!(
            audit["usageConfidence"],
            if reported { "actual" } else { "unavailable" }
        );
        assert_eq!(metadata["tokenUsage"].is_null(), !reported);
        assert_eq!(audit["providerUsage"].is_null(), !reported);
        if reported {
            assert_eq!(audit["providerUsage"]["inputTokens"], 24);
            assert_eq!(audit["providerUsage"]["cachedInputTokens"], 16);
            assert_eq!(audit["providerUsage"]["reasoningTokens"], 2);
        }
        drop(connection);
        drop(service);
        std::fs::remove_file(&database_url).unwrap();
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{database_url}{suffix}"));
        }
    }
}

#[path = "support/short_term_memory_pause_lease_tests.rs"]
mod pause_lease_tests;
