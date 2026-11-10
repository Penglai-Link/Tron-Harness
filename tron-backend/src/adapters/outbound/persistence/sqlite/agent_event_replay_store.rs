//! AG-UI frames and delivery diagnostics use the Framework event abstraction.
use super::framework_storage::blocking;
use crate::{
    application::agent::{
        AgentAguiReplayStore, CanonicalRuntimeEvent, FrameworkHost,
        agui_translate::{
            translate_agui_event, translate_native_agui_event, translate_protocol_events,
        },
        protocol_event_identity,
    },
    domain::agent::{AgentAguiEvent, AgentAguiProtocolEvent},
    shared::event_bus::{BusEvent, EventSink},
};
use eliteforge_agent_core as fw;
use eliteforge_agent_official_plugin_storage_local::SqliteDatabase;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, OnceLock,
        mpsc::{SyncSender, sync_channel},
    },
    thread,
};

const HEALTH: &str = "tron/presentation-health";
struct Storage {
    sessions: Arc<dyn fw::SessionLifecycleStore>,
    database: SqliteDatabase,
    native_run_active: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}
pub struct SqliteAgentEventReplayStore {
    storage: Arc<OnceLock<Storage>>,
    failures: Arc<Mutex<BTreeMap<String, String>>>,
    health_flush: SyncSender<()>,
}
impl SqliteAgentEventReplayStore {
    pub fn new() -> Result<Self, String> {
        let storage = Arc::new(OnceLock::<Storage>::new());
        let failures = Arc::new(Mutex::new(BTreeMap::<String, String>::new()));
        let (health_flush, signals) = sync_channel(1);
        let worker_storage = storage.clone();
        let worker_failures = failures.clone();
        thread::Builder::new()
            .name("tron-presentation-health".into())
            .spawn(move || {
                while signals.recv().is_ok() {
                    let snapshot = worker_failures
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    let Some(storage) = worker_storage.get() else {
                        continue;
                    };
                    let result = blocking(storage.database.transaction(async {
                        let keys=snapshot.keys().map(|key|format!("tron.replay-failure/{key}")).collect::<Vec<_>>();
                        let selected=serde_json::to_string(&keys).map_err(|error|fw::StateError::Backend(error.to_string()))?;
                        let existing=storage.database.call_read(move |c| {
                            let mut q=c.prepare("SELECT key FROM ef_session_context WHERE session_id=?1 AND key IN (SELECT value FROM json_each(?2))")?;
                            q.query_map(rusqlite::params![HEALTH,selected],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<std::collections::BTreeSet<_>>>()
                        }).await.map_err(|error|fw::StateError::Backend(error.to_string()))?;
                        for (key, detail) in &snapshot {
                            let context = format!("tron.replay-failure/{key}");
                            if !existing.contains(&context) {
                                storage
                                    .sessions
                                    .write_context(
                                        HEALTH,
                                        &context,
                                        None,
                                        serde_json::json!({"detail":detail}),
                                    )
                                    .await?;
                            }
                        }
                        Ok(())
                    }));
                    if let Err(error) = result {
                        tracing::error!("Framework replay diagnostics failed: {error}");
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            storage,
            failures,
            health_flush,
        })
    }
    pub fn connect(path: &str) -> Result<Self, String> {
        let store = Self::new()?;
        let host = if path == ":memory:" {
            FrameworkHost::temporary(None).map_err(|e| e.to_string())?
        } else {
            FrameworkHost::persistent(path, None)
        };
        store.bind_framework(&host)?;
        Ok(store)
    }
    pub fn bind_framework(&self, host: &FrameworkHost) -> Result<(), String> {
        let database = blocking(host.shared_database()).map_err(|e| e.to_string())?;
        let sessions = blocking(host.storage_framework())
            .map_err(|e| e.to_string())?
            .sessions()
            .map_err(|e| e.to_string())?
            .store;
        blocking(async {
            match sessions.get_session(HEALTH).await {
                Ok(_) => {}
                Err(fw::StateError::NotFound) => {
                    let mut health = fw::SessionRecord::new("Presentation delivery diagnostics");
                    health.id = HEALTH.into();
                    sessions.create_session(health).await?;
                }
                Err(e) => return Err(e),
            };
            Ok::<_, fw::StateError>(())
        })
        .map_err(|e| e.to_string())?;
        self.storage
            .set(Storage {
                sessions,
                database,
                native_run_active: host.native_run_observer(),
            })
            .map_err(|_| "replay storage already bound".to_owned())?;
        let _ = self.health_flush.try_send(());
        Ok(())
    }
    fn storage(&self) -> Result<&Storage, String> {
        self.storage
            .get()
            .ok_or_else(|| "Framework presentation storage is not bound".into())
    }
    async fn session(storage: &Storage, key: &str) -> Result<String, fw::StateError> {
        match storage.sessions.get_session(key).await {
            Ok(s) => Ok(s.id),
            Err(fw::StateError::NotFound) => {
                if let Ok(id) = fw::RunId::parse_str(key) {
                    if let Ok(run) = storage.sessions.get_run(id).await {
                        return Ok(run.session_id);
                    }
                }
                // A stream can start before its conversation session exists
                // (warnings, early diagnostics). The replay journal owns its
                // session keyed by the stream, shared with later runs.
                let mut record = fw::SessionRecord::new("");
                record.id = key.into();
                storage.sessions.create_session(record).await.map(|s| s.id)
            }
            Err(e) => Err(e),
        }
    }
    fn append(
        &self,
        key: &str,
        identity: &str,
        frames: Vec<(&str, serde_json::Value)>,
    ) -> Result<(), String> {
        let storage = self.storage()?;
        blocking(storage.database.transaction(async {
            let session = Self::session(storage, key).await?;
            for (index, (category, frame)) in frames.into_iter().enumerate() {
                let mut event = fw::SessionEvent::new(
                    &session,
                    format!("tron.agui.{category}"),
                    serde_json::json!({"streamKey":key,"frame":frame}),
                );
                event.id = format!("tron.agui/{key}/{identity}/{category}/{index}");
                event.run_id = fw::RunId::parse_str(key).ok().filter(|_| key != session);
                storage.sessions.append_event(event).await?;
            }
            Ok(())
        }))
        .map_err(|e| e.to_string())
    }
    fn persist(&self, event: &BusEvent) -> Result<(), String> {
        let Some(key) = event.stream_key() else {
            return Ok(());
        };
        let mut frames = vec![];
        if let Some(frame) = translate_agui_event(key, event) {
            frames.push((
                "audit",
                serde_json::to_value(frame).map_err(|e| e.to_string())?,
            ));
        }
        for frame in translate_protocol_events(key, event) {
            frames.push((
                "protocol",
                serde_json::to_value(frame).map_err(|e| e.to_string())?,
            ));
        }
        self.append(key, &event.event_id.as_decimal_string(), frames)
    }
    fn replay<T: serde::de::DeserializeOwned + Send>(
        &self,
        key: &str,
        category: &str,
    ) -> Result<Vec<T>, String> {
        if key.trim().is_empty() {
            return Ok(vec![]);
        }
        let storage = self.storage()?;
        blocking(async {
            let session = match Self::session(storage, key).await {
                Ok(s) => s,
                Err(fw::StateError::NotFound) => return Ok(vec![]),
                Err(e) => return Err(e.to_string()),
            };
            let mut after = 0;
            let mut result = vec![];
            let kind = format!("tron.agui.{category}");
            loop {
                let page = storage
                    .sessions
                    .list_events(&session, after, 1000)
                    .await
                    .map_err(|e| e.to_string())?;
                if page.is_empty() {
                    break;
                }
                after = page.last().unwrap().sequence;
                for event in page {
                    if event.kind == kind && event.payload["streamKey"].as_str() == Some(key) {
                        result.push(
                            serde_json::from_value(event.payload["frame"].clone())
                                .map_err(|e| e.to_string())?,
                        );
                    }
                }
            }
            Ok(result)
        })
    }
}
impl AgentAguiReplayStore for SqliteAgentEventReplayStore {
    fn append_protocol_event(
        &self,
        key: &str,
        event: AgentAguiProtocolEvent,
    ) -> Result<(), String> {
        let identity =
            protocol_event_identity(&event).unwrap_or_else(|| fw::RunId::new_v4().to_string());
        let mut frames = vec![(
            "protocol",
            serde_json::to_value(&event).map_err(|e| e.to_string())?,
        )];
        if let Some(audit) = translate_native_agui_event(key, &event) {
            frames.push((
                "audit",
                serde_json::to_value(audit).map_err(|e| e.to_string())?,
            ));
        }
        self.append(key, &identity, frames)
    }
    fn record_delivery_failure(&self, key: &str, detail: &str) {
        let mut failures = self.failures.lock().unwrap_or_else(|e| e.into_inner());
        let key = if failures.len() >= 1024 && !failures.contains_key(key) {
            ""
        } else {
            key
        };
        failures
            .entry(key.into())
            .or_insert_with(|| detail.chars().take(512).collect());
        drop(failures);
        if self.health_flush.try_send(()).is_err() {
            // Full means a flush is already queued; the retained registry is authoritative.
        }
    }
    fn replay_failure(&self, key: &str) -> Option<String> {
        let failures = self.failures.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(detail) = failures.get(key).or_else(|| failures.get("")) {
            return Some(detail.clone());
        }
        drop(failures);
        let storage = match self.storage() {
            Ok(s) => s,
            Err(e) => return Some(e),
        };
        let durable = blocking(async {
            for key in [key, ""] {
                match storage
                    .sessions
                    .read_context(HEALTH, &format!("tron.replay-failure/{key}"))
                    .await
                {
                    Ok(Some((_, value))) => return value["detail"].as_str().map(str::to_owned),
                    Ok(None) => {}
                    Err(e) => return Some(e.to_string()),
                }
            }
            None
        });
        if durable.is_some() {
            return durable;
        }
        let frames: Vec<AgentAguiProtocolEvent> = match self.replay(key, "protocol") {
            Ok(frames) => frames,
            Err(error) => return Some(error),
        };
        let mut unfinished = BTreeMap::<String, String>::new();
        for frame in frames {
            let execution = frame.data["metadata"]["framework"]["executionId"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            if execution.is_empty() {
                continue;
            }
            match frame.sse_event.as_str() {
                "RUN_STARTED" => {
                    let run = frame.data["metadata"]["framework"]["runId"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned();
                    unfinished.insert(execution, run);
                }
                "RUN_FINISHED" | "RUN_ERROR" => {
                    unfinished.remove(&execution);
                }
                _ => {}
            }
        }
        blocking(async {
            let mut ids = Vec::new();
            for run in unfinished.values() {
                if (storage.native_run_active)(run) {
                    continue;
                }
                match fw::RunId::parse_str(run) {
                    Ok(id) => ids.push(id.to_string()),
                    Err(_) => {
                        return Some("native execution has no terminal presentation event".into());
                    }
                }
            }
            if ids.is_empty() {
                return None;
            }
            let selected = match serde_json::to_string(&ids) {
                Ok(value) => value,
                Err(error) => return Some(error.to_string()),
            };
            let live=match storage.database.call_read(move |c| {
                let mut q=c.prepare("SELECT id FROM ef_runs WHERE id IN (SELECT value FROM json_each(?1)) AND status='RUNNING' AND lease_until>?2")?;
                q.query_map(rusqlite::params![selected,fw::lifecycle_time_ms()],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<std::collections::BTreeSet<_>>>()
            }).await {Ok(live)=>live,Err(error)=>return Some(error.to_string())};
            ids.iter()
                .any(|id| !live.contains(id))
                .then(|| "native execution has no terminal presentation event".into())
        })
    }

    fn replay_agui_events(&self, key: &str) -> Result<Vec<AgentAguiEvent>, String> {
        self.replay(key, "audit")
    }
    fn replay_protocol_events(&self, key: &str) -> Result<Vec<AgentAguiProtocolEvent>, String> {
        self.replay(key, "protocol")
    }
}
impl EventSink for SqliteAgentEventReplayStore {
    fn name(&self) -> &'static str {
        "framework_agui_replay"
    }
    fn accepts(&self, event: &BusEvent) -> bool {
        event.stream_key().is_some()
            && CanonicalRuntimeEvent::from_bus_event_name(&event.event_name).is_some()
            && !matches!(
                event.event_name.as_str(),
                "agent.reasoning.delta"
                    | "agent.reasoning.raw_delta"
                    | "agent.output.delta"
                    | "agent.tool.activity"
            )
    }
    fn delivery_failed(&self, event: &BusEvent, detail: &str) {
        if let Some(key) = event.stream_key() {
            self.record_delivery_failure(key, detail);
        }
    }
    fn delivery_rejected(&self, key: &str, detail: &str) {
        self.record_delivery_failure(key, detail);
    }
    fn handle(&self, event: Arc<BusEvent>) {
        if let Err(e) = self.persist(&event) {
            if let Some(key) = event.stream_key() {
                self.record_delivery_failure(key, &e);
            }
        }
    }
}
