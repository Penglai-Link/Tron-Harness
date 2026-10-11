use super::*;
use std::collections::BTreeSet;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

impl DashboardRepository for TelemetryProjection {
    fn dashboard(&self, workspace: &str) -> Result<ObservabilityDashboard, ObservabilityError> {
        let q = TraceQuery { sorts: Vec::new(), sort_by: Default::default(), sort_direction: Default::default(),
            workspace_code: workspace.into(),
            session_code: None,
            trace_id: None,
            current: 1,
            size: 1,
        };
        let metrics = self.metrics(&q)?;
        let runtime = self.runtime_metrics()?;
        let sampled_at = now();
        let (running_trace_count, recent_requests) = blocking(async {
            let mut buckets = request_buckets(sampled_at);
            let mut running = 0;
            let mut offset = 0;
            loop {
                let page = self
                    .store
                    .query_traces(filter(&q), offset, 5000)
                    .await
                    .map_err(failure)?;
                let count = page.records.len();
                for trace in page.records {
                    running += usize::from(trace_status(&trace) == "Running");
                    record_request(
                        &mut buckets,
                        trace.started_at,
                        trace_status(&trace) == "Error",
                        sampled_at,
                    );
                }
                offset += count;
                if offset >= page.total || count == 0 {
                    break;
                }
            }
            Ok::<_, ObservabilityError>((running, buckets))
        })?;
        Ok(ObservabilityDashboard {
            sampled_at,
            trace_count: metrics.trace_count,
            failed_trace_count: metrics.failed_trace_count,
            running_trace_count,
            input_tokens: metrics.input_tokens,
            output_tokens: metrics.output_tokens,
            average_duration_ms: metrics.average_duration_ms,
            p95_duration_ms: metrics.p95_duration_ms,
            recent_requests,
            cpu_percent: runtime.cpu_percent,
            memory_bytes: runtime.memory_bytes,
            resource_sampled_at: runtime.sampled_at,
            metric_point_count: if workspace.is_empty() { runtime.metric_point_count } else { self.scoped_metric_points(Some(workspace.into()), None, 1, 1)?.total },
            processes: service_processes(),
        })
    }
}
fn request_buckets(at: i64) -> Vec<RequestBucket> {
    let last_minute = at / 60_000 * 60_000;
    (0..15)
        .map(|i| RequestBucket {
            started_at: last_minute - (14 - i) * 60_000,
            requests: 0,
            errors: 0,
        })
        .collect()
}
fn record_request(buckets: &mut [RequestBucket], started: i64, error: bool, at: i64) {
    if started > at {
        return;
    }
    if let Some(bucket) = buckets
        .iter_mut()
        .find(|b| started >= b.started_at && started < b.started_at + 60_000)
    {
        bucket.requests += 1;
        bucket.errors += usize::from(error);
    }
}
fn service_processes() -> Vec<ServiceProcess> {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_memory(),
    );
    let root = sysinfo::Pid::from_u32(std::process::id());
    let mut owned = BTreeSet::from([root]);
    loop {
        let next = system
            .processes()
            .iter()
            .filter(|(_, p)| p.parent().is_some_and(|id| owned.contains(&id)))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        let before = owned.len();
        owned.extend(next);
        if before == owned.len() {
            break;
        }
    }
    owned
        .into_iter()
        .filter_map(|pid| {
            system.process(pid).map(|p| ServiceProcess {
                pid: pid.as_u32(),
                parent_pid: if pid == root {
                    None
                } else {
                    p.parent().map(|id| id.as_u32())
                },
                name: if pid == root {
                    "tron-backend".into()
                } else {
                    p.name().to_string_lossy().into_owned()
                },
                memory_bytes: p.memory(),
                started_at: (p.start_time().min(i64::MAX as u64 / 1000) * 1000) as i64,
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minute_buckets_exclude_old_and_future_requests_and_count_errors() {
        let at = 30 * 60_000 + 20_000;
        let mut buckets = request_buckets(at);
        for (time, error) in [
            (16 * 60_000 - 1, false),
            (16 * 60_000, false),
            (30 * 60_000, true),
            (at + 1, true),
        ] {
            record_request(&mut buckets, time, error, at);
        }
        assert_eq!(buckets.len(), 15);
        assert_eq!(buckets.iter().map(|b| b.requests).sum::<usize>(), 2);
        assert_eq!(buckets[14].errors, 1);
    }
    #[cfg(unix)]
    #[test]
    fn live_child_process_appears_under_backend() {
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        let snapshot = service_processes();
        let _ = child.kill();
        let _ = child.wait();
        assert!(
            snapshot
                .iter()
                .any(|p| p.pid == pid && p.parent_pid == Some(std::process::id()))
        );
    }
    #[test]
    fn process_snapshot_includes_only_backend_and_its_descendants() {
        let processes = service_processes();
        let root = processes
            .iter()
            .find(|p| p.pid == std::process::id())
            .unwrap();
        assert_eq!(root.parent_pid, None);
        assert_eq!(root.name, "tron-backend");
        for process in &processes {
            if let Some(parent) = process.parent_pid {
                assert!(processes.iter().any(|p| p.pid == parent));
            }
        }
    }
}
