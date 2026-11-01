use super::FrameworkHost;
use crate::domain::agent::AgentError;
use eliteforge_agent_core::{self as fw, StateStore};
use eliteforge_agent_official_plugin_storage_local::{SqliteOptions, SqliteStateStore};
use std::time::Duration;
impl FrameworkHost {
    /// A failed or interrupted run can leave a nonterminal checkpoint behind.
    /// Finish it under a new lease before admitting another turn in the session.
    pub(super) async fn recover_interrupted_checkpoint(
        &self,
        session_code: &str,
    ) -> Result<(), AgentError> {
        let framework = self.framework().await.map_err(super::map_error)?;
        let sessions = framework.sessions().map_err(super::map_error)?;
        let database = self
            .shared_database()
            .await
            .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
        let state = SqliteStateStore::with_database(database, SqliteOptions::default())
            .await
            .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
        let Some(head) = state
            .latest_scope(session_code, "main")
            .await
            .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?
        else {
            return Ok(());
        };
        if head.status.is_terminal() {
            return Ok(());
        }
        let run = sessions
            .store
            .get_run(head.run_id)
            .await
            .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
        let terminal = match &run.status {
            fw::ManagedRunStatus::Failed => fw::ExecutionStatus::Failed,
            fw::ManagedRunStatus::Cancelled | fw::ManagedRunStatus::RecoveryRequired => {
                fw::ExecutionStatus::Cancelled
            }
            _ => return Ok(()),
        };
        let original_status = run.status;
        let lease = state
            .claim(head.run_id, fw::RunId::new_v4(), Duration::from_secs(30))
            .await
            .map_err(|error| match error {
                fw::StateError::Busy => AgentError::SessionBusy(session_code.into()),
                error => AgentError::InvalidConfiguration(error.to_string()),
            })?;
        let result = async {
            // Reload after taking the fencing lease so a previous owner cannot
            // change the checkpoint between our inspection and commit.
            let mut current = state.load(head.run_id).await?;
            let run = sessions.store.get_run(head.run_id).await?;
            if run.status != original_status {
                return Err(fw::StateError::Busy);
            }
            let revision = current.revision;
            current.status = terminal;
            state.commit(&lease, revision, current).await?;
            Ok(())
        }
        .await;
        let release = state.release(&lease).await;
        result.map_err(|error| match error {
            fw::StateError::Busy => AgentError::SessionBusy(session_code.into()),
            error => AgentError::InvalidConfiguration(error.to_string()),
        })?;
        release.map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?;
        tracing::info!(session_code, run_id = %head.run_id, "Recovered interrupted checkpoint");
        Ok(())
    }

    /// The host owns one connection; Tron link writes join Framework transactions.
    pub async fn shared_database(
        &self,
    ) -> Result<
        eliteforge_agent_official_plugin_storage_local::SqliteDatabase,
        eliteforge_agent_core::StateError,
    > {
        self.0.database.get_or_try_init(|| async {
            if let Some(parent) = self.0.path.parent() { std::fs::create_dir_all(parent).map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))?; }
            crate::adapters::outbound::persistence::sqlite::framework_storage::SharedDatabase::open(&self.0.path.to_string_lossy()).map_err(|e| eliteforge_agent_core::StateError::Backend(e.to_string()))
        }).await.map(|database| database.runtime())
    }

    pub async fn storage_framework(
        &self,
    ) -> Result<eliteforge_agent_framework::Framework, eliteforge_agent_core::AgentError> {
        self.framework().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn persistent_history_does_not_block_a_new_conversation_after_1024() {
        let host = FrameworkHost::temporary(None).unwrap();
        let framework = host.framework().await.unwrap();
        let database = host.shared_database().await.unwrap();
        database.call(|connection| {
            connection.execute_batch("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<1024)
                INSERT INTO ef_conversations(session,branch,generation,revision,messages,accessed)
                SELECT 'saved-' || i,'main','fixture-generation',0,'[]',0 FROM n")
        }).await.unwrap();
        let services = framework.services().unwrap();
        let store = services.get::<fw::ConversationStoreService>().unwrap();
        let mut conversation = store.0.open("new-child-after-saved-history").await.unwrap();
        conversation.finish(fw::ConversationOutcome::Completed).await.unwrap();
        drop(conversation);
        let count: u64 = database.call(|connection| {
            connection.query_row("SELECT count(*) FROM ef_conversations", [], |row| row.get(0))
        }).await.unwrap();
        assert_eq!(count, 1025, "saved history must remain intact");
        assert!(host.close().await.is_success());
    }

    #[tokio::test]
    async fn recovered_run_no_longer_blocks_the_next_chat_turn() {
        let host = FrameworkHost::temporary(None).unwrap();
        let framework = host.framework().await.unwrap();
        let sessions = framework.sessions().unwrap();
        let session = fw::SessionRecord::new("Dude chat");
        let session = sessions.store.create_session(session).await.unwrap();
        let database = host.shared_database().await.unwrap();
        let state = SqliteStateStore::with_database(database.clone(), SqliteOptions::default())
            .await
            .unwrap();
        let run_id = fw::RunId::new_v4();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        sessions
            .store
            .register_run(fw::ManagedRun {
                id: run_id,
                session_id: session.id.clone(),
                execution_scope: "main".into(),
                conversation_branch: "main".into(),
                parent_run_id: None,
                root_run_id: run_id,
                goal_id: None,
                kind: "CHAT".into(),
                status: fw::ManagedRunStatus::Running,
                result: None,
                error: None,
                started_at_ms: now - 60_000,
                finished_at_ms: None,
                lease_until_ms: now + 3_600_000,
            }, false)
            .await
            .unwrap();
        state
            .create(fw::Checkpoint::new(run_id, &session.id, "test", 1, serde_json::json!({})), None)
            .await
            .unwrap();
        state.claim(run_id, fw::RunId::new_v4(), Duration::from_secs(60)).await.unwrap();
        database
            .call(move |connection| {
                connection.execute(
                    "UPDATE ef_checkpoints SET expires=?2 WHERE run_id=?1",
                    (run_id.to_string(), now - 1_000),
                )
            })
            .await
            .unwrap();
        sessions.store.recover_expired_runs().await.unwrap();

        host.recover_interrupted_checkpoint(&session.id).await.unwrap();
        assert_eq!(
            state.latest_scope(&session.id, "main").await.unwrap().unwrap().status,
            fw::ExecutionStatus::Cancelled,
        );
        state
            .create(fw::Checkpoint::new(fw::RunId::new_v4(), &session.id, "test", 1, serde_json::json!({})), Some(run_id))
            .await
            .unwrap();
    }
}
