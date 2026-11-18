//! Reconcile durable capability pins with the runs that own them.
//!
//! A run pins the exact package version it started with and releases the pin
//! when a terminal checkpoint commits. Interrupted runs never commit, so their
//! pins survive the process and would otherwise block pack removal forever.
use super::FrameworkHost;
use eliteforge_agent_capability_runtime::{CapabilityPackManager, DurablePackPin};
use eliteforge_agent_core as fw;
use std::collections::BTreeSet;

impl FrameworkHost {
    /// Drop pins whose run can no longer resume. `package` limits the sweep to
    /// one pack; a staler run elsewhere keeps its own pins.
    pub(crate) async fn prune_durable_pins(&self, package: Option<&str>) -> Result<usize, String> {
        let manager =
            CapabilityPackManager::new(&self.0.pack_root).map_err(|error| error.to_string())?;
        let pins = manager
            .durable_pins()
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|pin| {
                package.is_none_or(|id| pin.versions.iter().any(|version| version.package == id))
            })
            .collect::<Vec<_>>();
        if pins.is_empty() {
            return Ok(0);
        }
        let runs = pins.iter().filter_map(pin_run).collect::<Vec<_>>();
        let resumable = self.resumable_runs(&runs).await?;
        let mut stale = Vec::new();
        for pin in &pins {
            let alive = pin_run(pin).is_some_and(|run| resumable.contains(&run));
            if !alive {
                stale.push(pin.id.clone());
            }
        }
        for id in &stale {
            manager.unpin(id).map_err(|error| error.to_string())?;
        }
        Ok(stale.len())
    }

    /// Drop every pin that still references `package`. Only an explicit forced
    /// uninstall may discard a resumable run's package version.
    pub(crate) fn release_pack_pins(&self, package: &str) -> Result<usize, String> {
        let manager =
            CapabilityPackManager::new(&self.0.pack_root).map_err(|error| error.to_string())?;
        let mut released = 0;
        for pin in manager.durable_pins().map_err(|error| error.to_string())? {
            if pin
                .versions
                .iter()
                .any(|version| version.package == package)
            {
                manager.unpin(&pin.id).map_err(|error| error.to_string())?;
                released += 1;
            }
        }
        Ok(released)
    }

    /// A run stays resumable while it waits for user input or executes in this
    /// process. A persisted `RUNNING` state without a live executor is what an
    /// interrupted process leaves behind, so its pin must not outlive it.
    async fn resumable_runs(&self, runs: &[fw::RunId]) -> Result<BTreeSet<fw::RunId>, String> {
        let runs = runs.iter().copied().collect::<BTreeSet<_>>();
        if runs.is_empty() {
            return Ok(BTreeSet::new());
        }
        let framework = self
            .storage_framework()
            .await
            .map_err(|error| error.to_string())?;
        let sessions = framework.sessions().map_err(|error| error.to_string())?;
        let executing = self
            .executing_run_ids()
            .into_iter()
            .collect::<BTreeSet<_>>();
        let ordered = runs.into_iter().collect::<Vec<_>>();
        let found = sessions
            .store
            .find_runs(&ordered)
            .await
            .map_err(|error| error.to_string())?;
        Ok(ordered
            .into_iter()
            .zip(found)
            .filter_map(|(run, state)| {
                let state = state?;
                let live = match state.status {
                    fw::ManagedRunStatus::AwaitingInput => true,
                    fw::ManagedRunStatus::Running => executing.contains(&run),
                    _ => false,
                };
                live.then_some(run)
            })
            .collect())
    }
}

/// Pin IDs are `{namespace hash}.{run id}.{package id}`.
fn pin_run(pin: &DurablePackPin) -> Option<fw::RunId> {
    pin.id
        .split('.')
        .find_map(|part| fw::RunId::try_parse(part).ok())
}
