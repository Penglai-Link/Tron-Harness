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
                let mut query=c.prepare("SELECT thread_code,workspace_code,owner_kind,owner_code,record_json FROM collaboration_threads WHERE user_code=?1")?;
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
        // Native subagents are indexed under their trusted root session, including descendants.
        for (root, (workspace, owner, lineage)) in links.clone() {
            if owner.as_deref() == Some(memory_dude)
                || lineage.iter().any(|code| code == memory_dude)
            {
                continue;
            }
            let children = self
                .host
                .subagents(&root, &profile_user)
                .await
                .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
            for child in children {
                links
                    .entry(child.record.session_id)
                    .or_insert_with(|| (workspace.clone(), owner.clone(), lineage.clone()));
            }
        }
        let history = framework
            .history()
            .map_err(|e| DudeMemoryError::Storage(e.to_string()))?;
        let mut sources = Vec::new();
        for (session, (workspace, owner, lineage)) in links {
            if owner.as_deref() == Some(memory_dude)
                || lineage.iter().any(|code| code == memory_dude)
            {
                continue;
            }
            let mut cursor = None;
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
                        append_chunks(
                            &mut sources,
                            &session,
                            &message.id,
                            &workspace,
                            &owner,
                            &lineage,
                            MemorySourceKind::Conversation,
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
                        append_chunks(
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
                    break;
                }
            }
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
) {
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
        if sources.len() >= 64
            || sources.iter().map(|s| s.excerpt.len()).sum::<usize>() + source.excerpt.len()
                > 128 * 1024
        {
            return;
        }
        sources.push(source);
    }
}
