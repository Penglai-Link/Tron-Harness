//! A profile-owned native continuation driver and its fenced SDK journal.
use super::FrameworkHost;
use eliteforge_agent_framework::Agent;
impl FrameworkHost {
    pub(crate) async fn continuation_agent(&self) -> Result<Agent, String> {
        let _admission = self.0.config_lock.lock().await;
        self.ensure_open()?;
        let agent = self
            .0
            .continuation_agent
            .get_or_try_init(|| async { self.build_driver(false).await.map_err(|e| e.to_string()) })
            .await?;
        Ok(agent.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fresh_profile_builds_continuation_storage_with_private_artifacts_and_closes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join("fresh-profile")
            .join("framework.sqlite3");
        let artifacts = path.with_extension("artifacts");
        assert!(!artifacts.exists());
        assert!(!path.parent().unwrap().exists());
        let host = FrameworkHost::persistent(path.clone(), None);
        let agent = host
            .continuation_agent()
            .await
            .expect("actual SDK plugins install on a fresh profile");
        assert!(artifacts.is_dir());
        assert!(path.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&artifacts).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        assert!(
            agent
                .latest_checkpoint("fresh-driver")
                .await
                .unwrap()
                .is_none()
        );
        assert!(host.close().await.is_success());
        assert!(host.continuation_agent().await.is_err());
    }
}
