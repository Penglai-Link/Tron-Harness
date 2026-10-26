use super::*;
use crate::{
    adapters::outbound::persistence::SqliteAgentEventReplayStore,
    application::agent::{AgentAguiReplayStore, AgentAguiStreamService},
    shared::event_bus::{AguiEventSink, EventBusConfig},
};
use eliteforge_agent_framework::{ChatModelPlugin, StaticToolProvider};
use eliteforge_agent_official_plugin_agui::{AgUiPlugin, AgUiStream};
use eliteforge_agent_official_plugin_storage_local::{
    LocalStoragePlugin, SqliteOptions, SqliteStateStore,
};
use eliteforge_agent_official_plugin_tools_discovery::ToolDiscoveryPlugin;
use eliteforge_agent_test_support::batch_outcomes::*;

#[tokio::test]
async fn failed_approval_processing_closes_pending_and_allows_next_host_turn() {
    use fw::{StateStore, InteractionStore};
    let host = FrameworkHost::temporary(None).unwrap();
    let path = host.0.path.clone();
    prepare_fixture_artifacts(&path);
    let model = BatchRecoveryModel::default();
    let agent = TestBuilder::new()
        .plugin(ChatModelPlugin::new(model.clone()))
        .plugin(AgUiPlugin::default())
        .plugin(fw::ToolPolicyPlugin::new("mixed-approval", MixedBatchApproval))
        .plugin(ToolDiscoveryPlugin::new().provider(StaticToolProvider::new(vec![BatchOutcomeScenario::MixedApproval.binding()])))
        .plugin(LocalStoragePlugin::new(&path, path.with_extension("artifacts"))
            .with_database(host.shared_database().await.unwrap()))
        .build().await.unwrap();
    assert!(host.0.framework.set(agent.framework()).is_ok());
    assert!(host.0.agent.set(agent.clone()).is_ok());
    let replay = Arc::new(SqliteAgentEventReplayStore::new().unwrap());
    replay.bind_framework(&host).unwrap();
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())), replay.clone());
    host.bind_presentation(service.clone()).unwrap();
    let mut context = test_context(0);
    context.conversation_code = Some("answer-failure".into());
    context.stream_key = Some("answer-failure".into());
    context.run_code = fw::RunId::new_v4().to_string();
    let registration = host.reserve_native(&context.run_code).unwrap();
    let (handle, stream) = agent.start_with_output::<AgUiStream>(fw::RunRequest::new("answer-failure", "go")).unwrap();
    let paused = within(host.wait_native(agent.clone(), handle, stream, &context, registration)).await.unwrap();
    let entry = crate::application::chat::ChatResumeEntry {
        interrupt_id: paused.interactions[0].id.to_string(), status:"resolved".into(), payload:json!(false),
    };
    host.validate_native_resume("answer-failure", std::slice::from_ref(&entry)).await.unwrap();
    let mut request = fw::RunRequest::new("answer-failure", "");
    host_interactions::apply_answers(&agent, "answer-failure", &[entry], &mut request).await.unwrap();
    let processor = FailSecondResult::default();
    request.extensions.insert(fw::ToolResultProcessorService(Arc::new(processor.clone())));
    context.run_code = fw::RunId::new_v4().to_string();
    let registration = host.reserve_native(&context.run_code).unwrap();
    let (handle, stream) = agent.start_with_output::<AgUiStream>(request).unwrap();
    assert!(within(host.wait_native(agent.clone(), handle, stream, &context, registration)).await.unwrap_err()
        .to_string().contains("host audit unavailable"));
    assert_eq!(*processor.0.lock().unwrap(), vec![1]);
    let reopened = SqliteStateStore::open(&path, SqliteOptions::default()).await.unwrap();
    let cp = reopened.latest("answer-failure").await.unwrap().unwrap();
    assert_eq!(cp.status, fw::ExecutionStatus::Failed);
    assert!(reopened.pending("answer-failure").await.unwrap().is_empty());
    assert!(reopened.get("answer-failure", paused.interactions[1].id).await.unwrap().response.is_none());
    let history = agent.framework().history().unwrap().read_session("answer-failure", fw::HistoryQuery::default()).await.unwrap();
    let public = serde_json::to_value(crate::adapters::inbound::http::chats::history::ReadHistoryResponse::from(history)).unwrap();
    assert_eq!(public["pendingInteractions"], json!([]));
    let events = service.protocol_events("answer-failure").unwrap();
    let finals:Vec<_> = events.iter().filter(|e|e.sse_event == "TOOL_CALL_RESULT").collect();
    assert_eq!(finals.len(), 1);
    let live:serde_json::Value = serde_json::from_str(finals[0].data["content"].as_str().unwrap()).unwrap();
    assert_eq!(batch_statuses(&live), ["Failed", "Failed", "NotExecuted"]);
    let parent = cp.history.tool_invocations.iter().find(|r|r.batch.is_none()).unwrap();
    assert_eq!(Some(&live["structured"]), parent.structured.as_ref());
    // Use the same public host admission path as a new chat message.
    host.validate_native_resume("answer-failure", &[]).await.unwrap();
    context.run_code = fw::RunId::new_v4().to_string();
    let registration = host.reserve_native(&context.run_code).unwrap();
    let (handle, stream) = agent.start_with_output::<AgUiStream>(fw::RunRequest::new("answer-failure", "continue")).unwrap();
    within(host.wait_native(agent.clone(), handle, stream, &context, registration)).await.unwrap();
    assert_eq!(recovery_model_result(&model), serde_json::from_str::<serde_json::Value>(live["content"][0]["text"].as_str().unwrap()).unwrap());
    let reopened_replay = SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
    assert_eq!(reopened_replay.replay_protocol_events("answer-failure").unwrap(), service.protocol_events("answer-failure").unwrap());
    assert!(host.close().await.is_success());
}

#[tokio::test]
async fn batch_progress_stream_matches_reopened_sqlite_after_approval() {
    use fw::StateStore;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../.agent-deps/agent-framework/contracts/batch-progress.json"
    )))
    .unwrap();
    let host = FrameworkHost::temporary(None).unwrap();
    let path = host.0.path.clone();
    prepare_fixture_artifacts(&path);
    let control = BatchProgressControl::default();
    let model = BatchRecoveryModel::default();
    let agent = TestBuilder::new()
        .plugin(ChatModelPlugin::new(model.clone()))
        .plugin(AgUiPlugin::default())
        .plugin(fw::ToolPolicyPlugin::new(
            "progress-approval",
            BatchProgressApproval,
        ))
        .plugin(
            ToolDiscoveryPlugin::new().provider(StaticToolProvider::new(vec![control.binding()])),
        )
        .plugin(
            LocalStoragePlugin::new(&path, path.with_extension("artifacts"))
                .with_database(host.shared_database().await.unwrap()),
        )
        .build()
        .await
        .unwrap();
    assert!(host.0.framework.set(agent.framework()).is_ok());
    assert!(host.0.agent.set(agent.clone()).is_ok());
    let replay = Arc::new(SqliteAgentEventReplayStore::new().unwrap());
    replay.bind_framework(&host).unwrap();
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        replay.clone(),
    );
    host.bind_presentation(service.clone()).unwrap();
    let mut context = test_context(0);
    context.conversation_code = Some("progress".into());
    context.stream_key = Some("progress".into());
    context.run_code = fw::RunId::new_v4().to_string();
    let mut request = fw::RunRequest::new("progress", "go");
    request.limits.max_parallel_tools = 2;
    let registration = host.reserve_native(&context.run_code).unwrap();
    let (handle, stream) = agent.start_with_output::<AgUiStream>(request).unwrap();
    let paused = within(host.wait_native(agent.clone(), handle, stream, &context, registration))
        .await
        .unwrap();
    let entry = crate::application::chat::ChatResumeEntry {
        interrupt_id: paused.interactions[0].id.to_string(),
        status: "resolved".into(),
        payload: json!(true),
    };
    host.validate_native_resume("progress", std::slice::from_ref(&entry))
        .await
        .unwrap();
    let mut request = fw::RunRequest::new("progress", "");
    host_interactions::apply_answers(&agent, "progress", &[entry], &mut request)
        .await
        .unwrap();
    context.run_code = fw::RunId::new_v4().to_string();
    let registration = host.reserve_native(&context.run_code).unwrap();
    let (handle, stream) = agent.start_with_output::<AgUiStream>(request).unwrap();
    let (result, ()) = within(async {
        tokio::join!(
            host.wait_native(agent.clone(), handle, stream, &context, registration),
            async {
                for (name, expected, release) in [
                    ("approvedRunning", ["Failed", "Running", "Running"], 1),
                    ("partialCompleted", ["Failed", "Completed", "Running"], 2),
                ] {
                    let mut tick = tokio::time::interval(std::time::Duration::from_millis(5));
                    let snapshot = loop {
                        tick.tick().await;
                        // Durable replay is a synchronous host API. Read it off
                        // the executor that is advancing the active SQLite run.
                        let reader = service.clone();
                        let events =
                            tokio::task::spawn_blocking(move || reader.protocol_events("progress"))
                                .await
                                .unwrap()
                                .unwrap();
                        if let Some(snapshot) = events
                            .iter()
                            .rev()
                            .filter(|event| {
                                event.sse_event == "ACTIVITY_SNAPSHOT"
                                    && event.data["activityType"] == fw::TOOL_BATCH_ACTIVITY_TYPE
                            })
                            .map(|event| event.data["content"].clone())
                            .find(|snapshot| batch_statuses(&snapshot["output"]) == expected)
                        {
                            break snapshot;
                        }
                    };
                    let reopened = SqliteStateStore::open(&path, SqliteOptions::default())
                        .await
                        .unwrap();
                    let cp = reopened.latest("progress").await.unwrap().unwrap();
                    let history = agent.framework().history().unwrap().read_session("progress", fw::HistoryQuery::default()).await.unwrap();
                    let public = serde_json::to_value(crate::adapters::inbound::http::chats::history::ReadHistoryResponse::from(history)).unwrap();
                    let parent = public["records"]["toolInvocations"].as_array().unwrap().iter()
                        .find(|row| row["id"] == snapshot["toolCallId"]).unwrap();
                    assert_eq!(parent["batchRevision"], snapshot["revision"]);
                    assert_eq!(parent["structured"], snapshot["output"]["structured"]);
                    assert_eq!(
                        committed_progress_snapshot(&cp, snapshot, &expected),
                        fixture[name]
                    );
                    control.release(release);
                }
            }
        )
    })
    .await;
    result.unwrap();
    let reopened = SqliteStateStore::open(&path, SqliteOptions::default())
        .await
        .unwrap();
    let cp = reopened.latest("progress").await.unwrap().unwrap();
    let parent = cp
        .history
        .tool_invocations
        .iter()
        .find(|r| r.batch.is_none())
        .unwrap();
    let events = service.protocol_events("progress").unwrap();
    let final_results: Vec<_> = events
        .iter()
        .filter(|event| event.sse_event == "TOOL_CALL_RESULT")
        .collect();
    assert_eq!(final_results.len(), 1);
    let live: serde_json::Value =
        serde_json::from_str(final_results[0].data["content"].as_str().unwrap()).unwrap();
    assert_eq!(Some(&live["structured"]), parent.structured.as_ref());
    let mut actual = recovery_snapshot(live, parent, None);
    actual["model"] = recovery_model_result(&model);
    assert_eq!(actual, fixture["progressCompleted"]);
    let reopened_replay = SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
    assert_eq!(
        reopened_replay.replay_protocol_events("progress").unwrap(),
        events
    );
    assert_eq!(replay.replay_failure("progress"), None);
    assert!(host.close().await.is_success());
}

#[tokio::test]
async fn batch_recovery_survives_tron_forwarding_continue_and_sqlite_reopen() {
    use fw::StateStore;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../.agent-deps/agent-framework/contracts/batch-recovery.json"
    )))
    .unwrap();
    for scenario in [
        BatchOutcomeScenario::ParallelCancellation,
        BatchOutcomeScenario::MixedApproval,
        BatchOutcomeScenario::ProcessorFailure,
    ] {
        let host = FrameworkHost::temporary(None).unwrap();
        let path = host.0.path.clone();
        prepare_fixture_artifacts(&path);
        let model = BatchRecoveryModel::default();
        let mut builder = TestBuilder::new()
            .plugin(ChatModelPlugin::new(model.clone()))
            .plugin(AgUiPlugin::default())
            .plugin(
                ToolDiscoveryPlugin::new()
                    .provider(StaticToolProvider::new(vec![scenario.binding()])),
            )
            .plugin(
                LocalStoragePlugin::new(&path, path.with_extension("artifacts"))
                    .with_database(host.shared_database().await.unwrap()),
            );
        if scenario == BatchOutcomeScenario::MixedApproval {
            builder = builder.plugin(fw::ToolPolicyPlugin::new(
                "mixed-approval",
                MixedBatchApproval,
            ));
        }
        let agent = builder.build().await.unwrap();
        assert!(host.0.framework.set(agent.framework()).is_ok());
        assert!(host.0.agent.set(agent.clone()).is_ok());
        let replay = Arc::new(SqliteAgentEventReplayStore::new().unwrap());
        replay.bind_framework(&host).unwrap();
        let service = AgentAguiStreamService::with_replay_store(
            Arc::new(AguiEventSink::new(&EventBusConfig::default())),
            replay.clone(),
        );
        host.bind_presentation(service.clone()).unwrap();
        let mut context = test_context(0);
        context.conversation_code = Some(scenario.name().into());
        context.stream_key = Some(scenario.name().into());
        let mut request = fw::RunRequest::new(scenario.name(), "run");
        request.limits.max_parallel_tools = 2;
        if scenario == BatchOutcomeScenario::ParallelCancellation {
            request
                .extensions
                .insert(fw::ToolResultProcessorService(Arc::new(
                    CancelAfterKnownResult,
                )));
        }
        let processor = FailSecondResult::default();
        if scenario == BatchOutcomeScenario::ProcessorFailure {
            request
                .extensions
                .insert(fw::ToolResultProcessorService(Arc::new(processor.clone())));
        }
        let mut pending: Vec<fw::RunId> = Vec::new();
        let phases: &[&str] = if scenario == BatchOutcomeScenario::ParallelCancellation {
            &["cancelContinue"]
        } else if scenario == BatchOutcomeScenario::ProcessorFailure {
            &["processorFailure"]
        } else {
            &["mixedWaiting", "mixedRejected", "mixedCompleted"]
        };
        for (phase, name) in phases.iter().enumerate() {
            context.run_code = fw::RunId::new_v4().to_string();
            if phase > 0 {
                request = fw::RunRequest::new(scenario.name(), "");
                // Exercise the host's public interaction validation/answer path,
                // not only the framework resume field.
                let entry = crate::application::chat::ChatResumeEntry {
                    interrupt_id: pending[phase - 1].to_string(),
                    status: "resolved".into(),
                    payload: serde_json::json!(phase == 2),
                };
                host.validate_native_resume(scenario.name(), std::slice::from_ref(&entry))
                    .await
                    .unwrap();
                host_interactions::apply_answers(&agent, scenario.name(), &[entry], &mut request)
                    .await
                    .unwrap();
            }
            let registration = host.reserve_native(&context.run_code).unwrap();
            let (handle, stream) = agent
                .start_with_output::<AgUiStream>(request.clone())
                .unwrap();
            let result =
                within(host.wait_native(agent.clone(), handle, stream, &context, registration))
                    .await;
            if scenario == BatchOutcomeScenario::ParallelCancellation {
                assert!(matches!(
                    result.unwrap_err().primary(),
                    fw::AgentError::Cancelled
                ));
            } else if scenario == BatchOutcomeScenario::ProcessorFailure {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("host audit unavailable")
                );
                assert_eq!(
                    processor
                        .0
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|&&i| i == 1)
                        .count(),
                    1
                );
            } else {
                let result = result.unwrap();
                if phase == 0 {
                    pending = result.interactions.iter().map(|i| i.id).collect();
                }
                assert_eq!(result.interactions.len(), 2 - phase);
            }
            let reopened = SqliteStateStore::open(&path, SqliteOptions::default())
                .await
                .unwrap();
            let cp = reopened.latest(scenario.name()).await.unwrap().unwrap();
            let parent = cp
                .history
                .tool_invocations
                .iter()
                .find(|r| r.batch.is_none())
                .unwrap();
            let events = service.protocol_events(scenario.name()).unwrap();
            let current: Vec<_> = events
                .iter()
                .filter(|e| e.data["runId"] == context.run_code)
                .collect();
            let snapshot = current
                .iter()
                .rev()
                .find(|e| {
                    e.sse_event == "ACTIVITY_SNAPSHOT"
                        && e.data["activityType"] == fw::TOOL_BATCH_ACTIVITY_TYPE
                })
                .map(|e| e.data["content"].clone());
            let results: Vec<_> = current
                .iter()
                .filter(|e| e.sse_event == "TOOL_CALL_RESULT")
                .collect();
            let waiting = scenario == BatchOutcomeScenario::MixedApproval && phase < 2;
            let live = if waiting {
                assert!(results.is_empty());
                let snapshot = snapshot.as_ref().expect("generic batch snapshot forwarded");
                assert_eq!(snapshot["toolCallId"], parent.id.to_string());
                snapshot["output"].clone()
            } else {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].data["toolCallId"], parent.id.to_string());
                serde_json::from_str(results[0].data["content"].as_str().unwrap()).unwrap()
            };
            assert_eq!(live["structured"], parent.structured.clone().unwrap());
            let mut actual = recovery_snapshot(live, parent, if waiting { snapshot } else { None });
            if !waiting {
                if scenario != BatchOutcomeScenario::MixedApproval {
                    assert!(cp.status.is_terminal());
                    // The freshly reopened database already holds the model pair.
                    assert_eq!(
                        cp.transcript
                            .iter()
                            .filter(|m| m.role == fw::Role::Tool)
                            .count(),
                        1
                    );
                    context.run_code = fw::RunId::new_v4().to_string();
                    let registration = host.reserve_native(&context.run_code).unwrap();
                    let (handle, stream) = agent
                        .start_with_output::<AgUiStream>(fw::RunRequest::new(
                            scenario.name(),
                            "continue",
                        ))
                        .unwrap();
                    within(host.wait_native(agent.clone(), handle, stream, &context, registration))
                        .await
                        .unwrap();
                }
                actual["model"] = recovery_model_result(&model);
            }
            assert_eq!(
                actual, fixture[*name],
                "Tron recovery contract drift: {name}"
            );
            assert_eq!(replay.replay_failure(scenario.name()), None);
            let reopened_replay =
                SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
            assert_eq!(
                reopened_replay
                    .replay_protocol_events(scenario.name())
                    .unwrap(),
                service.protocol_events(scenario.name()).unwrap()
            );
        }
        assert!(host.close().await.is_success());
    }
}

#[tokio::test]
async fn batch_outcomes_survive_tron_live_forwarding_and_sqlite_reopen() {
    use fw::StateStore;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../.agent-deps/agent-framework/contracts/batch-outcomes.json"
    )))
    .unwrap();
    for scenario in [
        BatchOutcomeScenario::ParallelCancellation,
        BatchOutcomeScenario::ExposeFailure,
        BatchOutcomeScenario::SetExposureFailure,
    ] {
        let host = FrameworkHost::temporary(None).unwrap();
        let path = host.0.path.clone();
        prepare_fixture_artifacts(&path);
        let agent = TestBuilder::new()
            .plugin(ChatModelPlugin::new(BatchOutcomeModel))
            .plugin(AgUiPlugin::default())
            .plugin(
                ToolDiscoveryPlugin::new()
                    .provider(StaticToolProvider::new(vec![scenario.binding()])),
            )
            .plugin(
                LocalStoragePlugin::new(&path, path.with_extension("artifacts"))
                    .with_database(host.shared_database().await.unwrap()),
            )
            .build()
            .await
            .unwrap();
        assert!(host.0.framework.set(agent.framework()).is_ok());
        assert!(host.0.agent.set(agent.clone()).is_ok());
        let replay = Arc::new(SqliteAgentEventReplayStore::new().unwrap());
        replay.bind_framework(&host).unwrap();
        let service = AgentAguiStreamService::with_replay_store(
            Arc::new(AguiEventSink::new(&EventBusConfig::default())),
            replay.clone(),
        );
        host.bind_presentation(service.clone()).unwrap();
        let mut context = test_context(0);
        context.run_code = fw::RunId::new_v4().to_string();
        context.conversation_code = Some(scenario.name().into());
        context.stream_key = Some(scenario.name().into());
        let mut request = fw::RunRequest::new(scenario.name(), "run");
        request.limits.max_parallel_tools = 2;
        if scenario == BatchOutcomeScenario::ParallelCancellation {
            request
                .extensions
                .insert(fw::ToolResultProcessorService(Arc::new(
                    CancelAfterKnownResult,
                )));
        }
        let registration = host.reserve_native(&context.run_code).unwrap();
        let (handle, stream) = agent.start_with_output::<AgUiStream>(request).unwrap();
        let result =
            within(host.wait_native(agent.clone(), handle, stream, &context, registration)).await;
        if scenario == BatchOutcomeScenario::ParallelCancellation {
            assert!(matches!(
                result.unwrap_err().primary(),
                fw::AgentError::Cancelled
            ));
        } else {
            result.unwrap();
        }
        assert_eq!(replay.replay_failure(scenario.name()), None);
        let live_events = service.protocol_events(scenario.name()).unwrap();
        let results: Vec<_> = live_events
            .iter()
            .filter(|e| e.sse_event == "TOOL_CALL_RESULT")
            .collect();
        assert_eq!(results.len(), 1);
        let live: serde_json::Value =
            serde_json::from_str(results[0].data["content"].as_str().unwrap()).unwrap();
        // Read through a fresh SQLite connection, rather than comparing with
        // the loop's in-memory checkpoint or the stream service's cache.
        let reopened = SqliteStateStore::open(&path, SqliteOptions::default())
            .await
            .unwrap();
        let checkpoint = reopened.latest(scenario.name()).await.unwrap().unwrap();
        let parent = checkpoint
            .history
            .tool_invocations
            .iter()
            .find(|r| r.batch.is_none())
            .unwrap();
        assert_eq!(results[0].data["toolCallId"], parent.id.to_string());
        assert_eq!(live["structured"], *parent.structured.as_ref().unwrap());
        assert_eq!(live["isError"].as_bool(), parent.is_error);
        let snapshot = outcome_snapshot(live, parent);
        assert_eq!(
            snapshot,
            fixture[scenario.name()],
            "{} host contract drift",
            scenario.name()
        );
        let replay = SqliteAgentEventReplayStore::connect(path.to_str().unwrap()).unwrap();
        assert_eq!(
            replay.replay_protocol_events(scenario.name()).unwrap(),
            live_events
        );
        assert!(host.close().await.is_success());
    }
}


#[tokio::test]
async fn two_stage_search_survives_tron_forwarding_and_sqlite_reopen() {
    use eliteforge_agent_test_support::tool_search::*;
    use fw::StateStore;
    let host = FrameworkHost::temporary(None).unwrap();
    let path = host.0.path.clone();
    prepare_fixture_artifacts(&path);
    let model = TwoStageSearchModel::default();
    let agent = TestBuilder::new()
        .plugin(ChatModelPlugin::new(model.clone()))
        .plugin(AgUiPlugin::default())
        .plugin(ToolDiscoveryPlugin::new().provider(StaticToolProvider::new(two_stage_bindings())))
        .plugin(
            LocalStoragePlugin::new(&path, path.with_extension("artifacts"))
                .with_database(host.shared_database().await.unwrap()),
        )
        .build()
        .await
        .unwrap();
    assert!(host.0.framework.set(agent.framework()).is_ok());
    assert!(host.0.agent.set(agent.clone()).is_ok());
    let replay = Arc::new(SqliteAgentEventReplayStore::new().unwrap());
    replay.bind_framework(&host).unwrap();
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        replay.clone(),
    );
    host.bind_presentation(service.clone()).unwrap();
    let mut context = test_context(0);
    context.run_code = fw::RunId::new_v4().to_string();
    context.conversation_code = Some("search-contract".into());
    context.stream_key = Some("search-contract".into());
    let registration = host.reserve_native(&context.run_code).unwrap();
    let (handle, stream) = agent
        .start_with_output::<AgUiStream>(fw::RunRequest::new("search-contract", "inspect"))
        .unwrap();
    within(host.wait_native(agent.clone(), handle, stream, &context, registration))
        .await
        .unwrap();
    assert_two_stage_requests(&model);
    let reopened = SqliteStateStore::open(&path, SqliteOptions::default())
        .await
        .unwrap();
    let cp = reopened.latest("search-contract").await.unwrap().unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../.agent-deps/agent-framework/contracts/tool-search.json"
    )))
    .unwrap();
    let mut actual = json!({});
    for event in service
        .protocol_events("search-contract")
        .unwrap()
        .iter()
        .filter(|e| e.sse_event == "TOOL_CALL_RESULT")
    {
        let parent = cp
            .history
            .tool_invocations
            .iter()
            .find(|r| r.id.to_string() == event.data["toolCallId"].as_str().unwrap())
            .unwrap();
        let live: serde_json::Value =
            serde_json::from_str(event.data["content"].as_str().unwrap()).unwrap();
        assert_eq!(Some(&live["structured"]), parent.structured.as_ref());
        actual[parent.call_id.clone()] = outcome_snapshot(live, parent);
    }
    assert_eq!(actual, fixture);
    let history = agent
        .framework()
        .history()
        .unwrap()
        .read_session("search-contract", fw::HistoryQuery::default())
        .await
        .unwrap();
    let public = serde_json::to_value(
        crate::adapters::inbound::http::chats::history::ReadHistoryResponse::from(history),
    )
    .unwrap();
    let parents: Vec<_> = public["records"]["toolInvocations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["batch"].is_null())
        .collect();
    assert_eq!(parents.len(), 3);
    for record in parents {
        let expected = cp
            .history
            .tool_invocations
            .iter()
            .find(|r| r.id.to_string() == record["id"].as_str().unwrap())
            .unwrap();
        assert_eq!(record["structured"], *expected.structured.as_ref().unwrap());
    }
    assert_eq!(
        SqliteAgentEventReplayStore::connect(path.to_str().unwrap())
            .unwrap()
            .replay_protocol_events("search-contract")
            .unwrap(),
        service.protocol_events("search-contract").unwrap()
    );
    assert!(host.close().await.is_success());
}
