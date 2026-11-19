use crate::app::AppState;
use std::time::Duration;

/// Poll the durable queue so operations entered while every Agent was busy run
/// as soon as the reconfiguration gate opens again. HTTP requests still attempt
/// one immediate pass; this loop is the recovery path after a run or restart.
pub(super) fn start(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            match state.agent_framework.has_pending_capability_operations() {
                Ok(false) => continue,
                Ok(true) => {}
                Err(error) => {
                    tracing::warn!(%error, "capability queue inspection failed");
                    continue;
                }
            }
            if let Err(error) = state
                .agent_framework
                .process_capability_queue(usize::MAX)
                .await
            {
                tracing::warn!(%error, "capability queue processing failed");
            }
        }
    });
}
