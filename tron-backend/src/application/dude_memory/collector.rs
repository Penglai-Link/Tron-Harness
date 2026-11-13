//! Reads durable Framework history with product-owned provenance, never process logs.
use crate::{application::agent::FrameworkHost, domain::dude_memory::*};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct DudeMemorySourceCollector {
    host: FrameworkHost,
}
impl DudeMemorySourceCollector {
    pub fn new(host: FrameworkHost) -> Self {
        Self { host }
    }
    pub async fn collect(
        &self,
        user: &str,
        memory_dude: &str,
    ) -> Result<Vec<MemorySource>, DudeMemoryError> {
        self.collect_incremental(user, memory_dude, &BTreeMap::new())
            .await
    }
    pub async fn collect_incremental(
        &self,
        user: &str,
        memory_dude: &str,
        cursors: &BTreeMap<String, String>,
    ) -> Result<Vec<MemorySource>, DudeMemoryError> {
        let database = self
            .host
            .shared_database()
            .await
            .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        let profile_user = user.to_owned();
        let user = user.to_owned();
        let mut links=database.call(move|c| {
            let mut sessions=BTreeMap::<String,(String,Option<String>,Vec<String>)>::new();
            let mut query=c.prepare("SELECT session_id,workspace_code,business_json FROM workspace_session_links")?;
            for row in query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))? {
                let(id,workspace,json)=row?;let value:Value=serde_json::from_str(&json).unwrap_or(Value::Null);
                // Links are trusted persisted host state, not model-provided task arguments.
                if value["user_code"].as_str()!=Some(&user) && value["userCode"].as_str()!=Some(&user) { continue; }
                let owner=value["ownerCode"].as_str().or_else(||value["owner"]["code"].as_str()).map(str::to_owned);
                sessions.insert(id,(workspace,owner,Vec::new()));
            }
            let exists=|table:&str|c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",[table],|r|r.get::<_,bool>(0));
            if exists("collaboration_threads")? {
                let mut query=c.prepare("SELECT session_id,workspace_code,owner_kind,owner_code,record_json FROM collaboration_threads WHERE user_code=?1")?;
                for row in query.query_map([&user],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?)))? {
                    let(id,workspace,kind,owner,json)=row?;let value:Value=serde_json::from_str(&json).unwrap_or(Value::Null);
                    let lineage=value["lineageDudeCodes"].as_array().map(|a|a.iter().filter_map(|v|v.as_str().map(str::to_owned)).collect()).unwrap_or_default();
                    sessions.insert(id,(workspace,(kind.eq_ignore_ascii_case("DUDE")).then_some(owner),lineage));
                }
            }
            if exists("dude_sessions")? {
                let mut query=c.prepare("SELECT session_code,dude_code,record_json FROM dude_sessions")?;
                for row in query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))? {
                    let(id,owner,json)=row?;let value:Value=serde_json::from_str(&json).unwrap_or(Value::Null);
                    // Platform sessions have no workspace; user-scoped facts still use the current profile.
                    if value["userCode"].as_str()!=Some(user.as_str()) && !sessions.contains_key(&id) { continue; }
                    let workspace=value["workspaceCode"].as_str().unwrap_or("").to_owned();
                    sessions.insert(id,(workspace,Some(owner),Vec::new()));
                }
            }
            Ok::<_,rusqlite::Error>(sessions)
        }).await.map_err(|e|DudeMemoryError::Storage(e.to_string()))?;
        let framework = self
            .host
            .storage_framework()
            .await
            .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        // Read the trusted child-session index once; do not fetch unused run details.
        let roots = links
            .iter()
            .filter(|(_, (_, owner, lineage))| {
                owner.as_deref() != Some(memory_dude)
                    && !lineage.iter().any(|code| code == memory_dude)
            })
            .map(|(root, _)| root.clone())
            .collect::<Vec<_>>();
        let selected =
            serde_json::to_string(&roots).map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        let children = database.call_read(move |c| {
            let mut query = c.prepare("SELECT session_id,value FROM ef_session_context WHERE key='tron.subagents' AND session_id IN (SELECT value FROM json_each(?1)) ORDER BY session_id")?;
            query.query_map([selected], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<Result<Vec<_>,_>>()
        }).await.map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        for (root, raw) in children {
            let records: Vec<crate::application::agent::SubagentRecord> =
                serde_json::from_str(&raw).map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
            let Some((workspace, owner, lineage)) = links.get(&root).cloned() else {
                continue;
            };
            for child in records
                .into_iter()
                .filter(|record| record.user_code.as_deref() == Some(&profile_user))
            {
                links
                    .entry(child.session_id)
                    .or_insert_with(|| (workspace.clone(), owner.clone(), lineage.clone()));
            }
        }
        let selected = serde_json::to_string(&links.keys().collect::<Vec<_>>())
            .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        let versions = database.call_read(move |c| {
            let mut query = c.prepare("SELECT s.id,COALESCE(v.version,'0') FROM ef_sessions s LEFT JOIN ef_history_versions v ON v.session_id=s.id WHERE s.id IN (SELECT value FROM json_each(?1))")?;
            query.query_map([selected], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))?.collect::<Result<BTreeMap<_,_>,_>>()
        }).await.map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        let history = framework
            .history()
            .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        let mut sources = Vec::new();
        let mut completed_versions = BTreeMap::new();
        for (session, (workspace, owner, lineage)) in links {
            if owner.as_deref() == Some(memory_dude)
                || lineage.iter().any(|code| code == memory_dude)
            {
                continue;
            }
            let progress_key = format!("history-v1:{session}");
            let stamp = |version: &str| {
                serde_json::to_string(&(version, &workspace, &owner, &lineage))
                    .map_err(|e| DudeMemoryError::Storage(e.to_string()))
            };
            if let Some(version) = versions.get(&session)
                && cursors.get(&progress_key) == Some(&stamp(version)?)
            {
                continue;
            }
            let mut cursor = None;
            let mut fully_processed = true;
            loop {
                let page = history
                    .read_session(
                        &session,
                        eliteforge_agent_core::HistoryQuery {
                            limit: 100,
                            cursor: cursor.take(),
                        },
                    )
                    .await
                    .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
                for message in &page.records.messages {
                    if message.status != eliteforge_agent_core::HistoryStatus::Completed {
                        continue;
                    }
                    let text = message
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            eliteforge_agent_core::HistoryContentBlock::Text { text } => {
                                Some(text.as_str())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !text.trim().is_empty() {
                        fully_processed &= append_chunks(
                            &mut sources,
                            &session,
                            &message.id,
                            &workspace,
                            &owner,
                            &lineage,
                            if message.role == eliteforge_agent_core::Role::User {
                                MemorySourceKind::UserStatement
                            } else {
                                MemorySourceKind::Conversation
                            },
                            &format!("{} {:?}: {}", message.created_at_ms, message.role, text),
                            cursors,
                        );
                        if sources.len() >= 64
                            || sources.iter().map(|s| s.excerpt.len()).sum::<usize>() >= 112 * 1024
                        {
                            return Ok(sources);
                        }
                    }
                }
                for tool in &page.records.tool_invocations {
                    if tool.tool_name.starts_with("memory_") || tool.tool_name.starts_with("dude_")
                    {
                        continue;
                    }
                    if tool.status == eliteforge_agent_core::HistoryStatus::Completed
                        && tool.is_error != Some(true)
                    {
                        let json = serde_json::to_string(tool)
                            .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
                        fully_processed &= append_chunks(
                            &mut sources,
                            &session,
                            &tool.id.to_string(),
                            &workspace,
                            &owner,
                            &lineage,
                            MemorySourceKind::TaskResult,
                            &format!("Verified successful tool outcome: {json}"),
                            cursors,
                        );
                        if sources.len() >= 64
                            || sources.iter().map(|s| s.excerpt.len()).sum::<usize>() >= 112 * 1024
                        {
                            return Ok(sources);
                        }
                    }
                }
                cursor = page.next_cursor;
                if cursor.is_none() {
                    if fully_processed {
                        // Only acknowledge a fully scanned version whose chunks
                        // are already committed. Pending/failed batches cannot
                        // advance this gate. A concurrent write changes version
                        // and makes the next collection read it again.
                        completed_versions.insert(progress_key.clone(), stamp(&page.version)?);
                    }
                    break;
                }
            }
        }
        if !completed_versions.is_empty() {
            let dude = memory_dude.to_owned();
            let versions = serde_json::to_string(&completed_versions)
                .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
            database.call(move |c| {
                c.execute("INSERT INTO dude_memory_cursors(user_code,dude_code,source_key,source_version) SELECT ?1,?2,key,value FROM json_each(?3) WHERE true ON CONFLICT(user_code,dude_code,source_key) DO UPDATE SET source_version=excluded.source_version", rusqlite::params![profile_user,dude,versions])
            }).await.map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        }
        Ok(sources)
    }
}

fn append_chunks(
    sources: &mut Vec<MemorySource>,
    session: &str,
    record: &str,
    workspace: &str,
    owner: &Option<String>,
    lineage: &[String],
    kind: MemorySourceKind,
    text: &str,
    cursors: &BTreeMap<String, String>,
) -> bool {
    let mut processed = true;
    let chars: Vec<_> = text.chars().collect();
    for (index, chunk) in chars.chunks(8192).enumerate() {
        let excerpt: String = chunk.iter().collect();
        let source = MemorySource {
            source_code: format!("{session}/{record}/{index}"),
            workspace_code: workspace.into(),
            version: hex::encode(Sha256::digest(excerpt.as_bytes())),
            excerpt,
            root_owner_dude_code: owner.clone(),
            lineage_dude_codes: lineage.to_vec(),
            kind,
        };
        if cursors.get(&source.identity()) == Some(&source.version) {
            continue;
        }
        processed = false;
        if sources.len() >= 64
            || sources.iter().map(|s| s.excerpt.len()).sum::<usize>() + source.excerpt.len()
                > 128 * 1024
        {
            return false;
        }
        sources.push(source);
    }
    processed
}
