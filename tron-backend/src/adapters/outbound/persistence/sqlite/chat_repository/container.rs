//! Container mutations share the Framework transaction and reject a stale scope.
use super::*;

impl SqliteChatRepository {
    pub(super) async fn read_container_snapshot(
        &self,
        container: &ChatContainer,
    ) -> Result<ChatContainerSnapshot, ChatError> {
        self.framework().await?;
        let db = self.host.shared_database().await.map_err(chat_error)?;
        let container = container.clone();
        db.call(move |c| -> rusqlite::Result<ChatContainerSnapshot> {
            let (workspace, project, directory_codes) = match &container {
                ChatContainer::Project { workspace_code, project_code } => {
                    c.query_row("SELECT project_code FROM projects WHERE project_code=?1 AND workspace_code=?2", params![project_code, workspace_code], |row| row.get::<_, String>(0))?;
                    (workspace_code, Some(project_code), Vec::new())
                }
                ChatContainer::Directory { workspace_code, directory_code } => {
                    let (host, root): (String, String) = c.query_row("SELECT execution_host,canonical_path FROM directories WHERE directory_code=?1 AND workspace_code=?2", params![directory_code, workspace_code], |row| Ok((row.get(0)?, row.get(1)?)))?;
                    let mut query = c.prepare("SELECT directory_code,canonical_path FROM directories WHERE workspace_code=?1 AND execution_host=?2 ORDER BY directory_code")?;
                    let rows = query.query_map(params![workspace_code, host], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?.collect::<Result<Vec<_>, _>>()?;
                    let codes = rows.into_iter().filter(|(_, path)| std::path::Path::new(path).starts_with(&root)).map(|(code, _)| code).collect::<Vec<_>>();
                    (workspace_code, None, codes)
                }
            };
            let mut query = c.prepare("SELECT l.session_id,l.business_json FROM workspace_session_links l JOIN ef_sessions s ON s.id=l.session_id WHERE l.workspace_code=?1 ORDER BY l.session_id")?;
            let rows = query.query_map([workspace], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?.collect::<Result<Vec<_>, _>>()?;
            let mut session_codes = Vec::new();
            for (code, json) in rows {
                let link: Link = serde_json::from_str(&json).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let included = if let Some(project) = project {
                    link.project_code.as_ref() == Some(project)
                } else {
                    link.directory_code.as_ref().is_some_and(|code| directory_codes.contains(code))
                };
                if included { session_codes.push(code); }
            }
            let mut query = c.prepare("SELECT session_code,attachment_code FROM resource_chat_session_attachments WHERE workspace_code=?1 ORDER BY session_code,attachment_code")?;
            let attachments = query.query_map([workspace], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?.collect::<Result<Vec<_>, _>>()?;
            let attachment_codes = attachments.into_iter().filter(|(session, _)| session_codes.binary_search(session).is_ok()).map(|(_, code)| code).collect();
            Ok(ChatContainerSnapshot { directory_codes, session_codes, attachment_codes })
        }).await.map_err(|_| ChatError::StorageFailure)
    }

    pub(super) async fn apply_container_mutation(
        &self,
        container: &ChatContainer,
        expected: &ChatContainerSnapshot,
        delete: bool,
    ) -> Result<(), ChatError> {
        let framework = self.framework().await?;
        let sessions = framework
            .sessions()
            .map_err(|_| ChatError::StorageFailure)?;
        let goals = framework.goals().map_err(|_| ChatError::StorageFailure)?;
        let db = self.host.shared_database().await.map_err(chat_error)?;
        // Stage only session-owned directories. User project directories are preserved.
        let mut staged = None;
        let result = db.transaction(async {
            let actual = self.read_container_snapshot(container).await.map_err(|e| fw::StateError::Backend(e.to_string()))?;
            if &actual != expected { return Err(fw::StateError::Conflict); }
            for code in &actual.session_codes {
                if goals.active(code).await?.is_some_and(|goal| goal.status == fw::GoalStatus::Active) { return Err(fw::StateError::Busy); }
            }
            if delete {
                let mut owners = Vec::new();
                for code in &actual.session_codes {
                    let link = self.link(code).await.map_err(|e| fw::StateError::Backend(e.to_string()))?;
                    owners.push((code.clone(), link.user_code));
                }
                staged = Some(self.host.stage_session_working_directories(owners).await.map_err(|error| {
                    tracing::error!(%error, "session directory cleanup staging failed");
                    fw::StateError::Backend("session_directory_cleanup_failed".into())
                })?);
            }
            for code in &actual.session_codes {
                if delete {
                    self.delete_session_records(code, false).await.map_err(|e| fw::StateError::Backend(e.to_string()))?;
                } else if sessions.get(code).await?.status != fw::SessionStatus::Archived {
                    sessions.archive(code).await?;
                }
            }
            let container = container.clone();
            db.call(move |c| -> rusqlite::Result<()> {
                match container {
                    ChatContainer::Project { workspace_code, project_code } => {
                        if delete {
                            c.execute("DELETE FROM projects WHERE project_code=?1 AND workspace_code=?2", params![project_code, workspace_code])?;
                        } else {
                            c.execute("UPDATE projects SET status='ARCHIVED',archived_at=CAST(strftime('%s','now') AS INTEGER)*1000,updated_at=CAST(strftime('%s','now') AS INTEGER)*1000 WHERE project_code=?1 AND workspace_code=?2", params![project_code, workspace_code])?;
                        }
                    }
                    ChatContainer::Directory { workspace_code, .. } => {
                        for code in actual.directory_codes {
                            if delete {
                                c.execute("UPDATE topic_memos SET revision=revision+1 WHERE project_code IN (SELECT project_code FROM topic_references WHERE kind='Directory' AND target_code=?1)", [&code])?;
                                c.execute("DELETE FROM topic_references WHERE kind='Directory' AND target_code=?1", [&code])?;
                                // Other topics may use this directory only for execution.
                                // Keep those chats; remove their now-invalid directory reference.
                                c.execute("UPDATE workspace_session_links SET business_json=json_set(business_json,'$.working_directory_code',NULL) WHERE workspace_code=?1 AND json_extract(business_json,'$.working_directory_code')=?2", params![workspace_code, code])?;
                                c.execute("DELETE FROM directories WHERE directory_code=?1 AND workspace_code=?2", params![code, workspace_code])?;
                            } else {
                                c.execute("UPDATE directories SET archived=1 WHERE directory_code=?1 AND workspace_code=?2", params![code, workspace_code])?;
                            }
                        }
                    }
                }
                Ok(())
            }).await.map_err(|e| fw::StateError::Backend(e.to_string()))?;
            Ok(())
        }).await.map_err(|error| match error {
            fw::StateError::Conflict => ChatError::SessionLocationChanged,
            fw::StateError::Backend(ref message) if message == "session_directory_cleanup_failed" => ChatError::StorageFailure,
            other => chat_error(other),
        });
        if result.is_ok()
            && let Some(staged) = staged.take()
        {
            staged.commit().map_err(|_| ChatError::StorageFailure)?;
        }
        result
    }
}
