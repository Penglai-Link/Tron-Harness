use std::{fs, path::Path, sync::Mutex};

use rusqlite::{Connection, params};

use crate::{
    application::agent::{
        AgentAguiReplayStore, CanonicalRuntimeEvent,
        agui_translate::{translate_agui_event, translate_protocol_events},
    },
    domain::agent::{AgentAguiEvent, AgentAguiProtocolEvent},
    shared::event_bus::{BusEvent, EventSink},
};

use super::migrations::AGENT_RUNTIME_EVENT_REPLAY_MIGRATION;

pub struct SqliteAgentEventReplayStore {
    connection: Mutex<Connection>,
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

        Ok(Self {
            connection: Mutex::new(connection),
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
        let connection = self
            .connection
            .lock()
            .expect("agent event replay connection mutex poisoned");

        if let Some(agui_event) = translate_agui_event(&stream_key, event) {
            let payload_json = serde_json::to_string(&agui_event)
                .map_err(|error| format!("agent event replay serialize agui failed: {error}"))?;
            connection
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
            connection
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

        Ok(())
    }

    fn replay_by_category(&self, stream_key: &str, category: &str) -> Result<Vec<String>, String> {
        let stream_key = stream_key.trim();
        if stream_key.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self
            .connection
            .lock()
            .expect("agent event replay connection mutex poisoned");
        let mut statement = connection
            .prepare(
                "SELECT payload_json
                 FROM agent_runtime_event_replay
                 WHERE stream_key = ?1
                   AND category = ?2
                 ORDER BY occurred_at ASC, id ASC, ordinal ASC",
            )
            .map_err(|error| format!("agent event replay query prepare failed: {error}"))?;
        statement
            .query_map(params![stream_key, category], |row| row.get::<_, String>(0))
            .map_err(|error| format!("agent event replay query map failed: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("agent event replay query collect failed: {error}"))
    }
}

impl AgentAguiReplayStore for SqliteAgentEventReplayStore {
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

    fn handle(&self, event: std::sync::Arc<BusEvent>) {
        if let Err(error) = self.persist(event.as_ref()) {
            eprintln!("agent event replay persistence skipped: {error}");
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
