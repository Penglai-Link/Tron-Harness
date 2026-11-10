use super::FrameworkHost;
use crate::domain::agent::AgentError;
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::Framework;
use eliteforge_agent_official_plugin_storage_local::{LocalStorageConfig, LocalStorageRuntime};
use std::ops::Deref;
use std::time::Duration;

/// Keeps a replaceable assembly alive through an entire request or auxiliary run.
pub(crate) struct RuntimeFrameworkLease {
    framework: Framework,
    _lease: tokio::sync::OwnedRwLockReadGuard<()>,
}
impl Deref for RuntimeFrameworkLease {
    type Target = Framework;
    fn deref(&self) -> &Self::Target {
        &self.framework
    }
}

impl FrameworkHost {
    pub(super) async fn storage(&self) -> Result<&LocalStorageRuntime, fw::AgentError> {
        self.ensure_open()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        self.0
            .storage
            .get_or_try_init(|| async {
                self.prepare_artifacts()
                    .map_err(fw::AgentError::InvalidConfiguration)?;
                let mut config =
                    LocalStorageConfig::new(&self.0.path, self.0.path.with_extension("artifacts"));
                config.database = Some(self.shared_database().await?);
                config.summaries.max_record_bytes = 16 * 1024 * 1024;
                config.conversations.max_sessions = usize::MAX;
                LocalStorageRuntime::open(config)
                    .await
                    .map_err(|e| fw::AgentError::InvalidConfiguration(e.to_string()))
            })
            .await
    }

    pub(crate) async fn runtime_framework(&self) -> Result<RuntimeFrameworkLease, fw::AgentError> {
        let _admission = self.0.config_lock.lock().await;
        let lease = self.0.runtime_access.clone().read_owned().await;
        self.ensure_open()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        Ok(RuntimeFrameworkLease {
            framework: self.framework().await?,
            _lease: lease,
        })
    }

    /// A failed or interrupted run can leave a nonterminal checkpoint behind.
    /// Finish it under a new lease before admitting another turn in the session.
    pub(super) async fn recover_interrupted_checkpoint(
        &self,
        session_code: &str,
    ) -> Result<(), AgentError> {
        let framework = self.storage_framework().await.map_err(super::map_error)?;
        let sessions = framework.sessions().map_err(super::map_error)?;
        let state = framework
            .services()
            .map_err(super::map_error)?
            .get::<fw::StateStoreService>()
            .ok_or_else(|| {
                AgentError::InvalidConfiguration("application state storage is missing".into())
            })?
            .0
            .clone();
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
        // Shutdown must observe an initializer that was admitted before close,
        // including the borrow registered by the stable storage facade.
        let _initialization = self.0.storage_access.read().await;
        self.ensure_open()
            .map_err(fw::AgentError::InvalidConfiguration)?;
        self.0
            .storage_framework
            .get_or_try_init(|| async {
                Framework::builder()
                    .plugin(self.storage().await?.plugin())
                    .build()
                    .await
            })
            .await
            .cloned()
    }
}
