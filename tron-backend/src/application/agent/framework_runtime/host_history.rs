//! Product-derived state joins the Framework history transaction.
use super::FrameworkHost;
use eliteforge_agent_core as fw;

impl FrameworkHost {
    pub async fn delete_history_turn(
        &self,
        session: &str,
        root: fw::RunId,
        expected: &str,
    ) -> Result<String, fw::StateError> {
        let _admission = self.0.config_lock.lock().await;
        let framework = self
            .storage_framework()
            .await
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let history = framework
            .history()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        let sessions = framework
            .sessions()
            .map_err(|e| fw::StateError::Backend(e.to_string()))?;
        self.shared_database()
            .await?
            .transaction(async {
                let before = history.context_revision(session).await?;
                let revision = history.delete_turn(session, root, expected).await?;
                if before != revision {
                    let deleted = history
                        .deleted_runs(session)
                        .await?
                        .into_iter()
                        .map(|id| id.to_string())
                        .collect::<std::collections::HashSet<_>>();
                    if let Some((version, mut todos)) =
                        sessions.read_context(session, "tron.todos").await?
                    {
                        if let Some(items) = todos.as_array_mut() {
                            items.retain(|item| {
                                !["createdByRunCode", "updatedByRunCode"].iter().any(|key| {
                                    item[*key].as_str().is_some_and(|id| deleted.contains(id))
                                })
                            });
                            sessions
                                .write_context(session, "tron.todos", Some(version), todos)
                                .await?;
                        }
                    }
                    sessions
                        .store
                        .append_event(fw::SessionEvent::new(
                            session,
                            "tron.history-edited",
                            serde_json::json!({"rootRunId": root, "contextRevision": revision}),
                        ))
                        .await?;
                    let key = "tron.session-summary-view";
                    let previous = sessions.read_context(session, key).await?;
                    sessions
                        .write_context(
                            session,
                            key,
                            previous.map(|(version, _)| version),
                            serde_json::Value::Null,
                        )
                        .await?;
                }
                Ok(revision)
            })
            .await
    }
}
