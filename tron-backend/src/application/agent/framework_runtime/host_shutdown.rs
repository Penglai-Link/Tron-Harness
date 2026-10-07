//! A shared shutdown task retains every callback owner, including those exceeding its deadline.
use super::FrameworkHost;
use eliteforge_agent_plugin_core::ShutdownReport;
use std::{future::Future, sync::Mutex};
use tokio::sync::watch;
const CLOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
type Owner = tokio::task::JoinHandle<ShutdownReport>;
#[derive(Default)]
pub(super) struct OwnedShutdown {
    completion: Mutex<Option<watch::Receiver<Option<ShutdownReport>>>>,
    final_report: Mutex<Option<watch::Receiver<Option<ShutdownReport>>>>,
}
impl FrameworkHost {
    pub async fn close(&self) -> ShutdownReport {
        let mut completion = {
            let mut state = self
                .0
                .shutdown
                .completion
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(receiver) = &*state {
                receiver.clone()
            } else {
                self.0
                    .closed
                    .store(true, std::sync::atomic::Ordering::Release);
                let (sender, receiver) = watch::channel(None);
                let (finished, final_report) = watch::channel(None);
                *self
                    .0
                    .shutdown
                    .final_report
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(final_report);
                *state = Some(receiver.clone());
                let host = self.clone();
                tokio::spawn(async move {
                    let result =
                        tokio::spawn(async move { host.shutdown_owned(finished).await }).await;
                    sender.send_replace(Some(
                        result.unwrap_or_else(|error| join_error("host shutdown", error)),
                    ));
                });
                receiver
            }
        };
        loop {
            if let Some(report) = completion.borrow_and_update().clone() {
                return report;
            }
            if completion.changed().await.is_err() {
                return unfinished("host shutdown owner terminated");
            }
        }
    }
    /// Observe final late reports. Cancelling a waiter never abandons the shutdown owners.
    pub async fn wait_shutdown_completion(&self) -> ShutdownReport {
        let initial = self.close().await;
        let receiver = self
            .0
            .shutdown
            .final_report
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .cloned();
        if let Some(mut receiver) = receiver {
            loop {
                if let Some(report) = receiver.borrow_and_update().clone() {
                    return report;
                }
                if receiver.changed().await.is_err() {
                    return initial;
                }
            }
        }
        initial
    }
    async fn shutdown_owned(
        &self,
        finished: watch::Sender<Option<ShutdownReport>>,
    ) -> ShutdownReport {
        let mut report = ShutdownReport::default();
        let mut pending = Vec::new();
        let admission = tokio::time::timeout(CLOSE_TIMEOUT, self.0.config_lock.lock()).await;
        let admission_stalled = admission.is_err();
        if admission_stalled {
            report.timeouts += 1;
            report
                .errors
                .push("host admission shutdown timed out".into());
        }
        let client = self
            .0
            .model_client
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        drop(admission);
        let initialized = [
            self.0.agent.get().is_some(),
            self.0.external_agent.get().is_some(),
            self.0.continuation_agent.get().is_some(),
        ];
        for agent in [
            self.0.agent.get(),
            self.0.external_agent.get(),
            self.0.continuation_agent.get(),
        ]
        .into_iter()
        .flatten()
        {
            let agent = agent.clone();
            poll_owner(
                "SDK agent",
                tokio::spawn(async move { agent.close().await }),
                &mut report,
                &mut pending,
            )
            .await;
        }
        let drained = tokio::time::timeout(CLOSE_TIMEOUT, self.drain_native())
            .await
            .is_ok();
        if !drained {
            report.timeouts += 1;
        }
        // These independent cancellation resources are still closed if native callbacks stall.
        if let Some(client) = client {
            poll_owner(
                "model client",
                spawn_close(async move { client.close_observed().await }),
                &mut report,
                &mut pending,
            )
            .await;
        }
        let pty = self.pty();
        poll_owner(
            "PTY",
            spawn_close(async move {
                tokio::task::spawn_blocking(move || pty.close())
                    .await
                    .map_err(|e| format!("PTY close: {e}"))?;
                Ok(ShutdownReport::default())
            }),
            &mut report,
            &mut pending,
        )
        .await;
        if pending.is_empty() && drained && !admission_stalled {
            let host = self.clone();
            poll_owner(
                "dependent stores",
                tokio::spawn(async move { host.close_dependent_stores().await }),
                &mut report,
                &mut pending,
            )
            .await;
            if pending.is_empty() {
                finished.send_replace(Some(report.clone()));
                return report;
            }
            // Only this store owner remains. It already respects the SDK release dependency.
        }
        report.errors.push(
            "shutdown unfinished; retained owners and dependent stores are still draining".into(),
        );
        let host = self.clone();
        let mut final_report = report.clone();
        tokio::spawn(async move {
            let stores_started = pending.iter().any(|(name, _)| *name == "dependent stores");
            for (name, owner) in pending {
                merge_join(name, owner.await, &mut final_report);
            }
            if !stores_started {
                let admission = host.0.config_lock.lock().await;
                drop(admission);
                for (was_initialized, agent) in initialized.into_iter().zip([
                    host.0.agent.get(),
                    host.0.external_agent.get(),
                    host.0.continuation_agent.get(),
                ]) {
                    if !was_initialized {
                        if let Some(agent) = agent {
                            let agent = agent.clone();
                            merge_join(
                                "late initialized SDK",
                                tokio::spawn(async move { agent.close().await }).await,
                                &mut final_report,
                            );
                        }
                    }
                }
                host.drain_native().await;
                let closing = host.clone();
                merge_join(
                    "dependent stores",
                    tokio::spawn(async move { closing.close_dependent_stores().await }).await,
                    &mut final_report,
                );
            }
            final_report.errors.retain(|error|error!="shutdown unfinished; retained owners and dependent stores are still draining");
            finished.send_replace(Some(final_report));
        });
        report
    }
    async fn close_dependent_stores(&self) -> ShutdownReport {
        // Branch leases must finish before the backing SQLite connections close.
        let mut report = self.0.native_bindings.close().await;
        let mut owners = Vec::new();
        if let Some(store) = self.0.native_sessions.get().cloned() {
            owners.push((
                "native sessions",
                spawn_close(async move {
                    store.close().await.map_err(|e| e.to_string())?;
                    Ok(ShutdownReport::default())
                }),
            ));
        }
        for driver in [self.0.external_agent.get(), self.0.continuation_agent.get()]
            .into_iter()
            .flatten()
        {
            let framework = driver.framework();
            owners.push((
                "SDK driver Framework",
                tokio::spawn(async move { framework.close().await }),
            ));
        }
        // All store callbacks start even if another one stalls or panics.
        for (name, owner) in owners {
            merge_join(name, owner.await, &mut report);
        }
        if let Some(framework) = self.0.framework.get() {
            report.merge(framework.close().await);
        }
        report
    }
}
fn spawn_close(
    future: impl Future<Output = Result<ShutdownReport, String>> + Send + 'static,
) -> Owner {
    tokio::spawn(async move {
        future.await.unwrap_or_else(|error| ShutdownReport {
            errors: vec![error],
            panics: 0,
            timeouts: 0,
        })
    })
}
async fn poll_owner(
    name: &'static str,
    mut owner: Owner,
    report: &mut ShutdownReport,
    pending: &mut Vec<(&'static str, Owner)>,
) {
    match tokio::time::timeout(CLOSE_TIMEOUT, &mut owner).await {
        Ok(result) => merge_join(name, result, report),
        Err(_) => {
            report.timeouts += 1;
            pending.push((name, owner));
        }
    }
}
fn merge_join(
    name: &str,
    result: Result<ShutdownReport, tokio::task::JoinError>,
    report: &mut ShutdownReport,
) {
    report.merge(result.unwrap_or_else(|error| join_error(name, error)));
}
fn join_error(name: &str, error: tokio::task::JoinError) -> ShutdownReport {
    ShutdownReport {
        errors: vec![format!(
            "{name} owner failed: {error}; resource closure unconfirmed"
        )],
        panics: usize::from(error.is_panic()),
        timeouts: 0,
    }
}
fn unfinished(message: &str) -> ShutdownReport {
    ShutdownReport {
        errors: vec![format!("{message}; shutdown unfinished")],
        panics: 0,
        timeouts: 0,
    }
}
