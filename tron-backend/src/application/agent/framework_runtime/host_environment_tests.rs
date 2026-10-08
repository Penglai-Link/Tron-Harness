use super::*;

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
