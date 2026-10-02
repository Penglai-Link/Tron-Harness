use super::*;
use eliteforge_agent_framework as fw;
struct ParentLoop {
    entered: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<fw::RunContext>>>,
}
#[async_trait::async_trait]
impl fw::AgentLoop for ParentLoop {
    fn requires_conversations(&self) -> bool {
        false
    }
    fn cooperative_cancellation(&self) -> bool {
        true
    }
    async fn run(
        &self,
        _: &fw::RunRequest,
        context: &fw::RunContext,
        _: &fw::AgentServices,
    ) -> Result<fw::RunResult, fw::AgentError> {
        if let Some(sender) = self.entered.lock().await.take() {
            let _ = sender.send(context.clone());
        }
        context.cancellation.cancelled().await;
        Err(fw::AgentError::Cancelled)
    }
}
#[tokio::test]
async fn gaps_keep_session_exclusive_and_stop_before_binding_cancels_native_root() {
    let registry = ActiveRunRegistry::new();
    let group = registry
        .continuation_group("session", "initial", "REACT")
        .unwrap();
    assert!(matches!(
        registry.register("session", "ordinary", "REACT", true),
        Err(ActiveRunError::AlreadyActive)
    ));
    assert_eq!(
        registry.snapshot("session").unwrap().unwrap().run_code,
        "initial"
    );
    registry.request_stop("session", "initial").unwrap();
    let (sender, _) = tokio::sync::oneshot::channel();
    let agent = fw::AgentBuilder::new()
        .default_models(false)
        .plugin(fw::AgentLoopPlugin::new(ParentLoop {
            entered: tokio::sync::Mutex::new(Some(sender)),
        }))
        .build()
        .await
        .unwrap();
    let mut request = fw::RunRequest::new("native-root", "");
    group.attach(&mut request.extensions);
    let handle = agent.start(request).unwrap();
    group
        .bind(agent.clone(), agent.control(&handle).unwrap())
        .unwrap();
    assert!(matches!(
        agent.wait(handle).await.unwrap_err().primary(),
        fw::AgentError::Cancelled
    ));
    drop(group);
    assert!(registry.snapshot("session").unwrap().is_none());
    assert!(
        registry
            .register("session", "manual", "REACT", true)
            .is_ok()
    );
    agent.close().await;
}
#[tokio::test]
async fn only_explicit_parent_can_admit_child_and_stop_identity_survives_child_gap() {
    let registry = ActiveRunRegistry::new();
    let group = registry
        .continuation_group("session", "initial", "REACT")
        .unwrap();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let agent = fw::AgentBuilder::new()
        .default_models(false)
        .plugin(fw::AgentLoopPlugin::new(ParentLoop {
            entered: tokio::sync::Mutex::new(Some(sender)),
        }))
        .build()
        .await
        .unwrap();
    let mut request = fw::RunRequest::new("native-root", "");
    group.attach(&mut request.extensions);
    let handle = agent.start(request).unwrap();
    group
        .bind(agent.clone(), agent.control(&handle).unwrap())
        .unwrap();
    let parent = receiver.await.unwrap();
    let child = fw::with_continuation_parent(&parent, async {
        registry.register("session", "child", "REACT", true)
    })
    .await
    .unwrap();
    assert_eq!(
        registry.snapshot("session").unwrap().unwrap().run_code,
        "child"
    );
    let rejected = fw::with_continuation_parent(&parent, async {
        registry.register("session", "rejected", "REACT", true)
    })
    .await;
    assert!(matches!(rejected, Err(ActiveRunError::AlreadyActive)));
    drop(child);
    let gap = registry.snapshot("session").unwrap().unwrap();
    assert_eq!(gap.run_code, "child");
    assert!(!gap.supports_steer);
    assert!(matches!(
        registry.request_stop("session", "initial"),
        Err(ActiveRunError::RunMismatch { .. })
    ));
    registry.request_stop("session", "child").unwrap();
    assert!(matches!(
        agent.wait(handle).await.unwrap_err().primary(),
        fw::AgentError::Cancelled
    ));
    drop(group);
    agent.close().await;
}
