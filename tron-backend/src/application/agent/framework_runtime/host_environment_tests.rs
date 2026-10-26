use super::*;

struct UpstreamFailure;
#[async_trait::async_trait]
impl fw::AgentLoop for UpstreamFailure {
    async fn run(
        &self,
        _: &fw::RunRequest,
        _: &fw::RunContext,
        _: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        Err(fw::AgentError::Indeterminate(
            r#"model stream failed before completion: {"type":"response.failed","response":{"id":"upstream-request","error":{"code":"server_error","message":"上游生成失败"}}}"#.into(),
        ))
    }
}

#[tokio::test]
async fn host_exposes_upstream_error_details_in_agui_and_durable_replay() {
    use crate::adapters::outbound::persistence::SqliteAgentEventReplayStore;
    use crate::application::agent::{AgentAguiReplayStore, AgentAguiStreamService};
    use crate::shared::event_bus::{AguiEventSink, EventBusConfig};
    use eliteforge_agent_official_plugin_agui::AgUiStream;

    let host = FrameworkHost::temporary(None).unwrap();
    host.prepare_artifacts().unwrap();
    let (packs, extensions) = host.capability_state();
    let framework = host
        .framework_builder(&packs, &extensions)
        .await
        .unwrap()
        .plugin(fw::AgentLoopPlugin::named(
            "fixture.failure",
            UpstreamFailure,
        ))
        .build()
        .await
        .unwrap();
    let mut selection = host
        .main_selection(&framework, &packs, &extensions)
        .unwrap();
    selection.agent_loop = Some("fixture.failure".into());
    selection.tool_policies = Some(vec![]);
    selection.tool_providers = Some(vec![]);
    let agent = framework.agent(selection).build().await.unwrap();
    let mut session = fw::SessionRecord::new("Error replay fixture");
    session.id = "error-session".into();
    framework.sessions().unwrap().create(session).await.unwrap();
    let store =
        Arc::new(SqliteAgentEventReplayStore::connect(host.0.path.to_str().unwrap()).unwrap());
    let service = AgentAguiStreamService::with_replay_store(
        Arc::new(AguiEventSink::new(&EventBusConfig::default())),
        store.clone(),
    );
    let request = fw::RunRequest::new("error-session", "test");
    let (handle, mut stream) = agent.start_with_output::<AgUiStream>(request).unwrap();
    let (result, ()) = tokio::join!(agent.wait(handle), async {
        while let Some(event) = stream.recv().await {
            let data = serde_json::to_value(event).unwrap();
            service
                .record_protocol_event(
                    "error-session",
                    crate::domain::agent::AgentAguiProtocolEvent {
                        sse_event: data["type"].as_str().unwrap().into(),
                        data,
                    },
                )
                .unwrap();
        }
    });
    assert!(result.is_err());
    let events = service.protocol_events("error-session").unwrap();
    let error = events
        .iter()
        .find(|event| event.sse_event == "RUN_ERROR")
        .unwrap();
    let message = error.data["message"].as_str().unwrap();
    assert!(message.contains("server_error"));
    assert!(message.contains("上游生成失败"));
    assert!(message.contains("upstream-request"));
    assert_eq!(error.data["code"], "AGENT_RUN_FAILED");
    let replay = store.replay_protocol_events("error-session").unwrap();
    assert!(replay.iter().any(|event| event.data == error.data));
    assert!(agent.close().await.is_success());
    assert!(framework.close().await.is_success());
    assert!(host.close().await.is_success());
}

#[tokio::test]
async fn internal_agents_reuse_identity_and_close_independently_per_workspace() {
    let host = FrameworkHost::temporary(None).unwrap();
    let first = host
        .internal_agent("workspace-a", "reviewer")
        .await
        .unwrap();
    let reused = host
        .internal_agent("workspace-a", "reviewer")
        .await
        .unwrap();
    let second = host.internal_agent("workspace-a", "writer").await.unwrap();
    let other_workspace = host
        .internal_agent("workspace-b", "reviewer")
        .await
        .unwrap();
    assert_eq!(host.initialized_internal_agents().len(), 3);
    host.set_extension_enabled("tool", false).await.unwrap();
    host.set_extension_enabled("tool", true).await.unwrap();
    assert!(first.close().await.is_success());
    let (packs, extensions) = host.capability_state();
    let framework = second.framework();
    let selection = host
        .main_selection(&framework, &packs, &extensions)
        .unwrap();
    assert!(
        reused
            .reconfigure(framework.agent(selection.clone()))
            .await
            .is_err()
    );
    second
        .reconfigure(framework.agent(selection.clone()))
        .await
        .unwrap();
    other_workspace
        .reconfigure(framework.agent(selection))
        .await
        .unwrap();
    assert!(!framework.is_closed());
    assert!(host.close().await.is_success());
    assert!(framework.is_closed());
}

#[tokio::test]
async fn main_and_continuation_share_storage_and_agent_close_keeps_framework_alive() {
    let host = FrameworkHost::temporary(None).unwrap();
    let main = {
        let _admission = host.0.config_lock.lock().await;
        host.agent().await.unwrap().clone()
    };
    let continuation = host.continuation_agent().await.unwrap();
    let key = fw::ConversationKey::new("shared-profile-session");
    main.append_conversation(&key, 0, vec![fw::Message::user("shared history")])
        .await
        .unwrap();
    assert_eq!(
        continuation.conversation(&key).await.unwrap().messages[0]
            .content
            .text(),
        "shared history"
    );
    assert!(main.close().await.is_success());
    assert!(!continuation.framework().is_closed());
    assert_eq!(continuation.conversation(&key).await.unwrap().revision, 1);
    continuation
        .framework()
        .sessions()
        .unwrap()
        .archive(&key.session_id)
        .await
        .unwrap();
    assert!(
        continuation
            .framework()
            .execute(fw::DeleteSession(key.session_id.clone()))
            .await
            .unwrap()
    );
    let framework = continuation.framework();
    let store = framework
        .services()
        .unwrap()
        .get::<fw::ConversationStoreService>()
        .unwrap()
        .0
        .clone();
    assert!(store.history(&key.session_id).await.unwrap().is_empty());
    assert!(host.close().await.is_success());
    assert!(continuation.framework().is_closed());
}

#[tokio::test]
async fn capability_selection_changes_preserve_conversation_storage() {
    let host = FrameworkHost::temporary(None).unwrap();
    let main = {
        let _admission = host.0.config_lock.lock().await;
        host.agent().await.unwrap().clone()
    };
    let key = fw::ConversationKey::new("selection-session");
    main.append_conversation(&key, 0, vec![fw::Message::user("retained")])
        .await
        .unwrap();
    host.set_pack_enabled("framework.local.scoped", false)
        .await
        .unwrap();
    host.set_extension_enabled("tool", false).await.unwrap();
    host.set_extension_enabled("tool", true).await.unwrap();
    host.set_pack_enabled("framework.local.scoped", true)
        .await
        .unwrap();
    assert_eq!(
        main.conversation(&key).await.unwrap().messages[0]
            .content
            .text(),
        "retained"
    );
    assert!(host.set_extension_enabled("storage", false).await.is_err());
    assert!(
        host.set_pack_enabled("missing.package", true)
            .await
            .is_err()
    );
    assert!(!host.capability_state().0.contains_key("missing.package"));
    assert!(host.close().await.is_success());
}


#[tokio::test]
async fn gateway_selection_and_prompt_enablement_follow_tool_extension() {
    let host = FrameworkHost::temporary(None).unwrap();
    host.prepare_artifacts().unwrap();
    let (mut packs, extensions) = host.capability_state();
    // It is infrastructure, so a stale private pack flag must not disable it.
    packs.insert("framework.tools.discovery".into(), false);
    let framework = host
        .framework_builder(&packs, &extensions)
        .await
        .unwrap()
        .build()
        .await
        .unwrap();
    for enabled in [true, false, true] {
        host.set_extension_enabled("tool", enabled).await.unwrap();
        let (_, extensions) = host.capability_state();
        let selection = host
            .main_selection(&framework, &packs, &extensions)
            .unwrap();
        let providers = selection.tool_providers.unwrap();
        assert_eq!(
            providers.iter().any(|id| id == "framework.tools.discovery"),
            enabled
        );
        assert_eq!(host.gateway_enabled(), enabled);
        if !enabled {
            assert!(providers.is_empty());
            assert!(
                !selection
                    .context_sources
                    .unwrap()
                    .iter()
                    .any(|id| matches!(id.as_str(), "skills.default" | "agent.a2ui"))
            );
        }
    }
    assert!(framework.close().await.is_success());
    assert!(host.close().await.is_success());
}
