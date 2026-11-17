use super::framework_storage::{SharedDatabase, business_transaction};
use crate::application::assistant::AssistantRepository;
use crate::domain::assistant::*;
use rusqlite::{OptionalExtension, params};

#[derive(Clone)]
pub struct SqliteAssistantRepository {
    database: SharedDatabase,
}
impl SqliteAssistantRepository {
    pub fn connect(path: &str) -> Result<Self, AssistantError> {
        Ok(Self {
            database: SharedDatabase::open(path).map_err(storage)?,
        })
    }
    fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut rusqlite::Connection) -> Result<T, AssistantError> + Send + 'static,
    ) -> Result<T, AssistantError> {
        self.database.call(f, || AssistantError::Storage)
    }
    fn retry_event(
        &self,
        event: &AssistantEvent,
        now: i64,
        delay_ms: i64,
        error: &str,
        busy: bool,
    ) -> Result<(), AssistantError> {
        let event = event.clone();
        let error = error.to_owned();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let json: Option<String> = tx.query_row(
                "SELECT record_json FROM assistant_events WHERE event_id=?1 AND lease_token=?2 AND status='Running' AND lease_until>=?3",
                params![event.event_id, event.lease_token, now], |r| r.get(0),
            ).optional().map_err(storage)?;
            let mut record: AssistantEvent = decode(json.ok_or(AssistantError::Conflict)?)?;
            record.status = AssistantEventStatus::Queued;
            record.available_at = now + delay_ms;
            record.lease_until = None;
            record.lease_token = None;
            record.result = Some(error);
            if busy {
                record.attempts = record.attempts.saturating_sub(1);
            }
            tx.execute(
                "UPDATE assistant_events SET status='Queued',available_at=?2,lease_until=NULL,lease_token=NULL,attempts=?3,record_json=?4 WHERE event_id=?1",
                params![record.event_id, record.available_at, record.attempts, encode(&record)?],
            ).map_err(storage)?;
            tx.commit().map_err(storage)
        })
    }
}
// Validate against the same transaction that stores the assessment or delivery.
fn validate_work_snapshot(
    connection: &rusqlite::Connection,
    change: &AssistantWorkChange,
) -> Result<(), AssistantError> {
    let state: Option<(String, u8, Option<String>)> = connection.query_row(
        "SELECT status,progress_value,execution_result FROM tasks WHERE task_code=?1 AND owner_assistant_code=?2 AND archived_at IS NULL",
        params![change.task_code, change.assistant_code],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(storage)?;
    let (status, progress, result) = state.ok_or(AssistantError::NotFound)?;
    if change.kind.actionable() {
        if status == crate::domain::task::CLOSED_STATUS_CODE {
            return Err(AssistantError::Conflict);
        }
    } else if change.kind == WorkChangeKind::Progress {
        if status == crate::domain::task::CLOSED_STATUS_CODE
            || result.as_deref().map(str::trim)
                != change.task_snapshot["executionResult"]
                    .as_str()
                    .map(str::trim)
        {
            return Err(AssistantError::Conflict);
        }
    } else if serde_json::json!({"status":status,"progress":progress,"executionResult":result})
        != change.task_snapshot
    {
        return Err(AssistantError::Conflict);
    }
    if change.kind == WorkChangeKind::Completed {
        let unfinished: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE parent_task_code=?1 AND status!=?2 AND archived_at IS NULL)",
            params![change.task_code, crate::domain::task::CLOSED_STATUS_CODE], |row| row.get(0),
        ).map_err(storage)?;
        if unfinished {
            return Err(AssistantError::Conflict);
        }
    }
    Ok(())
}
fn storage(e: rusqlite::Error) -> AssistantError {
    tracing::error!("PersonAI SQLite: {e}");
    AssistantError::Storage
}
fn encode<T: serde::Serialize>(v: &T) -> Result<String, AssistantError> {
    serde_json::to_string(v).map_err(|_| AssistantError::Storage)
}
fn decode<T: serde::de::DeserializeOwned>(s: String) -> Result<T, AssistantError> {
    serde_json::from_str(&s).map_err(|_| AssistantError::Storage)
}
fn load_events(
    c: &rusqlite::Connection,
    sql: &str,
    code: &str,
) -> Result<Vec<AssistantEvent>, AssistantError> {
    let mut q = c.prepare(sql).map_err(storage)?;
    q.query_map([code], |r| r.get::<_, String>(0))
        .map_err(storage)?
        .map(|r| decode(r.map_err(storage)?))
        .collect()
}

fn prune_completed_heartbeat_events(
    connection: &rusqlite::Connection,
    assistant_code: Option<&str>,
) -> Result<usize, AssistantError> {
    connection
        .execute(
            "DELETE FROM assistant_events
             WHERE source='Heartbeat' AND status IN ('Completed','Failed')
               AND (?1 IS NULL OR assistant_code=?1)
               AND EXISTS (
                   SELECT 1 FROM assistant_events newer
                   WHERE newer.assistant_code=assistant_events.assistant_code
                     AND newer.source='Heartbeat' AND newer.rowid>assistant_events.rowid
                   LIMIT 1 OFFSET ?2
               )",
            params![assistant_code, (HEARTBEAT_HISTORY_LIMIT - 1) as i64],
        )
        .map_err(storage)
}

impl AssistantRepository for SqliteAssistantRepository {
    fn soul(&mut self, code: &str) -> Result<AssistantSoul, AssistantError> {
        let code = code.to_owned();
        self.call(move |c| {
            let json: Option<String> = c.query_row("SELECT s.record_json FROM assistant_souls s JOIN assistants d USING(assistant_code) WHERE s.assistant_code=?1", [code], |r| r.get(0)).optional().map_err(storage)?;
            let mut soul: AssistantSoul = decode(json.ok_or(AssistantError::NotFound)?)?;
            if soul.state != crate::domain::assistant::SoulState::Ready {
                if !soul.name_confirmed && soul.name == "未命名" { soul.name = "PersonAI".into(); }
                if soul.markdown.trim().is_empty() {
                    soul.markdown = AssistantSoul::new(&soul.assistant_code, &soul.name).markdown;
                }
                soul.state = crate::domain::assistant::SoulState::Ready;
                soul.name_confirmed = true;
                soul.pending_interaction_id = None;
                c.execute("UPDATE assistant_souls SET record_json=?2 WHERE assistant_code=?1", params![soul.assistant_code, encode(&soul)?]).map_err(storage)?;
            }
            Ok(soul)
        })
    }
    fn save_soul(&mut self, soul: &AssistantSoul) -> Result<(), AssistantError> {
        let soul = soul.clone();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate)
                .map_err(storage)?;
            tx.execute(
                "UPDATE assistant_souls SET record_json=?2 WHERE assistant_code=?1",
                params![soul.assistant_code, encode(&soul)?],
            )
            .map_err(storage)?;
            tx.commit().map_err(storage)
        })
    }

    fn create(
        &mut self,
        d: &Assistant,
        definition: &crate::domain::agent::PlatformInternalAgentConfig,
    ) -> Result<(), AssistantError> {
        let (d, definition) = (d.clone(), definition.clone());
        self.call(move|c|{
            let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            tx.execute("INSERT INTO assistant_souls(assistant_code,record_json) VALUES(?1,?2)",params![d.assistant_code,encode(&AssistantSoul::new(&d.assistant_code,&definition.name))?]).map_err(storage)?;
            tx.execute("INSERT INTO platform_internal_agent_configs(agent_code,config_json) VALUES(?1,?2)",params![definition.agent_code,encode(&definition)?]).map_err(storage)?;
            tx.execute("INSERT INTO agent_workspace_grant_policies(agent_code) VALUES(?1)",[&d.assistant_code]).map_err(storage)?;
            tx.execute("INSERT INTO assistants(assistant_code,record_json,enabled,heartbeat_minutes,next_heartbeat_at,is_main) VALUES(?1,?2,?3,?4,?5,0)",params![d.assistant_code,encode(&d)?,d.enabled,d.heartbeat_minutes,d.created_at+i64::from(d.heartbeat_minutes)*60000]).map_err(storage)?;
            tx.commit().map_err(storage)
        })
    }

    fn list(&mut self) -> Result<Vec<Assistant>, AssistantError> {
        self.call(|c| {
            let mut q = c
                .prepare("SELECT record_json,is_main FROM assistants ORDER BY rowid")
                .map_err(storage)?;
            q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)))
                .map_err(storage)?
                .map(|r| {
                    let (json, main) = r.map_err(storage)?;
                    let mut d: Assistant = decode(json)?;
                    d.is_main = main;
                    Ok(d)
                })
                .collect()
        })
    }
    fn save(&mut self, d: &Assistant) -> Result<(), AssistantError> {
        let d = d.clone();
        self.call(move|c|{c.execute("INSERT INTO assistants(assistant_code,record_json,enabled,heartbeat_minutes,next_heartbeat_at,is_main) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(assistant_code) DO UPDATE SET record_json=excluded.record_json,enabled=excluded.enabled,heartbeat_minutes=excluded.heartbeat_minutes,next_heartbeat_at=CASE WHEN assistants.heartbeat_minutes!=excluded.heartbeat_minutes OR (assistants.enabled=0 AND excluded.enabled=1) THEN excluded.next_heartbeat_at ELSE assistants.next_heartbeat_at END",params![d.assistant_code,encode(&d)?,d.enabled,d.heartbeat_minutes,d.updated_at+i64::from(d.heartbeat_minutes)*60000,d.is_main]).map_err(storage)?;Ok(())})
    }
    fn delete(&mut self, code: &str) -> Result<(), AssistantError> {
        let code = code.to_owned();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate)
                .map_err(storage)?;
            let is_main: Option<bool> = tx
                .query_row("SELECT is_main FROM assistants WHERE assistant_code=?1", [&code], |r| r.get(0))
                .optional()
                .map_err(storage)?;
            if is_main.ok_or(AssistantError::NotFound)? {
                return Err(AssistantError::Conflict);
            }
            let busy: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM assistant_events WHERE assistant_code=?1 AND status IN ('Queued','Running','AwaitingInput'))",
                [&code], |r| r.get(0),
            ).map_err(storage)?;
            if busy {
                return Err(AssistantError::Conflict);
            }
            super::agent_management_repository::delete_platform_internal_agent_records(&tx, &code)
                .map_err(storage)?;
            tx.execute("DELETE FROM assistant_events WHERE assistant_code=?1", [&code]).map_err(storage)?;
            tx.execute("DELETE FROM assistant_sessions WHERE assistant_code=?1", [&code]).map_err(storage)?;
            tx.execute("DELETE FROM assistant_souls WHERE assistant_code=?1", [&code]).map_err(storage)?;
            tx.execute("DELETE FROM assistants WHERE assistant_code=?1", [&code]).map_err(storage)?;
            tx.commit().map_err(storage)
        })
    }
    fn designate_main(&mut self, code: &str) -> Result<(), AssistantError> {
        let code = code.to_owned();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate)
                .map_err(storage)?;
            let current: Option<String> = tx
                .query_row(
                    "SELECT assistant_code FROM assistants WHERE is_main=1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            if let Some(current) = current {
                return if current == code {
                    Ok(())
                } else {
                    Err(AssistantError::Conflict)
                };
            }
            if tx
                .execute(
                    "UPDATE assistants SET is_main=1 WHERE assistant_code=?1",
                    [code],
                )
                .map_err(storage)?
                != 1
            {
                return Err(AssistantError::NotFound);
            }
            tx.commit().map_err(storage)
        })
    }
    fn resolve_session(
        &mut self,
        s: &AssistantSession,
    ) -> Result<AssistantSession, AssistantError> {
        let s = s.clone();
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;tx.execute("INSERT INTO assistant_sessions(session_code,assistant_code,purpose,correlation_key,record_json) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(assistant_code,purpose,correlation_key) DO NOTHING",params![s.session_code,s.assistant_code,format!("{:?}",s.purpose),s.correlation_key,encode(&s)?]).map_err(storage)?;let json=tx.query_row("SELECT record_json FROM assistant_sessions WHERE assistant_code=?1 AND purpose=?2 AND correlation_key=?3",params![s.assistant_code,format!("{:?}",s.purpose),s.correlation_key],|r|r.get(0)).map_err(storage)?;tx.commit().map_err(storage)?;decode(json)})
    }
    fn sessions(&mut self, code: &str) -> Result<Vec<AssistantSession>, AssistantError> {
        let code = code.to_owned();
        self.call(move |c| {
            let mut q = c
                .prepare(
                    "SELECT record_json FROM assistant_sessions WHERE assistant_code=?1 ORDER BY rowid DESC",
                )
                .map_err(storage)?;
            q.query_map([code], |r| r.get::<_, String>(0))
                .map_err(storage)?
                .map(|r| decode(r.map_err(storage)?))
                .collect()
        })
    }
    fn expired_heartbeat_sessions(&mut self) -> Result<Vec<String>, AssistantError> {
        self.call(|c| {
            let mut query = c.prepare(
                "SELECT s.session_code FROM assistant_sessions s
                 WHERE s.purpose='Heartbeat'
                   AND EXISTS (SELECT 1 FROM assistant_sessions newer
                        WHERE newer.assistant_code=s.assistant_code AND newer.purpose='Heartbeat'
                          AND newer.rowid>s.rowid LIMIT 1 OFFSET 19)
                   AND NOT EXISTS (SELECT 1 FROM assistant_events e WHERE (e.session_code=s.session_code OR e.event_id=s.correlation_key)
                                   AND (e.source!='Heartbeat' OR e.status IN ('Queued','Running','AwaitingInput')))
                   AND NOT EXISTS (SELECT 1 FROM tasks t WHERE t.work_session_code=s.session_code)
                   AND NOT EXISTS (SELECT 1 FROM collaboration_threads t WHERE t.session_id=s.session_code)
                   AND NOT EXISTS (SELECT 1 FROM assistant_work_changes w WHERE json_extract(w.record_json,'$.sessionCode')=s.session_code AND json_extract(w.record_json,'$.resolvedAt') IS NULL)
                   AND (EXISTS (SELECT 1 FROM workspace_session_links l WHERE l.session_id=s.session_code)
                        OR (NOT EXISTS (SELECT 1 FROM ef_sessions f WHERE f.id=s.session_code)
                            AND NOT EXISTS (SELECT 1 FROM ef_runs r WHERE r.session_id=s.session_code OR r.owner_session_id=s.session_code)
                            AND NOT EXISTS (SELECT 1 FROM ef_events e WHERE e.session_id=s.session_code)
                            AND NOT EXISTS (SELECT 1 FROM ef_conversations c WHERE c.session=s.session_code)
                            AND NOT EXISTS (SELECT 1 FROM ef_checkpoints c WHERE c.session_id=s.session_code)
                            AND NOT EXISTS (SELECT 1 FROM ef_history_runs h WHERE h.session_id=s.session_code)))
                 ORDER BY s.rowid LIMIT 50"
            ).map_err(storage)?;
            let rows = query.query_map([], |row| row.get(0)).map_err(storage)?
                .collect::<Result<Vec<String>, _>>().map_err(storage)?;
            Ok(rows)
        })
    }
    fn delete_empty_heartbeat_session(&mut self, code: &str) -> Result<(), AssistantError> {
        let code = code.to_owned();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let eligible: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM assistant_sessions s WHERE s.session_code=?1 AND s.purpose='Heartbeat'
                 AND EXISTS(SELECT 1 FROM assistant_sessions n WHERE n.assistant_code=s.assistant_code AND n.purpose='Heartbeat' AND n.rowid>s.rowid LIMIT 1 OFFSET 19)
                 AND NOT EXISTS(SELECT 1 FROM workspace_session_links l WHERE l.session_id=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM ef_sessions f WHERE f.id=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM ef_runs r WHERE r.session_id=s.session_code OR r.owner_session_id=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM ef_events e WHERE e.session_id=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM ef_conversations c WHERE c.session=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM ef_checkpoints c WHERE c.session_id=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM ef_history_runs h WHERE h.session_id=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM assistant_events e WHERE (e.session_code=s.session_code OR e.event_id=s.correlation_key) AND (e.source!='Heartbeat' OR e.status IN ('Queued','Running','AwaitingInput')))
                 AND NOT EXISTS(SELECT 1 FROM tasks t WHERE t.work_session_code=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM collaboration_threads t WHERE t.session_id=s.session_code)
                 AND NOT EXISTS(SELECT 1 FROM assistant_work_changes w WHERE json_extract(w.record_json,'$.sessionCode')=s.session_code AND json_extract(w.record_json,'$.resolvedAt') IS NULL))", [&code], |r| r.get(0)
            ).map_err(storage)?;
            if eligible {
                tx.execute("DELETE FROM assistant_events WHERE source='Heartbeat' AND EXISTS(SELECT 1 FROM assistant_sessions s WHERE s.session_code=?1 AND (assistant_events.session_code=s.session_code OR assistant_events.event_id=s.correlation_key))", [&code]).map_err(storage)?;
                tx.execute("DELETE FROM assistant_sessions WHERE session_code=?1 AND purpose='Heartbeat'", [&code]).map_err(storage)?;
            }
            tx.commit().map_err(storage)
        })
    }
    fn session(&mut self, code: &str) -> Result<Option<AssistantSession>, AssistantError> {
        let code = code.to_owned();
        self.call(move |c| {
            c.query_row(
                "SELECT record_json FROM assistant_sessions WHERE session_code=?1",
                [code],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?
            .map(decode)
            .transpose()
        })
    }
    fn enqueue(&mut self, e: &AssistantEvent) -> Result<AssistantEvent, AssistantError> {
        let e = e.clone();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            tx.execute("INSERT INTO assistant_events(event_id,assistant_code,dedup_key,source,session_code,status,available_at,record_json) VALUES(?1,?2,?3,?4,?5,'Queued',?6,?7) ON CONFLICT(dedup_key) DO NOTHING",params![e.event_id,e.assistant_code,e.dedup_key,format!("{:?}",e.source),e.session_code,e.available_at,encode(&e)?]).map_err(storage)?;
            let record = tx.query_row("SELECT record_json FROM assistant_events WHERE dedup_key=?1", [&e.dedup_key], |r| r.get::<_, String>(0)).map_err(storage)?;
            let result = decode(record)?;
            if e.source == AssistantEventSource::Heartbeat {
                prune_completed_heartbeat_events(&tx, Some(&e.assistant_code))?;
            }
            tx.commit().map_err(storage)?;
            Ok(result)
        })
    }
    fn events(&mut self, code: &str) -> Result<Vec<AssistantEvent>, AssistantError> {
        let code = code.to_owned();
        self.call(move|c|load_events(c,"SELECT record_json FROM assistant_events WHERE assistant_code=?1 ORDER BY rowid DESC LIMIT 200",&code))
    }
    fn activity_events(&mut self, code: &str) -> Result<Vec<AssistantEvent>, AssistantError> {
        let code = code.to_owned();
        self.call(move |c| {
            let mut query = c
                .prepare(
                    "SELECT event.record_json FROM assistant_events event
                 WHERE event.assistant_code=?1 AND (event.source!='Heartbeat' OR event.rowid IN (
                     SELECT heartbeat.rowid FROM assistant_events heartbeat
                     WHERE heartbeat.assistant_code=?1 AND heartbeat.source='Heartbeat'
                     ORDER BY heartbeat.rowid DESC LIMIT ?2
                 ))
                 ORDER BY event.rowid DESC LIMIT 200",
                )
                .map_err(storage)?;
            query
                .query_map(params![code, HEARTBEAT_HISTORY_LIMIT as i64], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(storage)?
                .map(|row| decode(row.map_err(storage)?))
                .collect()
        })
    }
    fn prune_heartbeat_events(&mut self) -> Result<usize, AssistantError> {
        self.call(|c| prune_completed_heartbeat_events(c, None))
    }
    fn claim(&mut self, now: i64, lease_ms: i64) -> Result<Option<AssistantEvent>, AssistantError> {
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
 let json:Option<String>=tx.query_row("SELECT e.record_json FROM assistant_events e JOIN assistants d USING(assistant_code) WHERE (d.enabled=1 OR e.source='UserMessage') AND e.available_at<=?1 AND (e.status='Queued' OR (e.status='Running' AND e.lease_until<?1)) AND NOT EXISTS(SELECT 1 FROM assistant_events busy WHERE busy.session_code=e.session_code AND e.session_code IS NOT NULL AND ((busy.status='Running' AND busy.lease_until>=?1) OR (busy.status='AwaitingInput' AND e.source!='UserMessage'))) AND NOT EXISTS(SELECT 1 FROM assistant_events busy WHERE json_extract(d.record_json,'$.kind')='Memory' AND e.source!='UserMessage' AND busy.assistant_code=e.assistant_code AND busy.event_id!=e.event_id AND busy.status='Running' AND busy.lease_until>=?1 AND busy.source!='UserMessage') ORDER BY e.available_at,e.rowid LIMIT 1",[now],|r|r.get(0)).optional().map_err(storage)?;
 let Some(json)=json else {return Ok(None)};let mut e:AssistantEvent=decode(json)?;e.status=AssistantEventStatus::Running;e.lease_token=Some(eliteforge_agent_core::SessionRecord::new("").id);e.lease_until=Some(now+lease_ms.max(1000));e.attempts+=1;
 tx.execute("UPDATE assistant_events SET status='Running',lease_until=?2,lease_token=?3,attempts=?4,record_json=?5 WHERE event_id=?1",params![e.event_id,e.lease_until,e.lease_token,e.attempts,encode(&e)?]).map_err(storage)?;tx.commit().map_err(storage)?;Ok(Some(e))})
    }
    fn finish(
        &mut self,
        id: &str,
        token: &str,
        status: AssistantEventStatus,
        result: &str,
        now: i64,
    ) -> Result<(), AssistantError> {
        let (id, token, result) = (id.to_owned(), token.to_owned(), result.to_owned());
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;let json:Option<String>=tx.query_row("SELECT record_json FROM assistant_events WHERE event_id=?1 AND lease_token=?2 AND status='Running' AND lease_until>=?3",params![id,token,now],|r|r.get(0)).optional().map_err(storage)?;let mut e:AssistantEvent=decode(json.ok_or(AssistantError::Conflict)?)?;e.status=status;e.result=Some(result);e.lease_until=None;e.lease_token=None;if tx.execute("UPDATE assistant_events SET status=?3,lease_until=NULL,lease_token=NULL,record_json=?4 WHERE event_id=?1 AND lease_token=?2",params![id,token,format!("{status:?}"),encode(&e)?]).map_err(storage)?!=1{return Err(AssistantError::Conflict)}
if e.source == AssistantEventSource::Heartbeat && matches!(status, AssistantEventStatus::Completed | AssistantEventStatus::Failed) { prune_completed_heartbeat_events(&tx, Some(&e.assistant_code))?; }
tx.commit().map_err(storage)})
    }
    fn attach_session(
        &mut self,
        id: &str,
        token: &str,
        session: &str,
    ) -> Result<(), AssistantError> {
        let (id, token, session) = (id.to_owned(), token.to_owned(), session.to_owned());
        self.call(move|c|{let json:Option<String>=c.query_row("SELECT record_json FROM assistant_events WHERE event_id=?1 AND lease_token=?2 AND status='Running'",params![id,token],|r|r.get(0)).optional().map_err(storage)?;let mut e:AssistantEvent=decode(json.ok_or(AssistantError::Conflict)?)?;e.session_code=Some(session.clone());c.execute("UPDATE assistant_events SET session_code=?3,record_json=?4 WHERE event_id=?1 AND lease_token=?2",params![id,token,session,encode(&e)?]).map_err(storage)?;Ok(())})
    }
    fn renew(
        &mut self,
        event: &AssistantEvent,
        now: i64,
        lease_ms: i64,
    ) -> Result<(), AssistantError> {
        let event = event.clone();
        self.call(move |c| {
            let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let json:Option<String>=tx.query_row("SELECT record_json FROM assistant_events WHERE event_id=?1 AND lease_token=?2 AND status='Running' AND lease_until>=?3",params![event.event_id,event.lease_token,now],|r|r.get(0)).optional().map_err(storage)?;
            let mut record:AssistantEvent=decode(json.ok_or(AssistantError::Conflict)?)?;
            record.lease_until=Some(now+lease_ms);
            tx.execute("UPDATE assistant_events SET lease_until=?2,record_json=?3 WHERE event_id=?1",params![record.event_id,record.lease_until,encode(&record)?]).map_err(storage)?;
            tx.commit().map_err(storage)
        })
    }
    fn retry(
        &mut self,
        event: &AssistantEvent,
        now: i64,
        delay_ms: i64,
        error: &str,
    ) -> Result<(), AssistantError> {
        self.retry_event(event, now, delay_ms, error, false)
    }
    fn defer_busy(
        &mut self,
        event: &AssistantEvent,
        now: i64,
        delay_ms: i64,
    ) -> Result<(), AssistantError> {
        self.retry_event(
            event,
            now,
            delay_ms,
            "Waiting for the current PersonAI run",
            true,
        )
    }
    fn resolve_awaiting_input(
        &mut self,
        session: &str,
        summary: &str,
        _now: i64,
    ) -> Result<(), AssistantError> {
        let session = session.to_owned();
        let summary = summary.to_owned();
        self.call(move|c|{
            let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let rows={let mut q=tx.prepare("SELECT record_json FROM assistant_events WHERE session_code=?1 AND status='AwaitingInput'").map_err(storage)?;q.query_map([&session],|r|r.get::<_,String>(0)).map_err(storage)?.collect::<Result<Vec<_>,_>>().map_err(storage)?};
            for json in rows {
                let mut event:AssistantEvent=decode(json)?;event.status=AssistantEventStatus::Completed;event.result=Some(summary.clone());
                tx.execute("UPDATE assistant_events SET status='Completed',record_json=?2 WHERE event_id=?1",params![event.event_id,encode(&event)?]).map_err(storage)?;
                if event.source == AssistantEventSource::Heartbeat { prune_completed_heartbeat_events(&tx, Some(&event.assistant_code))?; }
            }
            tx.commit().map_err(storage)
        })
    }
    fn schedule_heartbeats(&mut self, now: i64) -> Result<(), AssistantError> {
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;let rows={let mut q=tx.prepare("SELECT assistant_code,heartbeat_minutes,next_heartbeat_at FROM assistants WHERE enabled=1 AND next_heartbeat_at<=?1").map_err(storage)?;q.query_map([now],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?,r.get::<_,i64>(2)?))).map_err(storage)?.collect::<Result<Vec<_>,_>>().map_err(storage)?};
 for(code,cadence,scheduled)in rows{let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM assistant_events WHERE assistant_code=?1 AND source='Heartbeat' AND status IN ('Queued','Running'))",[&code],|r|r.get(0)).map_err(storage)?;
 if !busy{let e=AssistantEvent{event_id:eliteforge_agent_core::SessionRecord::new("").id,assistant_code:code.clone(),source:AssistantEventSource::Heartbeat,dedup_key:format!("heartbeat:{code}:{scheduled}"),session_code:None,task_code:None,workspace_code:None,payload:serde_json::json!({"scheduledAt":scheduled}),status:AssistantEventStatus::Queued,available_at:now,lease_until:None,lease_token:None,attempts:0,result:None,created_at:now};tx.execute("INSERT INTO assistant_events(event_id,assistant_code,dedup_key,source,status,available_at,record_json) VALUES(?1,?2,?3,'Heartbeat','Queued',?4,?5)",params![e.event_id,code,e.dedup_key,now,encode(&e)?]).map_err(storage)?;prune_completed_heartbeat_events(&tx,Some(&code))?;}
 tx.execute("UPDATE assistants SET next_heartbeat_at=?2 WHERE assistant_code=?1",params![code,now+i64::from(cadence)*60000]).map_err(storage)?;}
 tx.commit().map_err(storage)})
    }
    fn notify(&mut self, n: &PlatformNotification) -> Result<PlatformNotification, AssistantError> {
        let n = n.clone();
        self.call(move|c|{c.execute("INSERT INTO platform_notifications(notification_id,dedup_key,record_json,created_at) VALUES(?1,?2,?3,?4) ON CONFLICT(dedup_key) DO NOTHING",params![n.id,n.dedup_key,encode(&n)?,n.created_at]).map_err(storage)?;decode(c.query_row("SELECT record_json FROM platform_notifications WHERE dedup_key=?1",[n.dedup_key],|r|r.get(0)).map_err(storage)?)})
    }
    fn notify_replacing(
        &mut self,
        notification: &PlatformNotification,
        prior_kinds: &[String],
    ) -> Result<PlatformNotification, AssistantError> {
        let (notification, prior_kinds) = (notification.clone(), prior_kinds.to_vec());
        self.call(move |connection| {
            let tx = business_transaction(connection, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            if let Some(candidate) = notification.target["candidateCode"].as_str() {
                let prior = {
                    let mut query = tx.prepare("SELECT record_json FROM platform_notifications WHERE json_extract(record_json,'$.target.candidateCode')=?1").map_err(storage)?;
                    query.query_map([candidate], |row| row.get::<_,String>(0)).map_err(storage)?
                        .collect::<Result<Vec<_>,_>>().map_err(storage)?
                };
                for json in prior {
                    let mut old: PlatformNotification = decode(json)?;
                    if old.category == "Skill" && old.assistant_code == notification.assistant_code
                        && old.dedup_key != notification.dedup_key && old.resolved_at.is_none()
                        && prior_kinds.contains(&old.title) {
                        old.resolved_at = Some(notification.created_at);
                        tx.execute("UPDATE platform_notifications SET record_json=?2 WHERE notification_id=?1",params![old.id,encode(&old)?]).map_err(storage)?;
                    }
                }
            }
            tx.execute("INSERT INTO platform_notifications(notification_id,dedup_key,record_json,created_at) VALUES(?1,?2,?3,?4) ON CONFLICT(dedup_key) DO NOTHING",params![notification.id,notification.dedup_key,encode(&notification)?,notification.created_at]).map_err(storage)?;
            let result = decode(tx.query_row("SELECT record_json FROM platform_notifications WHERE dedup_key=?1",[&notification.dedup_key],|row|row.get(0)).map_err(storage)?)?;
            tx.commit().map_err(storage)?;
            Ok(result)
        })
    }

    fn record_work_change(
        &mut self,
        change: &AssistantWorkChange,
        fingerprint: &str,
    ) -> Result<AssistantWorkChange, AssistantError> {
        let change = change.clone();
        let fingerprint = fingerprint.to_owned();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let previous: Option<(String, String)> = tx.query_row(
                "SELECT fingerprint,record_json FROM assistant_work_changes WHERE assistant_code=?1 AND task_code=?2 AND change_key=?3",
                params![change.assistant_code,change.task_code,change.change_key], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage)?;
            if let Some((old_fingerprint, json)) = previous {
                if old_fingerprint != fingerprint { return Err(AssistantError::Conflict); }
                return decode(json);
            }
            let identical: Option<String> = tx.query_row("SELECT record_json FROM assistant_work_changes WHERE assistant_code=?1 AND task_code=?2 AND fingerprint=?3",
                params![change.assistant_code,change.task_code,fingerprint], |r| r.get(0)).optional().map_err(storage)?;
            if let Some(json) = identical { return decode(json); }
            validate_work_snapshot(&tx, &change)?;
            if change.kind == WorkChangeKind::Completed {
                let unresolved: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM assistant_work_changes WHERE assistant_code=?1 AND task_code=?2 AND json_extract(record_json,'$.kind') IN ('NeedsDecision','Blocked') AND json_extract(record_json,'$.resolvedAt') IS NULL)", params![change.assistant_code,change.task_code], |r| r.get(0)).map_err(storage)?;
                if unresolved { return Err(AssistantError::Conflict); }
            }
            tx.execute("INSERT INTO assistant_work_changes(change_id,assistant_code,task_code,change_key,fingerprint,record_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![change.change_id,change.assistant_code,change.task_code,change.change_key,fingerprint,encode(&change)?,change.created_at]).map_err(storage)?;
            tx.commit().map_err(storage)?; Ok(change)
        })
    }
    fn scoped_pending_work_changes(
        &mut self,
        assistant: &str,
        user: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError> {
        let (assistant, user) = (assistant.to_owned(), user.to_owned());
        self.call(move |c| {
            let mut q=c.prepare("SELECT w.record_json FROM assistant_work_changes w JOIN workspace_session_links s ON s.session_id=json_extract(w.record_json,'$.sessionCode') WHERE w.assistant_code=?1 AND json_extract(s.business_json,'$.user_code')=?2 AND json_extract(w.record_json,'$.kind') IN ('NeedsDecision','Blocked') AND json_extract(w.record_json,'$.resolvedAt') IS NULL ORDER BY w.created_at,w.change_id LIMIT 32").map_err(storage)?;
            q.query_map(params![assistant,user],|r|r.get::<_,String>(0)).map_err(storage)?.map(|r|decode(r.map_err(storage)?)).collect()
        })
    }
    fn scoped_recent_work_changes(
        &mut self,
        assistant: &str,
        user: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError> {
        let (assistant, user) = (assistant.to_owned(), user.to_owned());
        self.call(move |c| {
            let mut query = c.prepare("SELECT w.record_json FROM assistant_work_changes w JOIN workspace_session_links s ON s.session_id=json_extract(w.record_json,'$.sessionCode') WHERE w.assistant_code=?1 AND json_extract(s.business_json,'$.user_code')=?2 ORDER BY w.created_at DESC,w.change_id DESC LIMIT 32").map_err(storage)?;
            query.query_map(params![assistant, user], |row| row.get::<_, String>(0)).map_err(storage)?.map(|row| decode(row.map_err(storage)?)).collect()
        })
    }
    fn pending_work_changes(
        &mut self,
        assistant: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError> {
        let assistant = assistant.to_owned();
        self.call(move |c| {
            let mut query = c.prepare("SELECT record_json FROM assistant_work_changes WHERE assistant_code=?1 AND json_extract(record_json,'$.kind') IN ('NeedsDecision','Blocked') AND json_extract(record_json,'$.resolvedAt') IS NULL ORDER BY created_at,change_id").map_err(storage)?;
            query.query_map([assistant], |r| r.get::<_,String>(0)).map_err(storage)?.map(|r|decode(r.map_err(storage)?)).collect()
        })
    }
    fn work_changes(
        &mut self,
        assistant: &str,
        task: &str,
    ) -> Result<Vec<AssistantWorkChange>, AssistantError> {
        let (assistant, task) = (assistant.to_owned(), task.to_owned());
        self.call(move |c| {
            let mut q = c.prepare("SELECT record_json FROM assistant_work_changes WHERE assistant_code=?1 AND task_code=?2 ORDER BY created_at,change_id").map_err(storage)?;
            q.query_map(params![assistant,task], |r| r.get::<_,String>(0)).map_err(storage)?.map(|r| decode(r.map_err(storage)?)).collect()
        })
    }
    fn work_change(
        &mut self,
        assistant: &str,
        id: &str,
    ) -> Result<AssistantWorkChange, AssistantError> {
        let (assistant, id) = (assistant.to_owned(), id.to_owned());
        self.call(move |c| {
            let json = c
                .query_row(
                    "SELECT record_json FROM assistant_work_changes WHERE assistant_code=?1 AND change_id=?2",
                    params![assistant, id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage)?;
            decode(json.ok_or(AssistantError::NotFound)?)
        })
    }
    fn resolve_work_change(
        &mut self,
        assistant: &str,
        id: &str,
        resolution: &str,
        evidence: &[String],
        now: i64,
    ) -> Result<AssistantWorkChange, AssistantError> {
        let (assistant, id, resolution, evidence) = (
            assistant.to_owned(),
            id.to_owned(),
            resolution.to_owned(),
            evidence.to_vec(),
        );
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate)
                .map_err(storage)?;
            let json = tx
                .query_row(
                    "SELECT record_json FROM assistant_work_changes WHERE assistant_code=?1 AND change_id=?2",
                    params![assistant, id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage)?;
            let mut change: AssistantWorkChange = decode(json.ok_or(AssistantError::NotFound)?)?;
            if change.resolved_at.is_some() {
                return Ok(change);
            }
            change.resolved_at = Some(now);
            change.resolution = Some(resolution.clone());
            change.resolution_evidence = evidence.clone();
            if let Some(notice) = &mut change.notification {
                notice.resolved_at = Some(now);
                notice.target["resolution"] = resolution.clone().into();
                notice.target["resolutionEvidence"] = serde_json::json!(evidence);
                // Preserve read/archive state; resolving is independent of reading.
                let persisted: Option<String> = tx
                    .query_row(
                        "SELECT record_json FROM platform_notifications WHERE notification_id=?1",
                        [&notice.id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?;
                if let Some(json) = persisted {
                    let mut current: PlatformNotification = decode(json)?;
                    current.resolved_at = notice.resolved_at;
                    current.target = notice.target.clone();
                    tx.execute(
                        "UPDATE platform_notifications SET record_json=?2 WHERE notification_id=?1",
                        params![current.id, encode(&current)?],
                    )
                    .map_err(storage)?;
                }
            }
            tx.execute(
                "UPDATE assistant_work_changes SET record_json=?2 WHERE change_id=?1",
                params![id, encode(&change)?],
            )
            .map_err(storage)?;
            tx.commit().map_err(storage)?;
            Ok(change)
        })
    }
    fn notify_work_change(
        &mut self,
        assistant: &str,
        id: &str,
        title: &str,
        body: &str,
        now: i64,
    ) -> Result<PlatformNotification, AssistantError> {
        let (assistant, id, title, body) = (
            assistant.to_owned(),
            id.to_owned(),
            title.to_owned(),
            body.to_owned(),
        );
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let json = tx.query_row("SELECT record_json FROM assistant_work_changes WHERE assistant_code=?1 AND change_id=?2", params![assistant,id], |r| r.get(0)).optional().map_err(storage)?;
            let mut change: AssistantWorkChange = decode(json.ok_or(AssistantError::NotFound)?)?;
            // Receipt survives archive/deletion: replay never recreates a dismissed notice.
            if let Some(notice) = change.notification {
                if notice.title != title || notice.body != body {
                    return Err(AssistantError::Runtime("NOTIFICATION_CONTENT_CONFLICT: this changeId already has a notification with different title/body; inspect assistant_notification_get. The original notification was not changed or resent.".into()));
                }
                return Ok(notice);
            }
            if change.resolved_at.is_some() { return Err(AssistantError::Conflict); }
            validate_work_snapshot(&tx, &change)?;
            let notice = PlatformNotification {
                id: format!("work-change:{}",change.change_id), dedup_key: format!("work-change:{}",change.change_id),
                assistant_code: Some(assistant), title, body, category: format!("Work{:?}",change.kind),
                target: serde_json::json!({"changeId":change.change_id,"taskCode":change.task_code,"sessionCode":change.session_code,"sessionKind":"Assistant","changeKind":change.kind,"evidence":change.evidence,"requestedAction":change.requested_action}),
                read_at:None,resolved_at:None,archived_at:None,created_at:now,
            };
            tx.execute("INSERT INTO platform_notifications(notification_id,dedup_key,record_json,created_at) VALUES(?1,?2,?3,?4)", params![notice.id,notice.dedup_key,encode(&notice)?,now]).map_err(storage)?;
            change.notification = Some(notice.clone());
            tx.execute("UPDATE assistant_work_changes SET record_json=?2 WHERE change_id=?1", params![id,encode(&change)?]).map_err(storage)?;
            tx.commit().map_err(storage)?; Ok(notice)
        })
    }

    fn notifications(&mut self) -> Result<Vec<PlatformNotification>, AssistantError> {
        self.call(|c|{let mut q=c.prepare("SELECT record_json FROM platform_notifications ORDER BY created_at DESC LIMIT 500").map_err(storage)?;q.query_map([],|r|r.get::<_,String>(0)).map_err(storage)?.map(|r|decode(r.map_err(storage)?)).collect()})
    }
    fn scoped_notifications(
        &mut self,
        assistant: &str,
        user: &str,
        id: Option<&str>,
        current: usize,
        size: usize,
        include_archived: bool,
    ) -> Result<(usize, Vec<PlatformNotification>), AssistantError> {
        let (assistant, user, id) = (assistant.to_owned(), user.to_owned(), id.map(str::to_owned));
        self.call(move |c| {
            let predicate = "json_extract(n.record_json,'$.assistantCode')=?1 AND (EXISTS(SELECT 1 FROM workspace_session_links s WHERE s.session_id=json_extract(n.record_json,'$.target.sessionCode') AND json_extract(s.business_json,'$.user_code')=?2) OR EXISTS(SELECT 1 FROM collaboration_threads t WHERE t.session_id=json_extract(n.record_json,'$.target.sessionCode') AND t.user_code=?2 AND t.owner_kind='ASSISTANT' AND t.owner_code=?1)) AND (?3 IS NULL OR n.notification_id=?3) AND (?4 OR json_extract(n.record_json,'$.archivedAt') IS NULL)";
            let from = "platform_notifications n";
            let total: i64 = c.query_row(&format!("SELECT COUNT(*) FROM {from} WHERE {predicate}"), params![assistant,user,id,include_archived], |r| r.get(0)).map_err(storage)?;
            let mut q = c.prepare(&format!("SELECT n.record_json FROM {from} WHERE {predicate} ORDER BY n.created_at DESC,n.notification_id DESC LIMIT ?5 OFFSET ?6")).map_err(storage)?;
            let records = q.query_map(params![assistant,user,id,include_archived,size as i64,((current-1)*size) as i64], |r| r.get::<_,String>(0)).map_err(storage)?.map(|r|decode(r.map_err(storage)?)).collect::<Result<Vec<_>,_>>()?;
            Ok((total as usize, records))
        })
    }
    fn update_notification(
        &mut self,
        id: &str,
        read: bool,
        archive: bool,
        now: i64,
    ) -> Result<(), AssistantError> {
        let id = id.to_owned();
        self.call(move |c| {
            let json: Option<String> = c
                .query_row(
                    "SELECT record_json FROM platform_notifications WHERE notification_id=?1",
                    [&id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage)?;
            let mut n: PlatformNotification = decode(json.ok_or(AssistantError::NotFound)?)?;
            n.read_at = (read || archive).then_some(now);
            n.archived_at = archive.then_some(now);
            c.execute(
                "UPDATE platform_notifications SET record_json=?2 WHERE notification_id=?1",
                params![id, encode(&n)?],
            )
            .map_err(storage)?;
            Ok(())
        })
    }
    fn update_notifications(
        &mut self,
        ids: &[String],
        archive: bool,
        now: i64,
    ) -> Result<usize, AssistantError> {
        let count = ids.len();
        let ids = encode(&ids)?;
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
            let found: usize = tx.query_row("SELECT count(*) FROM platform_notifications WHERE notification_id IN (SELECT value FROM json_each(?1))", [&ids], |r| r.get(0)).map_err(storage)?;
            if found != count { return Err(AssistantError::NotFound); }
            // Mark-read must not resurrect a notification archived concurrently.
            let changed = tx.execute("UPDATE platform_notifications SET record_json=json_set(record_json,'$.readAt',COALESCE(json_extract(record_json,'$.readAt'),?2),'$.archivedAt',CASE WHEN ?3 THEN COALESCE(json_extract(record_json,'$.archivedAt'),?2) ELSE json_extract(record_json,'$.archivedAt') END) WHERE notification_id IN (SELECT value FROM json_each(?1))", params![ids,now,archive]).map_err(storage)?;
            tx.commit().map_err(storage)?;
            Ok(changed)
        })
    }
    fn delete_archived_notification(&mut self, id: &str) -> Result<(), AssistantError> {
        let id = id.to_owned();
        self.call(move |connection| {
            let tx = business_transaction(connection, rusqlite::TransactionBehavior::Immediate)
                .map_err(storage)?;
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM platform_notifications WHERE notification_id=?1)",
                    [&id],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            if !exists {
                return Err(AssistantError::NotFound);
            }
            let deleted = tx
                .execute(
                    "DELETE FROM platform_notifications WHERE notification_id=?1 AND json_extract(record_json,'$.archivedAt') IS NOT NULL",
                    [&id],
                )
                .map_err(storage)?;
            if deleted == 0 {
                return Err(AssistantError::Conflict);
            }
            tx.commit().map_err(storage)
        })
    }
    fn delete_archived_notifications(&mut self) -> Result<usize, AssistantError> {
        self.call(|connection| {
            let tx = business_transaction(connection, rusqlite::TransactionBehavior::Immediate)
                .map_err(storage)?;
            let deleted = tx
                .execute(
                    "DELETE FROM platform_notifications WHERE json_extract(record_json,'$.archivedAt') IS NOT NULL",
                    [],
                )
                .map_err(storage)?;
            tx.commit().map_err(storage)?;
            Ok(deleted)
        })
    }
}
