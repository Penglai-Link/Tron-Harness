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
            eprintln!("telemetry plugins failed: {e}");
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
        super::telemetry_legacy::import(path, store.clone()).await?;
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
    pub fn begin(
        &self,
        context: &crate::domain::agent::AgentRunContext,
        request: &mut eliteforge_agent_core::RunRequest,
    ) -> Result<TraceGuard, ObservabilityError> {
        let root =
            Context::new().with_span(self.runtime.traces.tracer("tron").start("tron.session.run"));
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
        Ok(TraceGuard(root))
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
pub struct TraceGuard(pub(super) Context);
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
        self.0
            .span()
            .set_attribute(KeyValue::new("agent.operation.outcome", outcome));
    }
}
impl Drop for TraceGuard {
    fn drop(&mut self) {
        self.0.span().end();
    }
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod tests;
