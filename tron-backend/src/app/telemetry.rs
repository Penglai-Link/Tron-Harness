//! Host composition: Profile isolation, retention settings and Tron trace attributes.
use crate::adapters::outbound::observability::{TelemetryProjection, now};
use crate::application::observability::*;
use eliteforge_agent_core::{TelemetryStore, TelemetryStoreService, TelemetryTrace};
use eliteforge_agent_official_plugin_otel::{
    OtelRuntimeConfig, OtelRuntimePlugin, OtelRuntimeService, TelemetryConfig, TelemetryPlugin,
    TraceParent,
};
use eliteforge_agent_official_plugin_otel_collector::{
    CollectorConfig, CollectorPlugin, CollectorService,
};
use eliteforge_agent_official_plugin_storage_local::TelemetryStoragePlugin;
use eliteforge_agent_plugin_core::PluginContainer;
use opentelemetry::{
    Context, KeyValue,
    trace::{FutureExt, SpanKind},
    trace::{TraceContextExt, Tracer, TracerProvider},
};
use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};
pub struct LocalTelemetry {
    pub(crate) store: Arc<dyn TelemetryStore>,
    pub(crate) runtime: Arc<OtelRuntimeService>,
    pub endpoint: String,
    pub(crate) authorization: String,
    container: tokio::sync::Mutex<PluginContainer>,
}
impl LocalTelemetry {
    pub async fn open(path: &Path) -> Result<Self, ObservabilityError> {
        let container = PluginContainer::build(vec![
            Arc::new(TelemetryStoragePlugin::new(
                path,
                eliteforge_agent_core::TelemetryRetention {
                    trace_max_age: Duration::from_secs(TRACE_RETENTION_DAYS as u64 * 86400),
                    metric_max_age: Duration::from_secs(METRIC_RETENTION_DAYS as u64 * 86400),
                    max_completed_traces: Some(MAX_TRACE_RECORDS),
                },
            )),
            Arc::new(CollectorPlugin {
                storage_plugin_id: "storage.telemetry".into(),
                config: CollectorConfig {
                    bind: "127.0.0.1:0".parse().unwrap(),
                    authorization: format!(
                        "Bearer {:032x}{:032x}",
                        rand::random::<u128>(),
                        rand::random::<u128>()
                    ),
                    max_concurrent_requests: 8,
                    cleanup_interval: Duration::from_secs(3600),
                },
            }),
            Arc::new(OtelRuntimePlugin {
                config: OtelRuntimeConfig {
                    resource: BTreeMap::from([("service.name".into(), "tron-backend".into())]),
                    process_metrics: true,
                    use_proxy: false,
                    ..Default::default()
                },
            }),
        ])
        .await
        .map_err(|e| {
            tracing::error!("telemetry plugins failed: {e}");
            ObservabilityError::Storage
        })?;
        let services = container.services();
        let runtime = services
            .get::<OtelRuntimeService>()
            .ok_or(ObservabilityError::Storage)?;
        let collector = services
            .get::<CollectorService>()
            .ok_or(ObservabilityError::Storage)?;
        let store = services
            .get::<TelemetryStoreService>()
            .ok_or(ObservabilityError::Storage)?
            .0
            .clone();
        Ok(Self {
            store,
            runtime,
            endpoint: collector.endpoint.clone(),
            authorization: collector.authorization.clone(),
            container: tokio::sync::Mutex::new(container),
        })
    }
    pub fn service(&self) -> ObservabilityService {
        ObservabilityService(Arc::new(TelemetryProjection {
            store: self.store.clone(),
            instance_id: self.runtime.instance_id.clone(),
        }))
    }
    pub fn connection(&self) -> CollectorConnection {
        CollectorConnection {
            endpoint: self.endpoint.clone(),
            authorization: self.authorization.clone(),
            trace_retention_days: TRACE_RETENTION_DAYS,
            metric_retention_days: METRIC_RETENTION_DAYS,
            max_trace_records: MAX_TRACE_RECORDS,
        }
    }
    pub fn plugin(&self) -> Result<TelemetryPlugin, String> {
        TelemetryPlugin::new(
            "tron.telemetry",
            &self.runtime.traces,
            &self.runtime.metrics,
            TelemetryConfig {
                record_identifiers: true,
                record_error_reports: false,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())
    }
    pub(crate) fn register(
        &self,
        trace_id: &str,
        workspace: &str,
        session: &str,
        run: &str,
        agent: &str,
    ) -> Result<(), ObservabilityError> {
        let attributes = [
            ("tron.workspace.code", workspace),
            ("tron.session.code", session),
            ("tron.run.code", run),
            ("tron.agent.code", agent),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(
            self.store.register_trace(TelemetryTrace {
                trace_id: trace_id.into(),
                started_at: now(),
                attributes,
                ..Default::default()
            }),
        )
        .map_err(|_| ObservabilityError::Storage)
    }
    pub(crate) fn begin_operation(&self, name: &str, parent: &Context, attributes: Vec<KeyValue>) -> TraceGuard {
        let tracer = self.runtime.traces.tracer("tron.background");
        let metadata = attributes.iter().map(|kv| (kv.key.to_string(), kv.value.to_string())).collect();
        TraceGuard::tracked(parent.with_span(tracer.span_builder(name.to_owned())
            .with_attributes(attributes).start_with_context(&tracer, parent)), self.store.clone(), metadata)
    }

    pub fn begin_http(&self, method: &str, path: &str, parent: &Context) -> Context {
        let tracer = self.runtime.traces.tracer("tron.http");
        let span = tracer.span_builder(format!("{method} {path}"))
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("http.request.method", method.to_owned()),
                KeyValue::new("url.path", path.to_owned()),
            ])
            .start_with_context(&tracer, parent);
        parent.with_span(span)
    }

    pub fn begin(
        &self,
        context: &crate::domain::agent::AgentRunContext,
        request: &mut eliteforge_agent_core::RunRequest,
    ) -> Result<TraceGuard, ObservabilityError> {
        let root =
            Context::current().with_span(self.runtime.traces.tracer("tron").start_with_context("tron.session.run", &Context::current()));
        for (key, value) in [
            ("tron.workspace.code", context.workspace_code.as_str()),
            ("tron.session.code", request.session_id.as_str()),
            ("tron.run.code", context.run_code.as_str()),
            (
                "tron.agent.code",
                context.agent_code.as_deref().unwrap_or("main-agent"),
            ),
            ("agent.operation.outcome", "unknown"),
        ] {
            root.span()
                .set_attribute(KeyValue::new(key, value.to_owned()));
        }
        self.register(
            &root.span().span_context().trace_id().to_string(),
            &context.workspace_code,
            &request.session_id,
            &context.run_code,
            context.agent_code.as_deref().unwrap_or("main-agent"),
        )?;
        request.extensions.insert(TraceParent(root.clone()));
        tracing::info!(trace_id=%root.span().span_context().trace_id(), span_id=%root.span().span_context().span_id(), workspace_code=%context.workspace_code, session_code=%request.session_id, agent_code=context.agent_code.as_deref().unwrap_or("main-agent"), "Agent run started");
        Ok(TraceGuard::tracked(root, self.store.clone(), BTreeMap::new()))
    }
    pub async fn close(&self) -> Result<(), String> {
        let report = self
            .container
            .lock()
            .await
            .close_async(Duration::from_secs(10))
            .await;
        if report.is_success() {
            Ok(())
        } else {
            Err(format!("{report:?}"))
        }
    }
    #[cfg(test)]
    pub async fn flush(&self) {
        self.runtime.force_flush().await.unwrap();
    }
}
pub struct TraceGuard(pub(super) Context, Option<Arc<dyn TelemetryStore>>, std::sync::atomic::AtomicBool);
impl TraceGuard {
    fn tracked(context: Context, store: Arc<dyn TelemetryStore>, mut attributes: BTreeMap<String, String>) -> Self {
        attributes.insert("agent.operation.outcome".into(), "unknown".into());
        attributes.insert("tron.operation.active".into(), "true".into());
        if let Err(error) = crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(store.register_trace(TelemetryTrace {
            trace_id: context.span().span_context().trace_id().to_string(), started_at: now(), attributes, ..Default::default()
        })) { tracing::error!(%error, "Trace registration persistence failed"); }
        Self(context, Some(store), std::sync::atomic::AtomicBool::new(true))
    }
    pub(crate) fn complete(&self, outcome: &str) {
        if !self.2.swap(false, std::sync::atomic::Ordering::AcqRel) { return; }
        self.0.span().set_attribute(KeyValue::new("agent.operation.outcome", outcome.to_owned()));
        if let Some(store) = &self.1 {
            if let Err(error) = crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(store.register_trace(TelemetryTrace {
                trace_id: self.0.span().span_context().trace_id().to_string(), started_at: now(), ended_at: Some(now()),
                status_code: if outcome == "error" { 2 } else { 0 },
                attributes: BTreeMap::from([("agent.operation.outcome".into(), outcome.to_owned()), ("tron.operation.active".into(), "false".into())]), ..Default::default()
            })) { tracing::error!(%error, "Trace completion persistence failed"); }
        }
    }
}
impl TraceGuard {
    pub fn finish(
        &self,
        result: &Result<eliteforge_agent_core::RunResult, eliteforge_agent_core::AgentError>,
    ) {
        use eliteforge_agent_core::{AgentError, RunStatus};
        let outcome = match result {
            Ok(r) => match r.status {
                RunStatus::Completed => "completed",
                RunStatus::AwaitingInput => "waiting",
                RunStatus::StepLimit | RunStatus::ToolCallLimit => "interrupted",
            },
            Err(e) if matches!(e.primary(), AgentError::Cancelled) => "cancelled",
            Err(_) => "error",
        };
        let span = self.0.span();
        let ids = span.span_context();
        if outcome == "error" {
            tracing::error!(trace_id=%ids.trace_id(), span_id=%ids.span_id(), outcome, "Agent run failed");
        } else {
            tracing::info!(trace_id=%ids.trace_id(), span_id=%ids.span_id(), outcome, "Agent run finished");
        }
        self.complete(outcome);
    }
}
impl Drop for TraceGuard {
    fn drop(&mut self) {
        self.complete("unknown");
        self.0.span().end();
    }
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod tests;

/// OTel context is thread-local; capture it before crossing a Tokio task boundary.
pub(crate) fn spawn_traced<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    {
        use tracing::Instrument;
        tokio::spawn(future.instrument(tracing::Span::current()).with_context(Context::current()))
    }
}
