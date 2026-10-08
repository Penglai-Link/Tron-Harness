use std::time::Duration;

use crate::{
    application::resource::{
        ResourceConnectionTarget, ResourceConnectionVerificationFuture, ResourceConnectionVerifier,
    },
    domain::resource::ResourceError,
};

use super::{kafka, mysql, postgresql, redis, ssh};

#[derive(Clone)]
pub struct ProtocolResourceConnectionVerifier {
    timeout: Duration,
    runtime_config: Option<crate::application::system_config::BackendRuntimeConfig>,
}

impl ProtocolResourceConnectionVerifier {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            runtime_config: None,
        }
    }

    pub fn with_runtime_config(
        mut self,
        runtime: crate::application::system_config::BackendRuntimeConfig,
    ) -> Self {
        self.runtime_config = Some(runtime);
        self
    }
}

impl ResourceConnectionVerifier for ProtocolResourceConnectionVerifier {
    fn verify(&self, target: ResourceConnectionTarget) -> ResourceConnectionVerificationFuture<'_> {
        Box::pin(async move {
            let timeout = self
                .runtime_config
                .as_ref()
                .map(|runtime| {
                    Duration::from_millis(
                        runtime
                            .snapshot()
                            .resource
                            .connection_verification_timeout_ms,
                    )
                })
                .unwrap_or(self.timeout);
            let kind = match &target {
                ResourceConnectionTarget::Mysql(_) => "MySQL",
                ResourceConnectionTarget::Postgresql(_) => "PostgreSQL",
                ResourceConnectionTarget::Redis(_) => "Redis",
                ResourceConnectionTarget::Kafka(_) => "Kafka",
                ResourceConnectionTarget::Ssh(_) => "SSH",
            };
            let probe = tokio::time::timeout(timeout, verify_target(target, timeout))
                .await
                .unwrap_or(Err(()));
            match probe {
                Ok(()) => Ok(()),
                Err(()) => Err(ResourceError::ConnectionVerificationFailed(format!(
                    "{kind} connection verification failed"
                ))),
            }
        })
    }
}

async fn verify_target(target: ResourceConnectionTarget, timeout: Duration) -> Result<(), ()> {
    match target {
        ResourceConnectionTarget::Mysql(target) => mysql::verify(target).await,
        ResourceConnectionTarget::Postgresql(target) => postgresql::verify(target).await,
        ResourceConnectionTarget::Redis(target) => redis::verify(target).await,
        ResourceConnectionTarget::Kafka(target) => kafka::verify(target, timeout).await,
        ResourceConnectionTarget::Ssh(target) => ssh::verify(target).await,
    }
}
