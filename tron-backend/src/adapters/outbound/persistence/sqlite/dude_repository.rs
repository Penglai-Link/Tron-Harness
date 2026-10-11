use super::framework_storage::{SharedDatabase, business_transaction};
use crate::application::dude::DudeRepository;
use crate::domain::dude::*;
use rusqlite::{OptionalExtension, params};

#[derive(Clone)]
pub struct SqliteDudeRepository {
    database: SharedDatabase,
}
impl SqliteDudeRepository {
    pub fn connect(path: &str) -> Result<Self, DudeError> {
        Ok(Self {
            database: SharedDatabase::open(path).map_err(storage)?,
        })
    }
    fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut rusqlite::Connection) -> Result<T, DudeError> + Send + 'static,
    ) -> Result<T, DudeError> {
        self.database.call(f, || DudeError::Storage)
    }
}
fn storage(e: rusqlite::Error) -> DudeError {
    tracing::error!("Dude SQLite: {e}");
    DudeError::Storage
}
fn encode<T: serde::Serialize>(v: &T) -> Result<String, DudeError> {
    serde_json::to_string(v).map_err(|_| DudeError::Storage)
}
fn decode<T: serde::de::DeserializeOwned>(s: String) -> Result<T, DudeError> {
    serde_json::from_str(&s).map_err(|_| DudeError::Storage)
}
fn load_events(
    c: &rusqlite::Connection,
    sql: &str,
    code: &str,
) -> Result<Vec<DudeEvent>, DudeError> {
    let mut q = c.prepare(sql).map_err(storage)?;
    q.query_map([code], |r| r.get::<_, String>(0))
        .map_err(storage)?
        .map(|r| decode(r.map_err(storage)?))
        .collect()
}
impl DudeRepository for SqliteDudeRepository {
    fn list(&mut self) -> Result<Vec<Dude>, DudeError> {
        self.call(|c| {
            let mut q = c
                .prepare("SELECT record_json,is_main FROM dudes ORDER BY rowid")
                .map_err(storage)?;
            q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)))
                .map_err(storage)?
                .map(|r| {
                    let (json, main) = r.map_err(storage)?;
                    let mut d: Dude = decode(json)?;
                    d.is_main = main;
                    Ok(d)
                })
                .collect()
        })
    }
    fn save(&mut self, d: &Dude) -> Result<(), DudeError> {
        let d = d.clone();
        self.call(move|c|{c.execute("INSERT INTO dudes(dude_code,record_json,enabled,heartbeat_minutes,next_heartbeat_at,is_main) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(dude_code) DO UPDATE SET record_json=excluded.record_json,enabled=excluded.enabled,heartbeat_minutes=excluded.heartbeat_minutes",params![d.dude_code,encode(&d)?,d.enabled,d.heartbeat_minutes,d.updated_at+i64::from(d.heartbeat_minutes)*60000,d.is_main]).map_err(storage)?;Ok(())})
    }
    fn delete(&mut self, code: &str) -> Result<(), DudeError> {
        let code = code.to_owned();
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM dude_events WHERE dude_code=?1 AND status IN ('Queued','Running','AwaitingInput'))",[&code],|r|r.get(0)).map_err(storage)?;if busy{return Err(DudeError::Conflict)}tx.execute("DELETE FROM dude_events WHERE dude_code=?1",[&code]).map_err(storage)?;tx.execute("DELETE FROM dude_sessions WHERE dude_code=?1",[&code]).map_err(storage)?;tx.execute("DELETE FROM dudes WHERE dude_code=?1 AND is_main=0",[&code]).map_err(storage)?;tx.commit().map_err(storage)})
    }
    fn designate_main(&mut self, code: &str) -> Result<(), DudeError> {
        let code = code.to_owned();
        self.call(move |c| {
            let tx = business_transaction(c, rusqlite::TransactionBehavior::Immediate)
                .map_err(storage)?;
            tx.execute("UPDATE dudes SET is_main=0 WHERE is_main=1", [])
                .map_err(storage)?;
            if tx
                .execute("UPDATE dudes SET is_main=1 WHERE dude_code=?1", [code])
                .map_err(storage)?
                != 1
            {
                return Err(DudeError::NotFound);
            }
            tx.commit().map_err(storage)
        })
    }
    fn resolve_session(&mut self, s: &DudeSession) -> Result<DudeSession, DudeError> {
        let s = s.clone();
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;tx.execute("INSERT INTO dude_sessions(session_code,dude_code,purpose,correlation_key,record_json) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(dude_code,purpose,correlation_key) DO NOTHING",params![s.session_code,s.dude_code,format!("{:?}",s.purpose),s.correlation_key,encode(&s)?]).map_err(storage)?;let json=tx.query_row("SELECT record_json FROM dude_sessions WHERE dude_code=?1 AND purpose=?2 AND correlation_key=?3",params![s.dude_code,format!("{:?}",s.purpose),s.correlation_key],|r|r.get(0)).map_err(storage)?;tx.commit().map_err(storage)?;decode(json)})
    }
    fn sessions(&mut self, code: &str) -> Result<Vec<DudeSession>, DudeError> {
        let code = code.to_owned();
        self.call(move |c| {
            let mut q = c
                .prepare(
                    "SELECT record_json FROM dude_sessions WHERE dude_code=?1 ORDER BY rowid DESC",
                )
                .map_err(storage)?;
            q.query_map([code], |r| r.get::<_, String>(0))
                .map_err(storage)?
                .map(|r| decode(r.map_err(storage)?))
                .collect()
        })
    }
    fn session(&mut self, code: &str) -> Result<Option<DudeSession>, DudeError> {
        let code = code.to_owned();
        self.call(move |c| {
            c.query_row(
                "SELECT record_json FROM dude_sessions WHERE session_code=?1",
                [code],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(storage)?
            .map(decode)
            .transpose()
        })
    }
    fn enqueue(&mut self, e: &DudeEvent) -> Result<DudeEvent, DudeError> {
        let e = e.clone();
        self.call(move|c|{c.execute("INSERT INTO dude_events(event_id,dude_code,dedup_key,source,session_code,status,available_at,record_json) VALUES(?1,?2,?3,?4,?5,'Queued',?6,?7) ON CONFLICT(dedup_key) DO NOTHING",params![e.event_id,e.dude_code,e.dedup_key,format!("{:?}",e.source),e.session_code,e.available_at,encode(&e)?]).map_err(storage)?;decode(c.query_row("SELECT record_json FROM dude_events WHERE dedup_key=?1",[e.dedup_key],|r|r.get(0)).map_err(storage)?)})
    }
    fn events(&mut self, code: &str) -> Result<Vec<DudeEvent>, DudeError> {
        let code = code.to_owned();
        self.call(move|c|load_events(c,"SELECT record_json FROM dude_events WHERE dude_code=?1 ORDER BY rowid DESC LIMIT 200",&code))
    }
    fn claim(&mut self, now: i64, lease_ms: i64) -> Result<Option<DudeEvent>, DudeError> {
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;
 let json:Option<String>=tx.query_row("SELECT e.record_json FROM dude_events e JOIN dudes d USING(dude_code) WHERE (d.enabled=1 OR e.source='UserMessage') AND e.available_at<=?1 AND (e.status='Queued' OR (e.status='Running' AND e.lease_until<?1)) AND NOT EXISTS(SELECT 1 FROM dude_events busy WHERE busy.session_code=e.session_code AND e.session_code IS NOT NULL AND ((busy.status='Running' AND busy.lease_until>=?1) OR (busy.status='AwaitingInput' AND e.source!='UserMessage'))) ORDER BY e.available_at,e.rowid LIMIT 1",[now],|r|r.get(0)).optional().map_err(storage)?;
 let Some(json)=json else {return Ok(None)};let mut e:DudeEvent=decode(json)?;e.status=DudeEventStatus::Running;e.lease_token=Some(eliteforge_agent_core::SessionRecord::new("").id);e.lease_until=Some(now+lease_ms.max(1000));e.attempts+=1;
 tx.execute("UPDATE dude_events SET status='Running',lease_until=?2,lease_token=?3,attempts=?4,record_json=?5 WHERE event_id=?1",params![e.event_id,e.lease_until,e.lease_token,e.attempts,encode(&e)?]).map_err(storage)?;tx.commit().map_err(storage)?;Ok(Some(e))})
    }
    fn finish(
        &mut self,
        id: &str,
        token: &str,
        status: DudeEventStatus,
        result: &str,
        now: i64,
    ) -> Result<(), DudeError> {
        let (id, token, result) = (id.to_owned(), token.to_owned(), result.to_owned());
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;let json:Option<String>=tx.query_row("SELECT record_json FROM dude_events WHERE event_id=?1 AND lease_token=?2 AND status='Running' AND lease_until>=?3",params![id,token,now],|r|r.get(0)).optional().map_err(storage)?;let mut e:DudeEvent=decode(json.ok_or(DudeError::Conflict)?)?;e.status=status;e.result=Some(result);e.lease_until=None;e.lease_token=None;if tx.execute("UPDATE dude_events SET status=?3,lease_until=NULL,lease_token=NULL,record_json=?4 WHERE event_id=?1 AND lease_token=?2",params![id,token,format!("{status:?}"),encode(&e)?]).map_err(storage)?!=1{return Err(DudeError::Conflict)}
if let Some(n)=e.completion_notification(status,e.result.as_deref().unwrap_or(""),now){tx.execute("INSERT INTO platform_notifications(notification_id,dedup_key,record_json,created_at) VALUES(?1,?2,?3,?4) ON CONFLICT(dedup_key) DO NOTHING",params![n.id,n.dedup_key,encode(&n)?,n.created_at]).map_err(storage)?;}
tx.commit().map_err(storage)})
    }
    fn attach_session(&mut self, id: &str, token: &str, session: &str) -> Result<(), DudeError> {
        let (id, token, session) = (id.to_owned(), token.to_owned(), session.to_owned());
        self.call(move|c|{let json:Option<String>=c.query_row("SELECT record_json FROM dude_events WHERE event_id=?1 AND lease_token=?2 AND status='Running'",params![id,token],|r|r.get(0)).optional().map_err(storage)?;let mut e:DudeEvent=decode(json.ok_or(DudeError::Conflict)?)?;e.session_code=Some(session.clone());c.execute("UPDATE dude_events SET session_code=?3,record_json=?4 WHERE event_id=?1 AND lease_token=?2",params![id,token,session,encode(&e)?]).map_err(storage)?;Ok(())})
    }
    fn schedule_heartbeats(&mut self, now: i64) -> Result<(), DudeError> {
        self.call(move|c|{let tx=business_transaction(c,rusqlite::TransactionBehavior::Immediate).map_err(storage)?;let rows={let mut q=tx.prepare("SELECT dude_code,heartbeat_minutes,next_heartbeat_at FROM dudes WHERE enabled=1 AND next_heartbeat_at<=?1").map_err(storage)?;q.query_map([now],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?,r.get::<_,i64>(2)?))).map_err(storage)?.collect::<Result<Vec<_>,_>>().map_err(storage)?};
 for(code,cadence,scheduled)in rows{let busy:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM dude_events WHERE dude_code=?1 AND source='Heartbeat' AND status IN ('Queued','Running'))",[&code],|r|r.get(0)).map_err(storage)?;
 if !busy{let e=DudeEvent{event_id:eliteforge_agent_core::SessionRecord::new("").id,dude_code:code.clone(),source:DudeEventSource::Heartbeat,dedup_key:format!("heartbeat:{code}:{scheduled}"),session_code:None,task_code:None,workspace_code:None,payload:serde_json::json!({"scheduledAt":scheduled}),status:DudeEventStatus::Queued,available_at:now,lease_until:None,lease_token:None,attempts:0,result:None,created_at:now};tx.execute("INSERT INTO dude_events(event_id,dude_code,dedup_key,source,status,available_at,record_json) VALUES(?1,?2,?3,'Heartbeat','Queued',?4,?5)",params![e.event_id,code,e.dedup_key,now,encode(&e)?]).map_err(storage)?;}
 tx.execute("UPDATE dudes SET next_heartbeat_at=?2 WHERE dude_code=?1",params![code,now+i64::from(cadence)*60000]).map_err(storage)?;}
 tx.commit().map_err(storage)})
    }
    fn notify(&mut self, n: &PlatformNotification) -> Result<PlatformNotification, DudeError> {
        let n = n.clone();
        self.call(move|c|{c.execute("INSERT INTO platform_notifications(notification_id,dedup_key,record_json,created_at) VALUES(?1,?2,?3,?4) ON CONFLICT(dedup_key) DO NOTHING",params![n.id,n.dedup_key,encode(&n)?,n.created_at]).map_err(storage)?;decode(c.query_row("SELECT record_json FROM platform_notifications WHERE dedup_key=?1",[n.dedup_key],|r|r.get(0)).map_err(storage)?)})
    }
    fn notifications(&mut self) -> Result<Vec<PlatformNotification>, DudeError> {
        self.call(|c|{let mut q=c.prepare("SELECT record_json FROM platform_notifications ORDER BY created_at DESC LIMIT 500").map_err(storage)?;q.query_map([],|r|r.get::<_,String>(0)).map_err(storage)?.map(|r|decode(r.map_err(storage)?)).collect()})
    }
    fn update_notification(
        &mut self,
        id: &str,
        read: bool,
        archive: bool,
        now: i64,
    ) -> Result<(), DudeError> {
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
            let mut n: PlatformNotification = decode(json.ok_or(DudeError::NotFound)?)?;
            n.read_at = read.then_some(now);
            n.archived_at = archive.then_some(now);
            c.execute(
                "UPDATE platform_notifications SET record_json=?2 WHERE notification_id=?1",
                params![id, encode(&n)?],
            )
            .map_err(storage)?;
            Ok(())
        })
    }
}
