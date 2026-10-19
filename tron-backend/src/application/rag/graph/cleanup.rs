//! Durable cleanup: SQL claim -> graph deletion/flush -> SQL projection deletion.
//! A crash between stores leaves the same idempotent job available after its lease.
use super::{GraphGenerationManifest, GraphInfrastructure, RagGraphIndexRepository, RagGraphStore};
use crate::application::rag::RagService;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct GraphCleanupClaim {
    pub manifest: GraphGenerationManifest,
    pub lease_token: String,
    pub attempts: u32,
}
#[derive(Clone, Debug, Default)]
pub struct GraphHousekeepingResult {
    pub expired_snapshots: usize,
    pub removed_checkpoints: usize,
}
#[derive(Clone, Debug, Default)]
pub struct GraphCleanupReport {
    pub cleaned: usize,
    pub failed: usize,
    pub housekeeping: GraphHousekeepingResult,
}
#[derive(Default)]
pub(super) struct GraphMaintenanceLifecycle {
    started: AtomicBool,
    stop: CancellationToken,
}
impl Drop for GraphMaintenanceLifecycle {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

fn cleanup_batch(
    store: &dyn RagGraphStore,
    index: &dyn RagGraphIndexRepository,
    now: i64,
    limit: usize,
) -> Result<GraphCleanupReport, String> {
    if !(1..=16).contains(&limit) {
        return Err("invalid graph cleanup batch budget".into());
    }
    let started = std::time::Instant::now();
    let current = || now.saturating_add(started.elapsed().as_millis().min(i64::MAX as u128) as i64);
    let mut report = GraphCleanupReport {
        housekeeping: index.sweep_computation(now, 64)?,
        ..Default::default()
    };
    for _ in 0..limit {
        let Some(claim) = index.claim_cleanup(current(), 300_000)? else {
            break;
        };
        match store.delete_generation(&claim.manifest.partition, &claim.manifest.id) {
            Ok(()) => {
                index.finish_cleanup(&claim, current(), None)?;
                report.cleaned += 1;
            }
            Err(error) => {
                tracing::warn!(generation_id=%claim.manifest.id,attempt=claim.attempts,error=%error,"RAG graph fact cleanup failed");
                index.finish_cleanup(&claim, current(), Some("graph fact deletion failed"))?;
                report.failed += 1;
            }
        }
    }
    Ok(report)
}
impl GraphInfrastructure {
    pub fn cleanup_batch(&self, now: i64, limit: usize) -> Result<GraphCleanupReport, String> {
        cleanup_batch(self.store.as_ref(), self.index.as_ref(), now, limit)
    }
    pub(crate) fn start_maintenance(&self) -> Result<(), String> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| "graph maintenance runtime is unavailable")?;
        if self.maintenance.started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let owner = Arc::downgrade(&self.maintenance);
        let stop = self.maintenance.stop.clone();
        let (store, index) = (self.store.clone(), self.index.clone());
        runtime.spawn(async move{
            loop {
                if stop.is_cancelled() || owner.upgrade().is_none(){break;}
                let (store,index)=(store.clone(),index.clone());
                let task=tokio::task::spawn_blocking(move||cleanup_batch(store.as_ref(),index.as_ref(),chrono::Utc::now().timestamp_millis(),4));
                match task.await {
                    Ok(Ok(report))=>{if report.cleaned+report.failed+report.housekeeping.expired_snapshots+report.housekeeping.removed_checkpoints>0{
                        tracing::info!(cleaned=report.cleaned,failed=report.failed,expired_snapshots=report.housekeeping.expired_snapshots,removed_checkpoints=report.housekeeping.removed_checkpoints,"RAG graph maintenance completed");
                    }},
                    Ok(Err(error))=>tracing::error!(%error,"RAG graph maintenance failed"),
                    Err(error)=>tracing::error!(%error,"RAG graph maintenance worker failed"),
                }
                tokio::select!{_ = stop.cancelled()=>break,_ = tokio::time::sleep(std::time::Duration::from_secs(10))=>{}}
            }
        });
        Ok(())
    }
}
impl<R: Clone, E: Clone> RagService<R, E> {
    pub(crate) fn start_graph_maintenance(&self) -> Result<(), String> {
        if let Some(graph) = &self.graph {
            graph.start_maintenance()?;
        }
        Ok(())
    }
}
