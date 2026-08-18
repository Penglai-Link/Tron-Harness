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
}

impl ProtocolResourceConnectionVerifier {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl ResourceConnectionVerifier for ProtocolResourceConnectionVerifier {
    fn verify(&self, target: ResourceConnectionTarget) -> ResourceConnectionVerificationFuture<'_> {
        Box::pin(async move {
            let kind = match &target {
                ResourceConnectionTarget::Mysql(_) => "MySQL",
                ResourceConnectionTarget::Postgresql(_) => "PostgreSQL",
                ResourceConnectionTarget::Redis(_) => "Redis",
                ResourceConnectionTarget::Kafka(_) => "Kafka",
                ResourceConnectionTarget::Ssh(_) => "SSH",
            };
            let probe = tokio::time::timeout(self.timeout, verify_target(target, self.timeout))
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
