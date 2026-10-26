//! Tron owns only workspace/business links. Session lifecycle and presentation
//! history are stored through Framework services on the injected connection.
use super::framework_storage::{blocking, chat_error};
use crate::{
    application::{agent::FrameworkHost, chat::ChatRepository},
    domain::chat::*,
};
use eliteforge_agent_core as fw;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct SqliteChatRepository {
    host: FrameworkHost,
}

/// One round trip builds the whole session projection: business link, native
/// session record, latest non-summary run, and the external runtime reference.
/// The correlated subqueries use the ef_runs_session index and
/// ef_session_context primary key.
const SESSION_PROJECTION_SQL: &str = "SELECT l.session_id, l.business_json, \
    json_extract(s.record,'$.title'), json_extract(s.record,'$.status'), \
    json_extract(s.record,'$.created_at_ms'), json_extract(s.record,'$.updated_at_ms'), \
    (SELECT r.record FROM ef_runs r WHERE (r.session_id=l.session_id OR r.owner_session_id=l.session_id) \
        AND json_extract(r.record,'$.kind')<>'SUMMARY' \
        ORDER BY json_extract(r.record,'$.started_at_ms') DESC, r.id DESC LIMIT 1), \
    (SELECT c.value FROM ef_session_context c WHERE c.session_id=l.session_id AND c.key='tron.external-reference') \
    FROM workspace_session_links l JOIN ef_sessions s ON s.id=l.session_id";

/// Columns of SESSION_PROJECTION_SQL before the ChatSession mapping.
struct SessionProjection {
    session_id: String,
    link_json: String,
    title: String,
    status: String,
    created_at_ms: i64,
    updated_at_ms: i64,
    last_run_json: Option<String>,
    external_reference: Option<String>,
}
fn session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionProjection> {
    Ok(SessionProjection {
        session_id: row.get(0)?,
        link_json: row.get(1)?,
        title: row.get(2)?,
        status: row.get(3)?,
        created_at_ms: row.get(4)?,
        updated_at_ms: row.get(5)?,
        last_run_json: row.get(6)?,
        external_reference: row.get(7)?,
    })
}
fn chat_session_from_row(row: SessionProjection) -> Result<ChatSession, ChatError> {
    let link: Link =
        serde_json::from_str(&row.link_json).map_err(|_| ChatError::StorageFailure)?;
    let status = match row.status.as_str() {
        "Active" | "ACTIVE" => ChatSessionStatus::Active,
        "Archived" | "ARCHIVED" => ChatSessionStatus::Archived,
        _ => return Err(ChatError::StorageFailure),
    };
    let last_run = row
        .last_run_json
        .as_deref()
        .map(serde_json::from_str::<fw::ManagedRun>)
        .transpose()
        .map_err(|_| ChatError::StorageFailure)?;
    let last = last_run.as_ref();
    let runtime = row
        .external_reference
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value["sessionId"].as_str().map(str::to_owned));
    Ok(ChatSession {
        id: link.id,
        session_code: row.session_id,
        workspace_code: link.workspace_code,
        project_code: link.project_code,
        user_code: link.user_code,
        app_code: link.app_code,
        task_code: link.task_code,
        source_type: link.source_type,
        source_code: link.source_code,
        agent_code: link.agent_code,
        system_agent_session_id: runtime,
        title: row.title,
        status,
        last_run_code: last.map(|r| r.id.to_string()),
        last_run_status: last.map(|r| {
            match r.status {
                fw::ManagedRunStatus::Completed => "COMPLETED",
                fw::ManagedRunStatus::AwaitingInput => "AWAITING_USER_INPUT",
                fw::ManagedRunStatus::Running => "RUNNING",
                fw::ManagedRunStatus::Cancelled => "STOPPED",
                _ => "FAILED",
            }
            .into()
        }),
        created_at: crate::shared::time::timestamp_from_millis(row.created_at_ms),
        updated_at: crate::shared::time::timestamp_from_millis(row.updated_at_ms),
    })
}

/// Remove the execution spans owned by a deleted session. A trace can also
/// contain an HTTP span or another session, so deleting the trace unconditionally
/// would erase unrelated observability data.
fn delete_session_telemetry(c: &Connection, code: &str) -> rusqlite::Result<()> {
    let exists: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM telemetry.sqlite_master WHERE type='table' AND name='ef_telemetry_traces')",
        [], |row| row.get(0),
    )?;
    if !exists { return Ok(()); }
    let trace_ids = c.prepare(
        "SELECT trace_id FROM telemetry.ef_telemetry_traces WHERE json_extract(attributes,'$.\"tron.session.code\"')=?1
         UNION SELECT trace_id FROM telemetry.ef_telemetry_spans WHERE json_extract(data,'$.attributes.\"tron.session.code\"')=?1",
    )?.query_map([code], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for trace_id in trace_ids {
        let spans = c.prepare(
            "SELECT span_id,json_extract(data,'$.parentSpanId'),json_extract(data,'$.attributes.\"tron.session.code\"')
             FROM telemetry.ef_telemetry_spans WHERE trace_id=?1",
        )?.query_map([&trace_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        let mut owners = HashMap::new();
        let mut doomed = HashSet::new();
        let mut queue = VecDeque::new();
        for (id, parent, owner) in &spans {
            if let Some(parent) = parent { children.entry(parent.clone()).or_default().push(id.clone()); }
            owners.insert(id.clone(), owner.clone());
            if owner.as_deref() == Some(code) {
                doomed.insert(id.clone());
                queue.push_back(id.clone());
            }
        }
        while let Some(parent) = queue.pop_front() {
            for child in children.get(&parent).into_iter().flatten() {
                // A child execution for a different session must survive even
                // when its parent belongs to the deleted session.
                if owners.get(child).and_then(Option::as_deref).is_some_and(|owner| owner != code) { continue; }
                if doomed.insert(child.clone()) { queue.push_back(child.clone()); }
            }
        }
        if !spans.is_empty() && doomed.len() == spans.len() {
            c.execute("DELETE FROM telemetry.ef_telemetry_traces WHERE trace_id=?1", [&trace_id])?;
            continue;
        }
        for id in &doomed {
            c.execute("DELETE FROM telemetry.ef_telemetry_spans WHERE trace_id=?1 AND span_id=?2", params![trace_id, id])?;
            c.execute("DELETE FROM telemetry.ef_telemetry_span_rollups WHERE trace_id=?1 AND span_id=?2", params![trace_id, id])?;
        }
        // Preserve surviving execution roots without dangling parent links.
        for (id, parent, _) in &spans {
            if doomed.contains(id) { continue; }
            if parent.as_ref().is_some_and(|parent| doomed.contains(parent)) {
                c.execute("UPDATE telemetry.ef_telemetry_spans SET data=json_set(data,'$.parentSpanId',NULL) WHERE trace_id=?1 AND span_id=?2", params![trace_id, id])?;
                c.execute("UPDATE telemetry.ef_telemetry_span_rollups SET parent_id=NULL WHERE trace_id=?1 AND span_id=?2", params![trace_id, id])?;
            }
        }
        if spans.is_empty() {
            c.execute("DELETE FROM telemetry.ef_telemetry_traces WHERE trace_id=?1", [&trace_id])?;
            continue;
        }
        // A shared trace may have been registered with the deleted session's
        // metadata. Remove only that association.
        c.execute("UPDATE telemetry.ef_telemetry_traces SET attributes=json_remove(attributes,'$.\"tron.session.code\"','$.\"tron.run.code\"','$.\"tron.agent.code\"','$.\"tron.trace.agent\"') WHERE trace_id=?1 AND json_extract(attributes,'$.\"tron.session.code\"')=?2", params![trace_id, code])?;
        if !doomed.is_empty() { rebuild_trace_rollups(c, &trace_id)?; }
    }
    Ok(())
}

fn rebuild_trace_rollups(c: &Connection, trace_id: &str) -> rusqlite::Result<()> {
    type Counts = [i64; 7];
    let nodes = c.prepare("SELECT span_id,parent_id,own_span_count,own_error_count,own_model_count,own_tool_count,own_input_tokens,own_cached_input_tokens,own_output_tokens FROM telemetry.ef_telemetry_span_rollups WHERE trace_id=?1")?
        .query_map([trace_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, [row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?])))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let parents: HashMap<_, _> = nodes.iter().map(|(id, parent, _)| (id.clone(), parent.clone())).collect();
    let mut totals: HashMap<String, Counts> = nodes.iter().map(|(id, _, _)| (id.clone(), [0; 7])).collect();
    let mut trace_total = [0; 7];
    for (id, _, own) in &nodes {
        for (sum, value) in trace_total.iter_mut().zip(own) { *sum += value; }
        let mut current = Some(id.as_str());
        let mut seen = HashSet::new();
        while let Some(node) = current {
            if !seen.insert(node) { break; }
            let Some(total) = totals.get_mut(node) else { break; };
            for (sum, value) in total.iter_mut().zip(own) { *sum += value; }
            current = parents.get(node).and_then(Option::as_deref);
        }
    }
    for (id, counts) in totals {
        c.execute("UPDATE telemetry.ef_telemetry_span_rollups SET span_count=?3,error_count=?4,model_count=?5,tool_count=?6,input_tokens=?7,cached_input_tokens=?8,output_tokens=?9 WHERE trace_id=?1 AND span_id=?2",
            params![trace_id,id,counts[0],counts[1],counts[2],counts[3],counts[4],counts[5],counts[6]])?;
    }
    c.execute("UPDATE telemetry.ef_telemetry_trace_rollups SET span_count=?2,error_count=?3,model_count=?4,tool_count=?5,input_tokens=?6,cached_input_tokens=?7,output_tokens=?8 WHERE trace_id=?1",
        params![trace_id,trace_total[0],trace_total[1],trace_total[2],trace_total[3],trace_total[4],trace_total[5],trace_total[6]])?;
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
struct Link {
    id: i64,
    workspace_code: String,
    project_code: Option<String>,
    user_code: Option<String>,
    app_code: Option<String>,
    task_code: Option<String>,
    source_type: ChatSessionSourceType,
    source_code: Option<String>,
    agent_code: Option<String>,
}
impl Link {
    fn from_session(s: &ChatSession) -> Self {
        Self {
            id: s.id,
            workspace_code: s.workspace_code.clone(),
            project_code: s.project_code.clone(),
            user_code: s.user_code.clone(),
            app_code: s.app_code.clone(),
            task_code: s.task_code.clone(),
            source_type: s.source_type,
            source_code: s.source_code.clone(),
            agent_code: s.agent_code.clone(),
        }
    }
}
impl SqliteChatRepository {
    pub fn connect(database_url: &str) -> Result<Self, ChatError> {
        let host = if database_url == ":memory:" {
            FrameworkHost::temporary(None)
        } else {
            Ok(FrameworkHost::persistent(database_url, None))
        }
        .map_err(|_| ChatError::StorageFailure)?;
        Self::with_host(host)
    }
    pub fn with_host(host: FrameworkHost) -> Result<Self, ChatError> {
        blocking(async {
            host.storage_framework()
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            host.shared_database()
                .await
                .map_err(chat_error)?
                .call(|c| super::schema::initialize_business(c))
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            Ok(Self { host })
        })
    }
    pub(crate) async fn ensure_run_session(
        host: &FrameworkHost,
        context: &crate::domain::agent::AgentRunContext,
    ) -> Result<(), ChatError> {
        host.shared_database()
            .await
            .map_err(chat_error)?
            .call(|c| super::schema::initialize_business(c))
            .await
            .map_err(|_| ChatError::StorageFailure)?;
        let repository = Self { host: host.clone() };
        let id = context
            .conversation_code
            .as_deref()
            .unwrap_or(&context.run_code);
        match repository.session(id).await {
            Ok(session) => {
                if session.workspace_code != context.workspace_code {
                    return Err(ChatError::SessionNotFound);
                }
                if session.status != ChatSessionStatus::Active {
                    return Err(ChatError::SessionArchived);
                }
                return Ok(());
            }
            // A missing conversation is materialized lazily: explicit
            // conversation codes and run-scoped temp sessions share the same
            // creation path, bound to the already validated run workspace.
            Err(ChatError::SessionNotFound) => {}
            Err(error) => return Err(error),
        }
        let mut record = fw::SessionRecord::new("临时运行");
        record.id = id.into();
        let link = Link {
            id: 0,
            workspace_code: context.workspace_code.clone(),
            project_code: None,
            user_code: context.user_code.clone(),
            app_code: context.execution_scope.app_code.clone(),
            task_code: context.execution_scope.task_code.clone(),
            source_type: ChatSessionSourceType::Manual,
            source_code: None,
            agent_code: context
                .short_term_memory
                .as_ref()
                .and_then(|m| m.agent_code.clone()),
        };
        let value = serde_json::to_string(&link).map_err(|_| ChatError::StorageFailure)?;
        let id = id.to_owned();
        let db = host.shared_database().await.map_err(chat_error)?;
        let sessions = repository
            .framework()
            .await?
            .sessions()
            .map_err(|_| ChatError::StorageFailure)?;
        db.transaction(async {
            sessions.create(record).await?;
            db.call(move |c| {
                c.execute(
                    "INSERT INTO workspace_session_links VALUES (?1,?2,?3)",
                    params![id, link.workspace_code, value],
                )
            })
            .await
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(chat_error)
    }
    async fn framework(&self) -> Result<eliteforge_agent_framework::Framework, ChatError> {
        self.host
            .storage_framework()
            .await
            .map_err(|_| ChatError::StorageFailure)
    }
    async fn link(&self, id: &str) -> Result<Link, ChatError> {
        let id = id.to_owned();
        let json: Option<String> = self
            .host
            .shared_database()
            .await
            .map_err(chat_error)?
            .call(move |c| {
                c.query_row(
                    "SELECT business_json FROM workspace_session_links WHERE session_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()
            })
            .await
            .map_err(|_| ChatError::StorageFailure)?;
        serde_json::from_str(&json.ok_or(ChatError::SessionNotFound)?)
            .map_err(|_| ChatError::StorageFailure)
    }
    async fn session(&self, id: &str) -> Result<ChatSession, ChatError> {
        // Building the framework lazily creates the ef_* tables this
        // projection reads; the first call initializes shared storage.
        self.framework().await?;
        let id = id.to_owned();
        let raw = self
            .host
            .shared_database()
            .await
            .map_err(chat_error)?
            .call_read(move |c| -> Result<Vec<SessionProjection>, ChatError> {
                let mut statement = c
                    .prepare(&format!("{SESSION_PROJECTION_SQL} WHERE l.session_id=?1"))
                    .map_err(|_| ChatError::StorageFailure)?;
                statement
                    .query_map(params![id], session_row)
                    .map_err(|_| ChatError::StorageFailure)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| ChatError::StorageFailure)
            })
            .await
            .map_err(|_| ChatError::StorageFailure)?;
        chat_session_from_row(raw.into_iter().next().ok_or(ChatError::SessionNotFound)?)
    }
    async fn events(&self, session: &str, kind: &str) -> Result<Vec<fw::SessionEvent>, ChatError> {
        self.link(session).await?;
        let store = self
            .framework()
            .await?
            .events()
            .map_err(|_| ChatError::StorageFailure)?;
        let mut result = Vec::new();
        let mut after = 0;
        loop {
            let page = store
                .list_by_kind(session, kind, after, 1000)
                .await
                .map_err(chat_error)?;
            if page.is_empty() {
                break;
            }
            after = page.last().unwrap().sequence;
            result.extend(page);
        }
        Ok(result)
    }
    async fn put_context(&self, id: &str, key: &str, value: Value) -> Result<(), ChatError> {
        let s = self
            .framework()
            .await?
            .sessions()
            .map_err(|_| ChatError::StorageFailure)?;
        let expected = s
            .read_context(id, key)
            .await
            .map_err(chat_error)?
            .map(|(revision, _)| revision);
        s.write_context(id, key, expected, value)
            .await
            .map_err(chat_error)?;
        Ok(())
    }
}
impl ChatRepository for SqliteChatRepository {
    fn validate_annotation_sources(&mut self, session: &str, annotations: &[fw::MessageAnnotation]) -> Result<(), ChatError> {
        if annotations.is_empty() { return Ok(()); }
        blocking(async {
            self.framework().await?.history().map_err(|_| ChatError::StorageFailure)?
                .validate_annotation_sources(session, annotations).await
                .map_err(|error| match error { fw::StateError::Invalid(_) => ChatError::AnnotationsInvalid, other => chat_error(other) })
        })
    }

    fn session_titles(
        &mut self,
        codes: &[String],
    ) -> Result<Vec<crate::application::chat::ChatSessionTitle>, ChatError> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let codes = serde_json::to_string(codes).map_err(|_| ChatError::StorageFailure)?;
        blocking(async {
            self.host.shared_database().await.map_err(chat_error)?.call_read(move |c| {
                let mut statement = c.prepare(&format!("SELECT l.session_id,l.workspace_code,json_extract(s.record,'$.title') FROM workspace_session_links l JOIN ef_sessions s ON s.id=l.session_id WHERE l.session_id IN (SELECT value FROM json_each(?1))"))?;
                statement.query_map([codes],|row| Ok(crate::application::chat::ChatSessionTitle {
                    session_code:row.get(0)?,workspace_code:row.get(1)?,title:row.get(2)?,
                }))?.collect::<rusqlite::Result<Vec<_>>>()
            }).await.map_err(|_| ChatError::StorageFailure)
        })
    }
    fn sessions_by_codes(&mut self, codes: &[String]) -> Result<Vec<ChatSession>, ChatError> {
        if codes.is_empty() {
            return Ok(Vec::new());
        }
        let codes = serde_json::to_string(codes).map_err(|_| ChatError::StorageFailure)?;
        blocking(async {
            let rows = self.host.shared_database().await.map_err(chat_error)?.call_read(move |c| {
                let mut q = c.prepare(&format!("{SESSION_PROJECTION_SQL} WHERE l.session_id IN (SELECT value FROM json_each(?1))"))?;
                q.query_map([codes], session_row)?.collect::<rusqlite::Result<Vec<_>>>()
            }).await.map_err(|_| ChatError::StorageFailure)?;
            rows.into_iter().map(chat_session_from_row).collect()
        })
    }
    fn pending_input_counts(
        &mut self,
        codes: &[String],
    ) -> Result<std::collections::BTreeMap<String, u32>, ChatError> {
        if codes.is_empty() {
            return Ok(Default::default());
        }
        let codes = serde_json::to_string(codes).map_err(|_| ChatError::StorageFailure)?;
        blocking(async {
            self.host.shared_database().await.map_err(chat_error)?.call_read(move |c| {
                let mut q = c.prepare("WITH requests AS (SELECT e.session_id,json_extract(e.record,'$.payload.requestCode') AS request_code FROM ef_events e WHERE e.session_id IN (SELECT value FROM json_each(?1)) AND e.kind='tron.interaction-view' GROUP BY e.session_id,request_code) SELECT requests.session_id,SUM(CASE WHEN i.answered=0 THEN 1 ELSE 0 END),SUM(i.id IS NULL) FROM requests LEFT JOIN ef_interactions i ON i.session_id=requests.session_id AND i.id=requests.request_code GROUP BY requests.session_id")?;
                q.query_map([codes], |r| Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?,r.get::<_,u32>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()
            }).await.map_err(|_| ChatError::StorageFailure)?.into_iter().map(|(session,pending,missing)| {
                if missing != 0 { Err(ChatError::StorageFailure) } else { Ok((session,pending)) }
            }).collect()
        })
    }
    fn create_session(&mut self, session: &ChatSession) -> Result<(), ChatError> {
        let mut record = fw::SessionRecord::new(&session.title);
        record.id = session.session_code.clone();
        let link = Link::from_session(session);
        let id = record.id.clone();
        let value = serde_json::to_string(&link).map_err(|_| ChatError::StorageFailure)?;
        blocking(async {
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let sessions = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            db.transaction(async {
                sessions.create(record).await?;
                db.call(move|c|c.execute("INSERT INTO workspace_session_links(session_id,workspace_code,business_json) VALUES(?1,?2,?3)",params![id,link.workspace_code,value]))
                    .await.map_err(|e|fw::StateError::Backend(e.to_string()))?; Ok(())
            }).await.map_err(chat_error)
        })
    }
    fn delete_session(&mut self, id: &str) -> Result<(), ChatError> {
        blocking(async {
            self.link(id).await?;
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let sessions = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            let code = id.to_owned();
            self.host.telemetry().await.map_err(|_| ChatError::StorageFailure)?;
            let telemetry_path = self.host.telemetry_database_path().to_string_lossy().into_owned();
            db.call(move |c| {
                let attached: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM pragma_database_list WHERE name='telemetry')",[],|r|r.get(0))?;
                if !attached { c.execute("ATTACH DATABASE ?1 AS telemetry",[telemetry_path])?; }
                Ok::<_,rusqlite::Error>(())
            }).await.map_err(|_| ChatError::StorageFailure)?;
            db.transaction(async {
                let record=sessions.get(id).await?;
                if record.status!=fw::SessionStatus::Archived {return Err(fw::StateError::Invalid("session must be archived".into()));}
                if self.framework().await.map_err(|e|fw::StateError::Backend(e.to_string()))?.goals().map_err(|e|fw::StateError::Backend(e.to_string()))?.active(id).await?.is_some_and(|goal| goal.status == fw::GoalStatus::Active) {
                    return Err(fw::StateError::Busy);
                }
                sessions.delete(id).await?;
                db.call(move|c| {
                    delete_session_telemetry(c, &code)?;
                    c.execute("DELETE FROM resource_chat_session_attachments WHERE session_code=?1",[&code])?;
                    for (table, sql) in [
                        ("chat_session_auto_model_routes", "DELETE FROM chat_session_auto_model_routes WHERE session_code=?1"),
                        ("provider_model_run_snapshots", "DELETE FROM provider_model_run_snapshots WHERE session_code=?1"),
                        ("automation_execution_records", "UPDATE automation_execution_records SET session_code=NULL WHERE session_code=?1"),
                    ] {
                        if c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",[table],|r|r.get::<_,bool>(0))? { c.execute(sql,[&code])?; }
                    }
                    c.execute("DELETE FROM workspace_session_links WHERE session_id=?1",[&code])?;
                    Ok::<_,rusqlite::Error>(())
                }).await.map_err(|e|fw::StateError::Backend(e.to_string()))?; Ok(())
            }).await.map_err(chat_error)
        })
    }
    fn list_sessions(&mut self) -> Result<Vec<ChatSession>, ChatError> {
        blocking(async {
            self.framework().await?;
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let raw = db
                .call_read(|c| -> Result<Vec<SessionProjection>, ChatError> {
                    let mut statement = c
                        .prepare(&format!("{SESSION_PROJECTION_SQL} ORDER BY l.session_id"))
                        .map_err(|_| ChatError::StorageFailure)?;
                    statement
                        .query_map([], session_row)
                        .map_err(|_| ChatError::StorageFailure)?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| ChatError::StorageFailure)
                })
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            let mut result = raw
                .into_iter()
                .map(chat_session_from_row)
                .collect::<Result<Vec<_>, _>>()?;
            result.sort_by(|a, b| {
                b.updated_at
                    .cmp(&a.updated_at)
                    .then_with(|| b.session_code.cmp(&a.session_code))
            });
            Ok(result)
        })
    }
    /// Archived listing pushes filtering, ordering, counting and paging into
    /// SQL so each page costs one read instead of replaying every session.
    fn list_archived_sessions(
        &mut self,
        workspace_code: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<ChatSession>, u64), ChatError> {
        blocking(async {
            self.framework().await?;
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let workspace = workspace_code.map(str::to_owned);
            let filter = "WHERE s.status IN ('Archived','ARCHIVED') \
                 AND (?1 IS NULL OR l.workspace_code=?1) \
                 AND substr(coalesce(json_extract(l.business_json,'$.source_code'),''),1,7) <> '__dude:'";
            let rows = db
                .call_read(move |c| -> Result<(Vec<SessionProjection>, u64), ChatError> {
                    let total: u64 = c
                        .query_row(
                            &format!(
                                "SELECT count(*) FROM workspace_session_links l \
                                 JOIN ef_sessions s ON s.id=l.session_id {filter}"
                            ),
                            params![workspace],
                            |r| r.get(0),
                        )
                        .map_err(|_| ChatError::StorageFailure)?;
                    let mut statement = c
                        .prepare(&format!("{SESSION_PROJECTION_SQL} {filter} \
                            ORDER BY json_extract(s.record,'$.updated_at_ms') DESC, l.session_id DESC \
                            LIMIT ?2 OFFSET ?3"))
                        .map_err(|_| ChatError::StorageFailure)?;
                    let rows = statement
                        .query_map(
                            params![workspace, i64::from(limit), i64::from(offset)],
                            session_row,
                        )
                        .map_err(|_| ChatError::StorageFailure)?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| ChatError::StorageFailure)?;
                    Ok((rows, total))
                })
                .await
                .map_err(|_| ChatError::StorageFailure)?;
            let total = rows.1;
            let items = rows
                .0
                .into_iter()
                .map(chat_session_from_row)
                .collect::<Result<Vec<_>, _>>()?;
            Ok((items, total))
        })
    }
    fn get_session(&mut self, id: &str) -> Result<ChatSession, ChatError> {
        blocking(self.session(id))
    }
    fn find_source_session(&mut self,user:&str,source:&str)->Result<Option<ChatSession>,ChatError>{
        let user=user.to_owned();let source=source.to_owned();
        blocking(async{
            self.framework().await?;
            let database=self.host.shared_database().await.map_err(chat_error)?;
            let row=database.call_read(move|connection|->Result<Option<SessionProjection>,ChatError>{
                connection.query_row(&format!("{SESSION_PROJECTION_SQL} WHERE json_extract(l.business_json,'$.user_code')=?1 AND json_extract(l.business_json,'$.source_code')=?2 AND json_extract(s.record,'$.status') IN ('Active','ACTIVE') LIMIT 1"),params![user,source],session_row).optional().map_err(|_|ChatError::StorageFailure)
            }).await.map_err(|_|ChatError::StorageFailure)?;
            row.map(chat_session_from_row).transpose()
        })
    }
    fn update_session_status(
        &mut self,
        id: &str,
        status: ChatSessionStatus,
        _at: &str,
    ) -> Result<(), ChatError> {
        blocking(async {
            self.link(id).await?;
            let f = self.framework().await?;
            let s = f.sessions().map_err(|_| ChatError::StorageFailure)?;
            let db = self.host.shared_database().await.map_err(chat_error)?;
            db.transaction(async {
                if status == ChatSessionStatus::Archived {
                    if f.goals()
                        .map_err(|e| fw::StateError::Backend(e.to_string()))?
                        .active(id)
                        .await?
                        .is_some_and(|goal| goal.status == fw::GoalStatus::Active)
                    {
                        return Err(fw::StateError::Busy);
                    }
                    s.archive(id).await?;
                } else {
                    s.restore(id).await?;
                }
                Ok(())
            })
            .await
            .map_err(chat_error)?;
            Ok(())
        })
    }
    fn update_session_title(&mut self, id: &str, title: &str, _at: &str) -> Result<(), ChatError> {
        blocking(async {
            self.link(id).await?;
            let s = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            let mut record = s.get(id).await.map_err(chat_error)?;
            let expected = record.revision;
            record.title = title.into();
            s.update(record, expected).await.map_err(chat_error)?;
            Ok(())
        })
    }
    fn update_session_run(
        &mut self,
        id: &str,
        _run: &str,
        _status: &str,
        agent: Option<&str>,
        runtime: Option<&str>,
        _at: &str,
    ) -> Result<(), ChatError> {
        blocking(async {
            let db = self.host.shared_database().await.map_err(chat_error)?;
            let sessions = self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?;
            db.transaction(async {
                let mut link = self
                    .link(id)
                    .await
                    .map_err(|e| fw::StateError::Backend(e.to_string()))?;
                if let Some(agent) = agent {
                    link.agent_code = Some(agent.into());
                }
                let value = serde_json::to_string(&link)
                    .map_err(|e| fw::StateError::Backend(e.to_string()))?;
                let code = id.to_owned();
                db.call(move |c| {
                    c.execute(
                        "UPDATE workspace_session_links SET business_json=?2 WHERE session_id=?1",
                        params![code, value],
                    )
                })
                .await
                .map_err(|e| fw::StateError::Backend(e.to_string()))?;
                if let Some(runtime) = runtime {
                    let expected = sessions
                        .read_context(id, "tron.external-reference")
                        .await?
                        .map(|(revision, _)| revision);
                    sessions
                        .write_context(
                            id,
                            "tron.external-reference",
                            expected,
                            json!({"sessionId":runtime}),
                        )
                        .await?;
                }
                Ok(())
            })
            .await
            .map_err(chat_error)
        })
    }

    fn runtime_session_id(
        &mut self,
        id: &str,
        adapter: &str,
        agent: &str,
    ) -> Result<Option<String>, ChatError> {
        blocking(async {
            self.link(id).await?;
            let key = format!("tron.external/{adapter}/{agent}");
            Ok(self
                .framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?
                .read_context(id, &key)
                .await
                .map_err(chat_error)?
                .and_then(|(_, v)| v.as_str().map(str::to_owned)))
        })
    }
    fn upsert_runtime_session(
        &mut self,
        id: &str,
        adapter: &str,
        agent: &str,
        runtime: &str,
        _run: Option<&str>,
        _at: &str,
    ) -> Result<(), ChatError> {
        blocking(self.put_context(
            id,
            &format!("tron.external/{adapter}/{agent}"),
            json!(runtime),
        ))
    }
    fn create_message(&mut self, message: &ChatMessage) -> Result<(), ChatError> {
        blocking(async {
            self.link(&message.session_code).await?;
            let mut event = fw::SessionEvent::new(
                &message.session_code,
                "tron.message",
                serde_json::to_value(message).map_err(|_| ChatError::StorageFailure)?,
            );
            event.run_id = message
                .run_code
                .as_deref()
                .and_then(|id| fw::RunId::parse_str(id).ok());
            self.framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?
                .store
                .append_event(event)
                .await
                .map_err(chat_error)?;
            Ok(())
        })
    }
    fn list_messages(&mut self, id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        blocking(async {
            self.events(id, "tron.message")
                .await?
                .into_iter()
                .map(|e| serde_json::from_value(e.payload).map_err(|_| ChatError::StorageFailure))
                .collect()
        })
    }
    fn list_user_input_requests(
        &mut self,
        id: &str,
    ) -> Result<Vec<ChatUserInputRequest>, ChatError> {
        blocking(async {
            let mut requests = std::collections::BTreeMap::new();
            for e in self.events(id, "tron.interaction-view").await? {
                let request: ChatUserInputRequest =
                    serde_json::from_value(e.payload).map_err(|_| ChatError::StorageFailure)?;
                requests.insert(request.request_code.clone(), request);
            }
            let services = self
                .framework()
                .await?
                .services()
                .map_err(|_| ChatError::StorageFailure)?;
            let interactions = services
                .get::<fw::InteractionStoreService>()
                .ok_or(ChatError::StorageFailure)?;
            // One batched lookup for every pending request in the session.
            let ids = requests
                .keys()
                .map(|code| fw::RunId::parse_str(code))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| ChatError::StorageFailure)?;
            let records = interactions
                .0
                .get_many(id, &ids)
                .await
                .map_err(chat_error)?;
            let mut views = Vec::new();
            for (mut request, record) in requests.into_values().zip(records) {
                request.status = if record.response.is_some() {
                    ChatUserInputRequestStatus::Answered
                } else {
                    ChatUserInputRequestStatus::Pending
                };
                request.answered_at = record
                    .answered_at_unix_ms
                    .map(|v| crate::shared::time::timestamp_from_millis(v as i64));
                request.answers = record
                    .response
                    .and_then(|r| r.value.get("answers").and_then(Value::as_object).cloned())
                    .map(|answers| {
                        answers
                            .into_iter()
                            .map(|(question_code, value)| ChatUserInputAnswer {
                                question_code,
                                answer_kind: if value.get("option_id").is_some() {
                                    ChatUserInputAnswerKind::Option
                                } else if value.get("text").is_some() {
                                    ChatUserInputAnswerKind::Other
                                } else {
                                    ChatUserInputAnswerKind::Skipped
                                },
                                option_code: value["option_id"].as_str().map(str::to_owned),
                                other_text: value["text"].as_str().map(str::to_owned),
                            })
                            .collect()
                    });
                views.push(request);
            }
            Ok(views)
        })
    }
    fn create_user_input_request(
        &mut self,
        request: &ChatUserInputRequest,
    ) -> Result<(), ChatError> {
        blocking(async {
            self.framework()
                .await?
                .sessions()
                .map_err(|_| ChatError::StorageFailure)?
                .store
                .append_event(fw::SessionEvent::new(
                    &request.session_code,
                    "tron.interaction-view",
                    serde_json::to_value(request).map_err(|_| ChatError::StorageFailure)?,
                ))
                .await
                .map_err(chat_error)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod shared_transaction_tests {
    use super::*;
    use crate::adapters::outbound::persistence::sqlite::SqliteResourceRepository;

    fn fixture() -> (tempfile::TempDir, SqliteChatRepository) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tron.sqlite3");
        let path = path.to_str().unwrap();
        SqliteResourceRepository::connect(path).unwrap();
        let repository = SqliteChatRepository::connect(path).unwrap();
        (directory, repository)
    }
    fn session(id: &str) -> ChatSession {
        ChatSession {
            id: 1,
            session_code: id.into(),
            workspace_code: "ws-a".into(),
            project_code: None,
            user_code: None,
            app_code: None,
            task_code: None,
            source_type: ChatSessionSourceType::Manual,
            source_code: None,
            agent_code: None,
            system_agent_session_id: None,
            title: "transaction".into(),
            status: ChatSessionStatus::Active,
            last_run_code: None,
            last_run_status: None,
            created_at: "".into(),
            updated_at: "".into(),
        }
    }
    #[test]
    fn pending_counts_deduplicate_events_and_preserve_session_boundaries() {
        let (_directory, mut repository) = fixture();
        repository.create_session(&session("count-owner")).unwrap();
        repository.create_session(&session("count-other")).unwrap();
        let database = blocking(repository.host.shared_database()).unwrap();
        let pending = fw::RunId::new_v4().to_string();
        let answered = fw::RunId::new_v4().to_string();
        database.call_blocking(move |c| {
            for (id,done) in [(&pending,false),(&answered,true)] {
                c.execute("INSERT INTO ef_interactions(id,session_id,run_id,answered,record) VALUES(?1,'count-owner','run',?2,'{}')",params![id,done])?;
            }
            for (sequence,session,id) in [(101,"count-owner",&pending),(102,"count-owner",&pending),(103,"count-owner",&answered),(101,"count-other",&pending)] {
                let event=fw::SessionEvent::new(session,"tron.interaction-view",json!({"requestCode":id}));
                c.execute("INSERT INTO ef_events(id,session_id,sequence,kind,record) VALUES(?1,?2,?3,?4,?5)",params![event.id,session,sequence,event.kind,serde_json::to_string(&event).unwrap()])?;
            }
            Ok::<_,rusqlite::Error>(())
        }).unwrap();
        let counts = repository
            .pending_input_counts(&["count-owner".into(), "absent".into()])
            .unwrap();
        assert_eq!(counts.get("count-owner"), Some(&1));
        assert!(!counts.contains_key("absent"));
        assert!(matches!(
            repository.pending_input_counts(&["count-other".into()]),
            Err(ChatError::StorageFailure)
        ));
    }
    fn seed_telemetry(
        repository: &SqliteChatRepository,
    ) -> eliteforge_agent_official_plugin_storage_local::SqliteDatabase {
        blocking(repository.host.telemetry()).unwrap();
        blocking(
            eliteforge_agent_official_plugin_storage_local::SqliteDatabase::open(
                repository.host.telemetry_database_path(),
            ),
        )
        .unwrap()
    }
    fn insert_span(database: &eliteforge_agent_official_plugin_storage_local::SqliteDatabase, trace: &str, id: &str, parent: Option<&str>, owner: Option<&str>) {
        let data = json!({"name":"tron.session.run","traceId":trace,"spanId":id,"parentSpanId":parent,"attributes":owner.map(|value|json!({"tron.session.code":value})).unwrap_or_else(||json!({}))}).to_string();
        let trace = trace.to_owned();
        let id = id.to_owned();
        let parent = parent.map(str::to_owned);
        database.call_blocking(move |c| {
            c.execute("INSERT INTO ef_telemetry_spans(trace_id,span_id,ended_at,data) VALUES(?1,?2,0,?3)",params![trace,id,data])?;
            c.execute("INSERT INTO ef_telemetry_span_rollups(trace_id,span_id,parent_id,own_span_count,span_count) VALUES(?1,?2,?3,1,1)",params![trace,id,parent])?;
            Ok::<_,rusqlite::Error>(())
        }).unwrap();
    }
    #[test]
    fn ordinary_archived_history_excludes_dude_work_even_in_same_workspace() {
        let (_directory, mut repository) = fixture();
        let user = session("archived-user");
        repository.create_session(&user).unwrap();
        let mut dude = session("archived-dude");
        dude.source_code = Some("__dude:main-dude".into());
        repository.create_session(&dude).unwrap();
        // Native sessions are created Active; exercise the actual archive lifecycle
        // rather than assigning a status only to the product projection fixture.
        for code in [&user.session_code, &dude.session_code] {
            repository
                .update_session_status(code, ChatSessionStatus::Archived, "")
                .unwrap();
        }
        let persisted = repository.list_sessions().unwrap();
        assert_eq!(persisted.len(), 2);
        assert!(
            persisted
                .iter()
                .all(|session| session.status == ChatSessionStatus::Archived)
        );
        let (items, total) = repository
            .list_archived_sessions(Some("ws-a"), 0, 100)
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].session_code, user.session_code);
        let (_, total) = repository.list_archived_sessions(None, 0, 100).unwrap();
        assert_eq!(total, 1);
        assert!(repository.get_session("archived-dude").is_ok());
    }

    #[test]
    fn workspace_link_failure_rolls_back_framework_session_and_events() {
        let (_directory, mut repository) = fixture();
        let framework = blocking(repository.framework()).unwrap();
        let database = blocking(repository.host.shared_database()).unwrap();
        database.call_blocking(|c| c.execute_batch("CREATE TRIGGER reject_link BEFORE INSERT ON workspace_session_links BEGIN SELECT RAISE(ABORT,'injected link failure'); END;")).unwrap();
        assert!(
            repository
                .create_session(&session("atomic-create"))
                .is_err()
        );
        assert!(matches!(
            blocking(framework.sessions().unwrap().get("atomic-create")),
            Err(fw::StateError::NotFound)
        ));
        database
            .call_blocking(|c| c.execute_batch("DROP TRIGGER reject_link"))
            .unwrap();
        repository
            .create_session(&session("atomic-create"))
            .unwrap();
        assert_eq!(
            repository
                .get_session("atomic-create")
                .unwrap()
                .workspace_code,
            "ws-a"
        );
    }
    #[test]
    fn delete_failure_rolls_back_session_history_and_business_link_then_can_retry() {
        let (_directory, mut repository) = fixture();
        repository
            .create_session(&session("atomic-delete"))
            .unwrap();
        let framework = blocking(repository.framework()).unwrap();
        blocking(
            framework
                .sessions()
                .unwrap()
                .store
                .append_event(fw::SessionEvent::new(
                    "atomic-delete",
                    "test.history",
                    json!({"text":"preserve"}),
                )),
        )
        .unwrap();
        repository
            .update_session_status("atomic-delete", ChatSessionStatus::Archived, "")
            .unwrap();
        let database = blocking(repository.host.shared_database()).unwrap();
        let telemetry = seed_telemetry(&repository);
        telemetry.call_blocking(|c| c.execute("INSERT INTO ef_telemetry_traces(trace_id,started_at,status_code,attributes) VALUES('trace-atomic',0,0,'{\"tron.session.code\":\"atomic-delete\"}')",[])).unwrap();
        insert_span(&telemetry,"trace-atomic","root",None,Some("atomic-delete"));
        database.call_blocking(|c| c.execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON workspace_session_links BEGIN SELECT RAISE(ABORT,'injected delete failure'); END;")).unwrap();
        assert!(repository.delete_session("atomic-delete").is_err());
        let trace_count: i64 = telemetry.call_blocking(|c| c.query_row("SELECT count(*) FROM ef_telemetry_traces WHERE trace_id='trace-atomic'",[],|row|row.get(0))).unwrap();
        assert_eq!(trace_count,1);
        assert_eq!(
            repository.get_session("atomic-delete").unwrap().status,
            ChatSessionStatus::Archived
        );
        assert!(
            blocking(framework.events().unwrap().list("atomic-delete", 0, 100))
                .unwrap()
                .iter()
                .any(|e| e.kind == "test.history")
        );
        database
            .call_blocking(|c| c.execute_batch("DROP TRIGGER reject_delete"))
            .unwrap();
        repository.delete_session("atomic-delete").unwrap();
        let trace_count: i64 = telemetry.call_blocking(|c| c.query_row("SELECT count(*) FROM ef_telemetry_traces WHERE trace_id='trace-atomic'",[],|row|row.get(0))).unwrap();
        assert_eq!(trace_count,0);
        assert!(matches!(
            blocking(framework.sessions().unwrap().get("atomic-delete")),
            Err(fw::StateError::NotFound)
        ));
        assert!(matches!(
            repository.get_session("atomic-delete"),
            Err(ChatError::SessionNotFound)
        ));
    }
    #[test]
    fn deleting_archived_session_preserves_other_session_in_shared_trace() {
        let (_directory, mut repository) = fixture();
        repository.create_session(&session("session-a")).unwrap();
        repository.create_session(&session("session-b")).unwrap();
        repository.update_session_status("session-a",ChatSessionStatus::Archived,"").unwrap();
        let database = seed_telemetry(&repository);
        database.call_blocking(|c| c.execute("INSERT INTO ef_telemetry_traces(trace_id,started_at,status_code,attributes) VALUES('shared',0,0,'{\"tron.session.code\":\"session-a\",\"tron.trace.page\":\"true\"}')",[])).unwrap();
        database.call_blocking(|c| c.execute("INSERT INTO ef_telemetry_trace_rollups(trace_id,span_count) VALUES('shared',4)",[])).unwrap();
        insert_span(&database,"shared","page",None,None);
        insert_span(&database,"shared","a",Some("page"),Some("session-a"));
        insert_span(&database,"shared","a-child",Some("a"),None);
        insert_span(&database,"shared","b",Some("a"),Some("session-b"));
        repository.delete_session("session-a").unwrap();
        let survivors: Vec<(String,Option<String>)> = database.call_blocking(|c| c.prepare("SELECT span_id,json_extract(data,'$.parentSpanId') FROM ef_telemetry_spans WHERE trace_id='shared' ORDER BY span_id")?.query_map([],|row|Ok((row.get(0)?,row.get(1)?)))?.collect()).unwrap();
        assert_eq!(survivors,vec![("b".into(),None),("page".into(),None)]);
        let (session, trace_count, page_count): (Option<String>,i64,i64) = database.call_blocking(|c| {
            Ok::<_,rusqlite::Error>((
                c.query_row("SELECT json_extract(attributes,'$.\"tron.session.code\"') FROM ef_telemetry_traces WHERE trace_id='shared'",[],|row|row.get(0))?,
                c.query_row("SELECT span_count FROM ef_telemetry_trace_rollups WHERE trace_id='shared'",[],|row|row.get(0))?,
                c.query_row("SELECT span_count FROM ef_telemetry_span_rollups WHERE trace_id='shared' AND span_id='page'",[],|row|row.get(0))?,
            ))
        }).unwrap();
        assert_eq!(session,None);
        assert_eq!(trace_count,2);
        assert_eq!(page_count,1);
        assert!(repository.get_session("session-b").is_ok());
    }
    #[test]
    fn synchronous_business_sql_joins_the_framework_transaction() {
        let (_directory, repository) = fixture();
        let database = blocking(repository.host.shared_database()).unwrap();
        let shared = super::super::framework_storage::SharedDatabase::open(
            directory_path(&_directory).to_str().unwrap(),
        )
        .unwrap();
        let framework = blocking(repository.framework()).unwrap();
        let result: Result<(), fw::StateError> = blocking(database.transaction(async {
            framework
                .sessions()
                .unwrap()
                .create(fw::SessionRecord::new("rollback"))
                .await?;
            shared
                .call(
                    |c| {
                        c.execute(
                            "INSERT INTO workspace_session_links VALUES('joined','ws-a','{}')",
                            [],
                        )
                    },
                    || rusqlite::Error::InvalidQuery,
                )
                .unwrap();
            Err(fw::StateError::Invalid("rollback whole unit".into()))
        }));
        assert!(result.is_err());
        let count: i64 = database
            .call_blocking(|c| {
                c.query_row(
                    "SELECT count(*) FROM workspace_session_links WHERE session_id='joined'",
                    [],
                    |r| r.get(0),
                )
            })
            .unwrap();
        assert_eq!(count, 0);
        assert!(
            blocking(framework.sessions().unwrap().list(None, 100))
                .unwrap()
                .is_empty()
        );
    }
    fn directory_path(directory: &tempfile::TempDir) -> std::path::PathBuf {
        directory.path().join("tron.sqlite3")
    }
}
