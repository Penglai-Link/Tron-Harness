//! Resolve explicit local project roots and reject deletion of shared paths.
use super::*;
use std::path::{Path, PathBuf};

fn normalized(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}
fn overlaps(a: &Path, b: &Path) -> bool { a.starts_with(b) || b.starts_with(a) }

pub(super) fn paths(c: &rusqlite::Connection, container: &ChatContainer, sessions: &[String]) -> rusqlite::Result<Vec<String>> {
    let mut paths = Vec::new();
    match container {
        ChatContainer::Directory { directory_code, .. } => {
            paths.push(c.query_row("SELECT canonical_path FROM directories WHERE directory_code=?1 AND execution_host='server'", [directory_code], |r| r.get::<_, String>(0))?);
        }
        ChatContainer::Project { project_code, .. } => {
            let mut q = c.prepare("SELECT d.canonical_path FROM directories d JOIN topic_references r ON r.target_code=d.directory_code AND r.kind='Directory' WHERE r.project_code=?1 AND d.execution_host='server' UNION SELECT a.local_directory FROM apps a JOIN project_app_links l ON l.app_code=a.app_code WHERE l.project_code=?1 AND a.local_directory<>''")?;
            paths.extend(q.query_map([project_code], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?);
            let mut q = c.prepare("SELECT l.session_id,d.canonical_path FROM workspace_session_links l JOIN directories d ON d.directory_code=json_extract(l.business_json,'$.working_directory_code') WHERE d.execution_host='server'")?;
            for row in q.query_map([], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?)))? {
                let (session, path) = row?;
                if sessions.binary_search(&session).is_ok() { paths.push(path); }
            }
        }
    }
    // Preserve registered canonical paths: staging rejects replaced symlinks.
    paths.sort(); paths.dedup();
    Ok(paths)
}

pub(super) fn exclusive(c: &rusqlite::Connection, container: &ChatContainer, snapshot: &ChatContainerSnapshot) -> rusqlite::Result<bool> {
    let roots = snapshot.disk_paths.iter().map(|s| normalized(s)).collect::<Vec<_>>();
    let project = match container { ChatContainer::Project { project_code, .. } => Some(project_code.as_str()), _ => None };
    let conflict = |path: &str| roots.iter().any(|r| overlaps(r, &normalized(path)));
    // Include archived chats and other workspaces, as the physical disk is shared.
    let mut q = c.prepare("SELECT l.session_id,d.canonical_path FROM workspace_session_links l JOIN directories d ON d.directory_code=json_extract(l.business_json,'$.directory_code') OR d.directory_code=json_extract(l.business_json,'$.working_directory_code') WHERE d.execution_host='server'")?;
    for row in q.query_map([], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?)))? {
        let (session,path) = row?;
        if snapshot.session_codes.binary_search(&session).is_err() && conflict(&path) {
            tracing::warn!(%session,%path,"disk deletion rejected: another chat uses the directory");
            return Ok(false);
        }
    }
    let mut q = c.prepare("SELECT r.project_code,d.canonical_path FROM topic_references r JOIN directories d ON d.directory_code=r.target_code WHERE r.kind='Directory' AND d.execution_host='server' UNION SELECT l.project_code,a.local_directory FROM project_app_links l JOIN apps a ON a.app_code=l.app_code WHERE a.local_directory<>''")?;
    for row in q.query_map([], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?)))? {
        let (owner,path) = row?;
        if Some(owner.as_str()) != project && conflict(&path) {
            tracing::warn!(%owner,%path,"disk deletion rejected: another topic uses the directory");
            return Ok(false);
        }
    }
    // An unlinked application is still a standalone resource; only the deleting
    // topic's explicitly linked applications can have their source removed.
    let mut q = c.prepare("SELECT a.local_directory FROM apps a WHERE a.local_directory<>'' AND NOT EXISTS (SELECT 1 FROM project_app_links l WHERE l.app_code=a.app_code AND l.project_code=?1)")?;
    for path in q.query_map([project.unwrap_or("")], |r| r.get::<_, String>(0))? {
        if conflict(&path?) { return Ok(false); }
    }
    Ok(true)
}
