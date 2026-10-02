use super::*;
fn summary_key() -> eliteforge_agent_official_plugin_storage_local::SummaryKey {
    eliteforge_agent_official_plugin_storage_local::SummaryKey {
        conversation: fw::ConversationKey::new("CONV-1"),
        generation: "fixture".into(),
        block_id: "checkpoint".into(),
        policy_version: "v1".into(),
        configuration_digest: {
            use sha2::{Digest, Sha256};
            format!("{:x}", Sha256::digest(b"shutdown-fixture-policy-v1"))
        },
    }
}
fn assert_pty_closed(host: &FrameworkHost) {
    let result = host
        .pty()
        .start(eliteforge_agent_capability_tools::PtyStart {
            owner: "fixture".into(),
            invocation_id: None,
            command: "exit 0".into(),
            args: vec![],
            program: "/bin/sh".into(),
            program_args: vec!["-c".into(), "exit 0".into()],
            cwd: host.0.path.parent().unwrap().into(),
            env: Default::default(),
        });
    assert!(
        result
            .err()
            .expect("closed PTY must reject admission")
            .to_string()
            .contains("closed")
    );
}
#[tokio::test]
async fn timed_out_client_close_is_owned_and_late_report_observable_without_racing_cache() {
    let host = FrameworkHost::temporary(None).unwrap();
    let summaries = host.summary_store().await.unwrap();
    let expected = seed_summary(&summaries).await;
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    host.bind_model_client(Arc::new(ClosingClient {
        entered: entered.clone(),
        release: release.clone(),
        calls: calls.clone(),
    }))
    .unwrap();
    let closing = host.clone();
    let first = tokio::spawn(async move { closing.close().await });
    within(entered.notified()).await;
    let initial = tokio::time::timeout(Duration::from_secs(8), first)
        .await
        .unwrap()
        .unwrap();
    assert!(initial.timeouts >= 1);
    assert!(
        initial
            .errors
            .iter()
            .any(|value| value.contains("unfinished"))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_summary_readable(&summaries, &expected).await;
    assert_pty_closed(&host);
    release.notify_one();
    let final_report = within(host.wait_shutdown_completion()).await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        final_report
            .errors
            .iter()
            .any(|value| value == "fixture MCP close failed")
    );
    assert!(
        !final_report
            .errors
            .iter()
            .any(|value| value.contains("still draining"))
    );
    assert_eq!(final_report.panics, 1);
    assert_summary_closed(&summaries).await;
    assert_persisted_summary(&host, &expected).await;
    assert_eq!(host.close().await, initial);
}
#[tokio::test]
async fn stalled_native_drain_does_not_skip_independent_closes_and_retains_dependent_store_owner() {
    let host = FrameworkHost::temporary(None).unwrap();
    let summaries = host.summary_store().await.unwrap();
    let expected = seed_summary(&summaries).await;
    let registration = host.reserve_native("stalled-native").unwrap();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let calls = Arc::new(AtomicUsize::new(0));
    release.notify_one();
    host.bind_model_client(Arc::new(ClosingClient {
        entered,
        release,
        calls: calls.clone(),
    }))
    .unwrap();
    let report = tokio::time::timeout(Duration::from_secs(8), host.close())
        .await
        .unwrap();
    assert!(report.timeouts >= 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_pty_closed(&host);
    assert_summary_readable(&summaries, &expected).await;
    drop(registration);
    let final_report = within(host.wait_shutdown_completion()).await;
    assert!(
        final_report
            .errors
            .iter()
            .any(|value| value == "fixture MCP close failed")
    );
    assert_summary_closed(&summaries).await;
    assert_persisted_summary(&host, &expected).await;
}
struct PanicClose;
impl ModelClient for PanicClose {
    fn generate<'a>(
        &'a self,
        _: &'a AgentModelConfig,
        _: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        Box::pin(async { Err("unused".into()) })
    }
    fn generate_once<'a>(
        &'a self,
        c: &'a AgentModelConfig,
        r: &'a ModelRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelResponse, String>> + Send + 'a>> {
        self.generate(c, r)
    }
    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let _ = self.close_observed().await;
        })
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
        Box::pin(async { panic!("fixture owned close panic") })
    }
}
#[tokio::test]
async fn close_callback_panic_is_reported_and_other_resource_owners_still_close() {
    let host = FrameworkHost::temporary(None).unwrap();
    let summaries = host.summary_store().await.unwrap();
    let expected = seed_summary(&summaries).await;
    host.bind_model_client(Arc::new(PanicClose)).unwrap();
    let report = within(host.close()).await;
    assert_eq!(report.panics, 1);
    assert!(!report.is_success());
    assert_pty_closed(&host);
    assert_summary_closed(&summaries).await;
    assert_persisted_summary(&host, &expected).await;
    assert_eq!(host.wait_shutdown_completion().await, report);
}
#[test]
fn neutral_execution_snapshot_roundtrips_full_limits_and_conservative_unknown_usage() {
    let native = fw::RunExecutionSnapshot {
        limits: fw::RunLimits::default(),
        budget_usage: fw::Usage {
            input_tokens: u64::from(u32::MAX) + 9,
            output_tokens: 128,
        },
        model_calls: 5,
        child_runs: 3,
        steps: 2,
        tool_calls: 1,
        deadline_unix_ms: 999_999,
        opaque: false,
        incomplete: true,
        historical_calls_bound: Some(3),
    };
    let product = host_snapshot::product_snapshot(native.clone()).unwrap();
    assert_eq!(
        product.budget_usage.input_tokens,
        native.budget_usage.input_tokens
    );
    let decoded: fw::RunExecutionSnapshot = product.try_into().unwrap();
    assert_eq!(decoded, native);
}

async fn seed_summary(
    store: &eliteforge_agent_official_plugin_storage_local::SqliteSummaryStore,
) -> eliteforge_agent_official_plugin_storage_local::VersionedSummary {
    use eliteforge_agent_official_plugin_storage_local::{ConversationSummary, SummaryStore};
    use sha2::{Digest, Sha256};
    let value = ConversationSummary {
        format_version: 1,
        source_revision: 1,
        covered_messages: 1,
        source_digest: format!("{:x}", Sha256::digest(b"original fixture conversation")),
        summary: "persisted shutdown fixture summary".into(),
    };
    assert!(
        store
            .compare_exchange(&summary_key(), None, &value)
            .await
            .expect("valid fixture summary CAS")
    );
    let saved = store
        .load(&summary_key())
        .await
        .expect("fixture summary must be readable before close")
        .expect("CAS summary present");
    assert_eq!(saved.value, value);
    saved
}
async fn assert_summary_readable(
    store: &eliteforge_agent_official_plugin_storage_local::SqliteSummaryStore,
    expected: &eliteforge_agent_official_plugin_storage_local::VersionedSummary,
) {
    use eliteforge_agent_official_plugin_storage_local::SummaryStore;
    let actual = store
        .load(&summary_key())
        .await
        .expect("dependent summary store must remain readable while owned close is unfinished")
        .expect("retained summary must remain present");
    assert_eq!(
        &actual, expected,
        "unfinished close must preserve original summary version and data"
    );
}
async fn assert_summary_closed(
    store: &eliteforge_agent_official_plugin_storage_local::SqliteSummaryStore,
) {
    use eliteforge_agent_official_plugin_storage_local::SummaryStore;
    let error = store
        .load(&summary_key())
        .await
        .err()
        .expect("final shutdown must close the dependent summary store");
    assert!(
        error.to_string().contains("closed"),
        "expected closed store, got {error}"
    );
}
async fn assert_persisted_summary(
    host: &FrameworkHost,
    expected: &eliteforge_agent_official_plugin_storage_local::VersionedSummary,
) {
    let reopened = eliteforge_agent_official_plugin_storage_local::SqliteSummaryStore::open(
        host.0.path.with_extension("context-summaries.sqlite"),
        Default::default(),
    )
    .await
    .unwrap();
    assert_summary_readable(&reopened, expected).await;
    reopened.close().await.unwrap();
}
