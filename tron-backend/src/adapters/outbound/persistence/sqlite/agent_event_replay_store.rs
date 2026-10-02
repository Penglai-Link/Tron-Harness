use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        mpsc::{SyncSender, TrySendError, sync_channel},
    },
    thread,
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    application::agent::{
        AgentAguiReplayStore, CanonicalRuntimeEvent,
        agui_translate::{
            translate_agui_event, translate_native_agui_event, translate_protocol_events,
        },
        protocol_event_identity,
    },
    domain::agent::{AgentAguiEvent, AgentAguiProtocolEvent},
    shared::event_bus::{BusEvent, EventSink},
};

use super::migrations::AGENT_RUNTIME_EVENT_REPLAY_MIGRATION;

pub struct SqliteAgentEventReplayStore {
    connection: Arc<Mutex<Connection>>,
    failures: Arc<Mutex<BTreeMap<String, String>>>,
    health_flush: SyncSender<()>,
}

impl SqliteAgentEventReplayStore {
    pub fn connect(database_url: &str) -> Result<Self, String> {
        ensure_database_parent(database_url)?;
        let connection = if database_url == ":memory:" || database_url.starts_with("file:") {
            Connection::open_with_flags(
                database_url,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open(database_url)
        }
        .map_err(|error| format!("agent event replay database open failed: {error}"))?;
        connection
            .execute_batch(AGENT_RUNTIME_EVENT_REPLAY_MIGRATION)
            .map_err(|error| format!("agent event replay migration failed: {error}"))?;

        connection
            .execute_batch(include_str!(
                "../../../../../migrations/20261002_0001_create_agent_runtime_replay_health.sql"
            ))
            .map_err(|error| format!("agent event replay health migration failed: {error}"))?;

        let connection = Arc::new(Mutex::new(connection));
        let failures = Arc::new(Mutex::new(BTreeMap::new()));
        let (health_flush, signals) = sync_channel(1);
        let worker_connection = connection.clone();
        let worker_failures = failures.clone();
        thread::Builder::new()
            .name("tron-agent-replay-health".into())
            .spawn(move || {
                while signals.recv().is_ok() {
                    let attempt = (|| {
                        let mut connection = worker_connection
                            .lock()
                            .map_err(|_| "replay health connection unavailable".to_string())?;
                        let transaction = connection.transaction().map_err(|error| {
                            format!("replay health transaction failed: {error}")
                        })?;
                        write_pending_failures(&transaction, &worker_failures)?;
                        transaction
                            .commit()
                            .map_err(|error| format!("replay health commit failed: {error}"))
                    })();
                    if attempt.is_err() {
                        // The live registry remains authoritative until durable acknowledgment.
                        // No retry loop can spin indefinitely on an unavailable disk.
                        if let Ok(mut failures) = worker_failures.lock() {
                            failures.entry(String::new()).or_insert_with(|| {
                                "presentation replay health acknowledgment unavailable".into()
                            });
                        }
                    }
                }
            })
            .map_err(|error| format!("agent replay health worker failed: {error}"))?;
        Ok(Self {
            connection,
            failures,
            health_flush,
        })
    }

    fn persist(&self, event: &BusEvent) -> Result<(), String> {
        let Some(stream_key) = event.stream_key().map(str::to_string) else {
            return Ok(());
        };
        let Some(canonical) = CanonicalRuntimeEvent::from_bus_event_name(event.event_name.as_str())
        else {
            return Ok(());
        };

        let bus_event_id = event.event_id.as_decimal_string();
        let run_code = event
            .subject
            .as_ref()
            .and_then(|subject| subject.subject_code.clone());
        let created_at = chrono::Utc::now().timestamp_millis();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "agent event replay connection unavailable".to_string())?;

        let transaction = connection
            .transaction()
            .map_err(|error| format!("agent event replay transaction failed: {error}"))?;

        self.flush_failures(&transaction)?;

        if let Some(agui_event) = translate_agui_event(&stream_key, event) {
            let payload_json = serde_json::to_string(&agui_event)
                .map_err(|error| format!("agent event replay serialize agui failed: {error}"))?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO agent_runtime_event_replay (
                        bus_event_id,
                        stream_key,
                        run_code,
                        occurred_at,
                        canonical_event_code,
                        category,
                        ordinal,
                        sse_event,
                        payload_json,
                        created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 'AGUI', 0, NULL, ?6, ?7)",
                    params![
                        bus_event_id,
                        stream_key,
                        run_code,
                        event.occurred_at_ms,
                        canonical.code(),
                        payload_json,
                        created_at,
                    ],
                )
                .map_err(|error| format!("agent event replay insert agui failed: {error}"))?;
        }

        let protocol_events = translate_protocol_events(&stream_key, event);
        for (index, protocol_event) in protocol_events.into_iter().enumerate() {
            let payload_json = serde_json::to_string(&protocol_event).map_err(|error| {
                format!("agent event replay serialize protocol failed: {error}")
            })?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO agent_runtime_event_replay (
                        bus_event_id,
                        stream_key,
                        run_code,
                        occurred_at,
                        canonical_event_code,
                        category,
                        ordinal,
                        sse_event,
                        payload_json,
                        created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 'PROTOCOL', ?6, ?7, ?8, ?9)",
                    params![
                        bus_event_id,
                        stream_key,
                        run_code,
                        event.occurred_at_ms,
                        canonical.code(),
                        i64::try_from(index).unwrap_or(i64::MAX),
                        protocol_event.sse_event,
                        payload_json,
                        created_at,
                    ],
                )
                .map_err(|error| format!("agent event replay insert protocol failed: {error}"))?;
        }

        transaction
            .commit()
            .map_err(|error| format!("agent event replay commit failed: {error}"))?;

        Ok(())
    }

    fn flush_failures(&self, connection: &Connection) -> Result<(), String> {
        write_pending_failures(connection, &self.failures)
    }

    fn replay_by_category(&self, stream_key: &str, category: &str) -> Result<Vec<String>, String> {
        let stream_key = stream_key.trim();
        if stream_key.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| "agent event replay connection unavailable".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT payload_json
                 FROM agent_runtime_event_replay
                 WHERE stream_key = ?1
                   AND category = ?2
                 ORDER BY id ASC, ordinal ASC",
            )
            .map_err(|error| format!("agent event replay query prepare failed: {error}"))?;
        statement
            .query_map(params![stream_key, category], |row| row.get::<_, String>(0))
            .map_err(|error| format!("agent event replay query map failed: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("agent event replay query collect failed: {error}"))
    }

    fn persist_protocol_event(
        &self,
        stream_key: &str,
        event: AgentAguiProtocolEvent,
    ) -> Result<(), String> {
        let stream_key = stream_key.trim();
        if stream_key.is_empty() {
            return Ok(());
        }
        let payload_json = serde_json::to_string(&event)
            .map_err(|error| format!("agent event replay serialize protocol failed: {error}"))?;
        let run_code = event
            .data
            .get("runId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let occurred_at = event
            .data
            .get("timestamp")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        let bus_event_id = protocol_event_identity(&event)
            .unwrap_or_else(|| format!("host-protocol:{}", eliteforge_agent_core::RunId::new_v4()));
        let created_at = chrono::Utc::now().timestamp_millis();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "agent event replay connection unavailable".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| format!("agent event replay transaction failed: {error}"))?;
        self.flush_failures(&transaction)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO agent_runtime_event_replay (
                    bus_event_id,
                    stream_key,
                    run_code,
                    occurred_at,
                    canonical_event_code,
                    category,
                    ordinal,
                    sse_event,
                    payload_json,
                    created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'PROTOCOL', 0, ?6, ?7, ?8)",
                params![
                    bus_event_id,
                    stream_key,
                    run_code,
                    occurred_at,
                    event.sse_event,
                    event.sse_event,
                    payload_json,
                    created_at,
                ],
            )
            .map_err(|error| format!("agent event replay insert protocol failed: {error}"))?;
        if let Some(audit) = translate_native_agui_event(stream_key, &event) {
            let audit_json = serde_json::to_string(&audit)
                .map_err(|error| format!("native audit serialize failed: {error}"))?;
            transaction.execute("INSERT OR IGNORE INTO agent_runtime_event_replay (bus_event_id,stream_key,run_code,occurred_at,canonical_event_code,category,ordinal,sse_event,payload_json,created_at) VALUES (?1,?2,?3,?4,?5,'AGUI',0,NULL,?6,?7)",params![bus_event_id,stream_key,run_code,occurred_at,audit.metadata.get("canonicalEvent"),audit_json,created_at]).map_err(|error|format!("native audit insert failed: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("agent event replay commit failed: {error}"))?;
        Ok(())
    }
}

impl AgentAguiReplayStore for SqliteAgentEventReplayStore {
    fn append_protocol_event(
        &self,
        stream_key: &str,
        event: AgentAguiProtocolEvent,
    ) -> Result<(), String> {
        self.persist_protocol_event(stream_key, event)
    }

    fn record_delivery_failure(&self, stream_key: &str, detail: &str) {
        // This callback also runs on nonblocking bus handoff failures; never perform I/O here.
        if let Ok(mut failures) = self.failures.lock() {
            if failures.len() >= 1024 && !failures.contains_key(stream_key) {
                failures
                    .entry(String::new())
                    .or_insert_with(|| "presentation replay delivery failed".into());
            } else {
                failures
                    .entry(stream_key.to_owned())
                    .or_insert_with(|| detail.chars().take(512).collect());
            }
        }
        // Capacity one coalesces notifications; the bounded registry retains every
        // affected scope until the worker's SQLite transaction acknowledges it.
        match self.health_flush.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => {}
            Err(TrySendError::Disconnected(())) => {
                if let Ok(mut failures) = self.failures.lock() {
                    failures.entry(String::new()).or_insert_with(|| {
                        "presentation replay health acknowledgment unavailable".into()
                    });
                }
            }
        }
    }

    fn replay_failure(&self, stream_key: &str) -> Option<String> {
        let memory = match self.failures.lock() {
            Ok(failures) => failures
                .get(stream_key)
                .or_else(|| failures.get(""))
                .cloned(),
            Err(_) => return Some("presentation replay diagnostics unavailable".into()),
        };
        if memory.is_some() {
            return memory;
        }
        let Ok(connection) = self.connection.lock() else {
            return Some("presentation replay diagnostics unavailable".into());
        };
        match connection.query_row(
            "SELECT detail FROM agent_runtime_event_replay_failure WHERE stream_key = ?1 OR stream_key = '' LIMIT 1",
            [stream_key], |row| row.get::<_, String>(0),
        ).optional() {
            Ok(failure) => failure,
            Err(_) => Some("presentation replay diagnostics unavailable".into()),
        }
    }

    fn replay_agui_events(&self, stream_key: &str) -> Result<Vec<AgentAguiEvent>, String> {
        self.replay_by_category(stream_key, "AGUI")?
            .into_iter()
            .map(|payload| {
                serde_json::from_str::<AgentAguiEvent>(&payload)
                    .map_err(|error| format!("agent event replay deserialize agui failed: {error}"))
            })
            .collect()
    }

    fn replay_protocol_events(
        &self,
        stream_key: &str,
    ) -> Result<Vec<AgentAguiProtocolEvent>, String> {
        self.replay_by_category(stream_key, "PROTOCOL")?
            .into_iter()
            .map(|payload| {
                serde_json::from_str::<AgentAguiProtocolEvent>(&payload).map_err(|error| {
                    format!("agent event replay deserialize protocol failed: {error}")
                })
            })
            .collect()
    }
}

impl EventSink for SqliteAgentEventReplayStore {
    fn name(&self) -> &'static str {
        "agent_event_replay_store"
    }

    fn accepts(&self, event: &BusEvent) -> bool {
        event.stream_key().is_some()
            && CanonicalRuntimeEvent::from_bus_event_name(event.event_name.as_str()).is_some()
    }

    fn delivery_failed(&self, event: &BusEvent, detail: &str) {
        if let Some(stream_key) = event.stream_key() {
            self.record_delivery_failure(stream_key, detail);
        }
    }

    fn delivery_rejected(&self, stream_key: &str, detail: &str) {
        self.record_delivery_failure(stream_key, detail);
    }

    fn handle(&self, event: std::sync::Arc<BusEvent>) {
        if let Err(error) = self.persist(event.as_ref()) {
            if let Some(stream_key) = event.stream_key() {
                self.record_delivery_failure(stream_key, &error);
            }
        }
    }
}

fn ensure_database_parent(database_url: &str) -> Result<(), String> {
    if database_url == ":memory:" || database_url.starts_with("file:") {
        return Ok(());
    }
    if let Some(parent) = Path::new(database_url).parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("agent event replay database directory failed: {error}"))?;
    }
    Ok(())
}

fn write_pending_failures(
    connection: &Connection,
    failures: &Mutex<BTreeMap<String, String>>,
) -> Result<(), String> {
    // Never hold the diagnostics mutex during database I/O: failure feedback also
    // runs on the nonblocking event-bus publication path.
    let keys = failures
        .lock()
        .map_err(|_| "presentation replay diagnostics unavailable".to_string())?
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for stream_key in keys {
        connection.execute(
            "INSERT OR IGNORE INTO agent_runtime_event_replay_failure(stream_key, detail, created_at) VALUES (?1, ?2, ?3)",
            params![stream_key,"presentation replay delivery incomplete",chrono::Utc::now().timestamp_millis()],
        ).map_err(|error|format!("agent event replay health write failed: {error}"))?;
    }
    Ok(())
}
