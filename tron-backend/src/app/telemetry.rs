//! Host composition: Profile isolation, retention settings and Tron trace attributes.
use crate::adapters::outbound::observability::{TelemetryProjection, now};
use crate::application::observability::*;
use eliteforge_agent_core::{ErrorRedactor, TelemetryStore, TelemetryStoreService, TelemetryTrace};
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
    trace::{FutureExt, Span as _, SpanKind},
    trace::{TraceContextExt, Tracer, TracerProvider},
};
use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};

struct TronErrorRedactor;

impl ErrorRedactor for TronErrorRedactor {
    fn redact(&self, _kind: &str, message: &str) -> Option<String> {
        Some(crate::application::short_term_memory::redact_sensitive_text(message))
    }
}

pub(crate) fn record_error(
    context: &Context,
    kind: &str,
    error: &(dyn std::error::Error + 'static),
) {
    let report = eliteforge_agent_core::ErrorReport::from_error(kind, error, &TronErrorRedactor);
    if let Ok(report) = serde_json::to_string(&report) {
        context.span().add_event(
            "agent.error",
            vec![
                KeyValue::new("error.type", kind.to_owned()),
                KeyValue::new("agent.error.report", report),
            ],
        );
    }
}

/// Tron-specific observability indexes on top of the framework-neutral
/// telemetry tables. The store's clear() drops and recreates its tables, which
/// implicitly removes these indexes; replay this batch to restore them.
const HOST_INDEXES: &str = "CREATE INDEX IF NOT EXISTS ef_telemetry_traces_workspace_time ON ef_telemetry_traces(json_extract(attributes,'$.\"tron.workspace.code\"'),started_at DESC,trace_id DESC);
             CREATE INDEX IF NOT EXISTS ef_telemetry_agent_roots_time ON ef_telemetry_spans(json_extract(data,'$.startedAt') DESC,trace_id,span_id) WHERE json_extract(data,'$.name') IN ('tron.session.run','tron.assistant.execute');
             CREATE INDEX IF NOT EXISTS ef_telemetry_agent_roots_workspace ON ef_telemetry_spans(json_extract(data,'$.attributes.\"tron.workspace.code\"'),json_extract(data,'$.startedAt') DESC,trace_id,span_id) WHERE json_extract(data,'$.name') IN ('tron.session.run','tron.assistant.execute');
             CREATE INDEX IF NOT EXISTS ef_telemetry_agent_roots_session ON ef_telemetry_spans(json_extract(data,'$.attributes.\"tron.session.code\"'),json_extract(data,'$.startedAt') DESC,trace_id,span_id) WHERE json_extract(data,'$.name') IN ('tron.session.run','tron.assistant.execute');
             CREATE INDEX IF NOT EXISTS ef_telemetry_page_roots_time ON ef_telemetry_traces(started_at DESC,trace_id) WHERE json_extract(attributes,'$.\"tron.trace.page\"')='true';";

pub struct LocalTelemetry {
    pub(crate) store: Arc<dyn TelemetryStore>,
    query_database: eliteforge_agent_official_plugin_storage_local::SqliteDatabase,
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
        let query_database =
            eliteforge_agent_official_plugin_storage_local::SqliteDatabase::open(path)
                .await
                .map_err(|_| ObservabilityError::Storage)?;
        // These indexes serve Tron-specific observability views. The reusable
        // telemetry store owns only host-neutral schema and indexes.
        query_database
            .call(|connection| connection.execute_batch(HOST_INDEXES))
            .await
            .map_err(|_| ObservabilityError::Storage)?;
        Ok(Self {
            query_database,
            store,
            runtime,
            endpoint: collector.endpoint.clone(),
            authorization: collector.authorization.clone(),
            container: tokio::sync::Mutex::new(container),
        })
    }
    pub fn service(&self) -> ObservabilityService {
        ObservabilityService(Arc::new(TelemetryProjection {
            database: self.query_database.clone(),
            store: self.store.clone(),
            instance_id: self.runtime.instance_id.clone(),
        }))
    }
    /// Replay Tron-specific indexes after the store dropped and recreated the
    /// host-neutral telemetry tables during a reset.
    pub(crate) async fn recreate_host_indexes(&self) -> Result<(), ObservabilityError> {
        self.query_database
            .call(|connection| connection.execute_batch(HOST_INDEXES))
            .await
            .map_err(|_| ObservabilityError::Storage)
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
                record_error_reports: true,
                ..Default::default()
            },
        )
        .map(|plugin| plugin.with_error_redactor(Arc::new(TronErrorRedactor)))
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
            ("tron.trace.agent", "true"),
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
    pub(crate) fn begin_operation(
        &self,
        name: &str,
        parent: &Context,
        attributes: Vec<KeyValue>,
    ) -> TraceGuard {
        let tracer = self.runtime.traces.tracer("tron.background");
        let metadata = attributes
            .iter()
            .map(|kv| (kv.key.to_string(), kv.value.to_string()))
            .collect();
        TraceGuard::tracked(
            parent.with_value(self.runtime.clone()).with_span(
                tracer
                    .span_builder(name.to_owned())
                    .with_attributes(attributes)
                    .start_with_context(&tracer, parent),
            ),
            self.store.clone(),
            name,
            parent,
            metadata,
        )
    }

    pub fn begin_http(&self, method: &str, path: &str, parent: &Context) -> Context {
        let tracer = self.runtime.traces.tracer("tron.http");
        let span = tracer
            .span_builder(format!("{method} {path}"))
            .with_kind(SpanKind::Server)
            .with_attributes([
                KeyValue::new("tron.trace.page", "true"),
                KeyValue::new("http.request.method", method.to_owned()),
                KeyValue::new("url.path", path.to_owned()),
            ])
            .start_with_context(&tracer, parent);
        parent.with_value(self.runtime.clone()).with_span(span)
    }

    pub fn begin(
        &self,
        context: &crate::domain::agent::AgentRunContext,
        request: &mut eliteforge_agent_core::RunRequest,
    ) -> Result<TraceGuard, ObservabilityError> {
        let root = Context::current()
            .with_value(self.runtime.clone())
            .with_span(
                self.runtime
                    .traces
                    .tracer("tron")
                    .start_with_context("tron.session.run", &Context::current()),
            );
        for (key, value) in [
            ("tron.workspace.code", context.workspace_code.as_str()),
            ("tron.session.code", request.session_id.as_str()),
            ("tron.run.code", context.run_code.as_str()),
            (
                "tron.agent.code",
                context.agent_code.as_deref().unwrap_or("main-agent"),
            ),
            ("agent.operation.outcome", "unknown"),
            ("agent.telemetry.exclude_self", "true"),
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
        let mut assistant_attributes: BTreeMap<String, String> = BTreeMap::new();
        if let Some(owner) = context
            .collaboration_owner
            .as_ref()
            .filter(|owner| owner.kind == "ASSISTANT")
        {
            assistant_attributes.insert("tron.assistant.code".into(), owner.code.clone());
            if let Some(task) = &owner.task_code {
                assistant_attributes.insert("tron.task.code".into(), task.clone());
            }
            if let Some(origin) = &owner.origin_session_code {
                assistant_attributes
                    .insert("tron.assistant.origin.session.code".into(), origin.clone());
            }
            if let Some(memory) = &context.short_term_memory {
                assistant_attributes.insert(
                    "tron.assistant.input.kind".into(),
                    memory.input_kind.clone(),
                );
            }
        }
        for (key, value) in &assistant_attributes {
            root.span()
                .set_attribute(KeyValue::new(key.clone(), value.clone()));
        }
        request.extensions.insert(TraceParent(root.clone()));
        tracing::info!(trace_id=%root.span().span_context().trace_id(), span_id=%root.span().span_context().span_id(), workspace_code=%context.workspace_code, session_code=%request.session_id, agent_code=context.agent_code.as_deref().unwrap_or("main-agent"), "Agent run started");
        for (key, value) in [
            ("tron.workspace.code", context.workspace_code.as_str()),
            ("tron.session.code", request.session_id.as_str()),
            ("tron.run.code", context.run_code.as_str()),
            (
                "tron.agent.code",
                context.agent_code.as_deref().unwrap_or("main-agent"),
            ),
        ] {
            assistant_attributes.insert(key.into(), value.into());
        }
        Ok(TraceGuard::tracked(
            root,
            self.store.clone(),
            "tron.session.run",
            &Context::current(),
            assistant_attributes,
        ))
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
}
pub struct TraceGuard(
    pub(super) Context,
    Option<Arc<dyn TelemetryStore>>,
    std::sync::atomic::AtomicBool,
    eliteforge_agent_core::TelemetrySpan,
);
impl TraceGuard {
    fn tracked(
        context: Context,
        store: Arc<dyn TelemetryStore>,
        name: &str,
        parent: &Context,
        mut attributes: BTreeMap<String, String>,
    ) -> Self {
        attributes.insert("agent.operation.outcome".into(), "unknown".into());
        attributes.insert("agent.operation.active".into(), "true".into());
        attributes.insert("tron.operation.active".into(), "true".into());
        if name == "tron.session.run" {
            attributes.insert("agent.telemetry.exclude_self".into(), "true".into());
        }
        if let Err(error) =
            crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(
                store.register_trace(TelemetryTrace {
                    trace_id: context.span().span_context().trace_id().to_string(),
                    started_at: now(),
                    attributes: attributes.clone(),
                    ..Default::default()
                }),
            )
        {
            tracing::error!(%error, "Trace registration persistence failed");
        }
        let ids = context.span().span_context().clone();
        let snapshot = eliteforge_agent_core::TelemetrySpan {
            trace_id: ids.trace_id().to_string(),
            span_id: ids.span_id().to_string(),
            parent_span_id: parent
                .span()
                .span_context()
                .is_valid()
                .then(|| parent.span().span_context().span_id().to_string()),
            name: name.into(),
            started_at: now(),
            ended_at: now(),
            attributes: attributes.clone(),
            ..Default::default()
        };
        // Keep a queryable running entry until the completed OTel span arrives.
        if let Err(error) =
            crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(
                store.write_spans(vec![snapshot.clone()]),
            )
        {
            tracing::error!(%error, "Running span persistence failed");
        }
        Self(
            context,
            Some(store),
            std::sync::atomic::AtomicBool::new(true),
            snapshot,
        )
    }
    pub(crate) fn trace_id(&self) -> String {
        self.0.span().span_context().trace_id().to_string()
    }

    pub(crate) fn complete(&self, outcome: &str) {
        if !self.2.swap(false, std::sync::atomic::Ordering::AcqRel) {
            return;
        }
        self.0
            .span()
            .set_attribute(KeyValue::new("agent.operation.outcome", outcome.to_owned()));
        if let Some(store) = &self.1 {
            let mut snapshot = self.3.clone();
            snapshot.ended_at = now();
            snapshot.status_code = if outcome == "error" { 2 } else { 0 };
            snapshot
                .attributes
                .insert("agent.operation.outcome".into(), outcome.into());
            snapshot
                .attributes
                .insert("agent.operation.active".into(), "false".into());
            snapshot
                .attributes
                .insert("tron.operation.active".into(), "false".into());
            if let Err(error) =
                crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(
                    store.write_spans(vec![snapshot]),
                )
            {
                tracing::error!(%error, "Completed span persistence failed");
            }
            if let Err(error) =
                crate::adapters::outbound::persistence::sqlite::framework_storage::blocking(
                    store.register_trace(TelemetryTrace {
                        trace_id: self.0.span().span_context().trace_id().to_string(),
                        started_at: now(),
                        ended_at: Some(now()),
                        status_code: if outcome == "error" { 2 } else { 0 },
                        attributes: BTreeMap::from([
                            ("agent.operation.outcome".into(), outcome.to_owned()),
                            ("tron.operation.active".into(), "false".into()),
                        ]),
                        ..Default::default()
                    }),
                )
            {
                tracing::error!(%error, "Trace completion persistence failed");
            }
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
            if let Err(error) = result {
                record_error(&self.0, error.kind(), error);
            }
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

/// OTel context is thread-local; capture it before crossing a Tokio task boundary.
pub(crate) fn spawn_traced<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    {
        use tracing::Instrument;
        tokio::spawn(
            future
                .instrument(tracing::Span::current())
                .with_context(Context::current()),
        )
    }
}

/// Use the active Profile's provider without a process-global tracer.
pub(crate) fn child_operation(name: &str, attributes: Vec<KeyValue>) -> Context {
    let parent = Context::current();
    let Some(runtime) = parent.get::<Arc<OtelRuntimeService>>() else {
        return Context::new();
    };
    if !parent.span().span_context().is_valid() {
        return Context::new();
    }
    let tracer = runtime.traces.tracer("tron.business");
    let span = tracer
        .span_builder(name.to_owned())
        .with_attributes(attributes)
        .start_with_context(&tracer, &parent);
    parent.with_span(span)
}

/// Keep database diagnostics for slow or failed calls without exporting every
/// fast SQLite lookup performed by a long-running agent or a polling request.
pub(crate) fn completed_operation(
    name: &str,
    parent: &Context,
    started: std::time::SystemTime,
    ended: std::time::SystemTime,
    attributes: Vec<KeyValue>,
    failed: bool,
) {
    let Some(runtime) = parent.get::<Arc<OtelRuntimeService>>() else {
        return;
    };
    if !parent.span().span_context().is_valid() {
        return;
    }
    let tracer = runtime.traces.tracer("tron.business");
    let mut span = tracer
        .span_builder(name.to_owned())
        .with_start_time(started)
        .with_attributes(attributes)
        .start_with_context(&tracer, parent);
    if failed {
        span.set_status(opentelemetry::trace::Status::error(
            "Database operation failed",
        ));
    }
    span.end_with_timestamp(ended);
}

/// Preserve the request's Profile and parent span on blocking business workers.
pub(crate) fn spawn_blocking_traced<F, R>(operation: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let context = Context::current();
    let log_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || {
        let _context = context.attach();
        let _log = log_span.enter();
        operation()
    })
}
