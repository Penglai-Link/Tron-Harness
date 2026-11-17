//! Durable Thread completion delivery independent of the parent Run.
use super::*;

/// Persisted routing envelope delivered independently of the originating Run/UI.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadFeedback {
    pub event_key: String,
    pub owner: crate::domain::agent::AgentCollaborationOwner,
    pub session_id: String,
    pub origin_session_code: String,
    pub initiator_agent_code: Option<String>,
    pub status: fw::ThreadStatus,
    pub result: Option<fw::ThreadResult>,
    pub error: Option<String>,
    /// Present when the feedback reports a detached child run completion
    /// instead of an independent thread; PersonAI prompts name it accordingly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_run_id: Option<fw::RunId>,
}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantThreadCatalogItem {
    pub summary: fw::ThreadSummary,
    pub agent_code: String,
    pub closed: bool,
    pub origin_session_code: String,
    pub task_code: Option<String>,
}

#[async_trait::async_trait]
pub trait ThreadFeedbackSink: Send + Sync {
    /// Implementations durably enqueue idempotently by event_key before returning.
    async fn publish(&self, feedback: ThreadFeedback) -> Result<(), String>;
}

impl FrameworkHost {
    /// The product service must authenticate access to the platform PersonAI before calling.
    pub async fn assistant_threads(
        &self,
        assistant_code: &str,
    ) -> Result<Vec<fw::ThreadSummary>, fw::AgentError> {
        Ok(self
            .thread_records()
            .await?
            .into_iter()
            .filter(|record| {
                record.owner.kind == "ASSISTANT" && record.owner.code == assistant_code
            })
            .map(|record| record.summary)
            .collect())
    }
    pub async fn assistant_thread_catalog(
        &self,
        assistant_code: &str,
    ) -> Result<Vec<AssistantThreadCatalogItem>, fw::AgentError> {
        Ok(self
            .thread_records()
            .await?
            .into_iter()
            .filter(|record| {
                record.owner.kind == "ASSISTANT" && record.owner.code == assistant_code
            })
            .map(|record| AssistantThreadCatalogItem {
                summary: record.summary,
                agent_code: record.agent_code,
                closed: record.closed,
                origin_session_code: record.origin_session_code,
                task_code: record.owner.task_code,
            })
            .collect())
    }
    /// Closed Threads keep their native history; ordinary user chat APIs cannot resolve them.
    pub async fn assistant_thread_history(
        &self,
        assistant_code: &str,
        id: &str,
        query: fw::HistoryQuery,
    ) -> Result<fw::SessionHistory, fw::AgentError> {
        self.thread_record(id)
            .await?
            .filter(|record| {
                record.owner.kind == "ASSISTANT" && record.owner.code == assistant_code
            })
            .ok_or_else(|| failure("thread not found"))?;
        self.storage_framework()
            .await?
            .history()?
            .read_session(id, query)
            .await
            .map_err(Into::into)
    }
    pub fn set_thread_feedback_sink(
        &self,
        sink: Arc<dyn ThreadFeedbackSink>,
    ) -> Result<(), String> {
        self.0
            .thread_feedback
            .set(sink)
            .map_err(|_| "Thread feedback sink already configured".into())
    }
    /// Called at startup and by the host background dispatcher; no original Run is needed.
    pub async fn recover_collaboration_threads(&self) -> Result<usize, fw::AgentError> {
        let framework = self.storage_framework().await?;
        let store = framework.sessions()?.store;
        store.recover_expired_runs().await?;
        let mut recovered = 0;
        let candidates = self.running_thread_ids().await?;
        for batch in candidates.chunks(64) {
            let cached = {
                let threads = self.0.collaboration_threads.read().unwrap();
                batch
                    .iter()
                    .map(|id| (id.clone(), threads.get(id).cloned()))
                    .collect::<BTreeMap<_, _>>()
            };
            // Never wait while holding another thread's transition lock. Busy
            // admissions/completions are retried by the next recovery tick.
            let guards = cached
                .iter()
                .map(|(id, thread)| {
                    (
                        id,
                        thread.as_ref().map(|thread| thread.transition.try_lock()),
                    )
                })
                .collect::<Vec<_>>();
            let selected = guards
                .iter()
                .filter(|(_, guard)| !matches!(guard, Some(Err(_))))
                .map(|(id, _)| (*id).clone())
                .collect::<Vec<_>>();
            // Refresh after acquiring locks: resume can reuse a RunId, so the
            // candidate list alone cannot fence an earlier execution.
            let records = self
                .thread_records_by_ids(&selected)
                .await?
                .into_iter()
                .filter(|record| {
                    record.summary.status == fw::ThreadStatus::Running
                        && record.summary.latest_run_id.is_some()
                })
                .collect::<Vec<_>>();
            let ids = records
                .iter()
                .filter_map(|record| record.summary.latest_run_id)
                .collect::<Vec<_>>();
            // Judge lease/grace windows against this batch's observation time,
            // not the time spent persisting preceding completions.
            let observed_at_ms = fw::lifecycle_time_ms();
            let native = store.find_runs(&ids).await?;
            if native.len() != records.len() {
                return Err(failure("run batch response length differs"));
            }
            for (mut record, native) in records.into_iter().zip(native) {
                let run_id = record.summary.latest_run_id.unwrap();
                let current = self
                    .0
                    .collaboration_threads
                    .read()
                    .unwrap()
                    .get(&record.summary.session_id)
                    .cloned();
                let previous = cached
                    .get(&record.summary.session_id)
                    .and_then(Option::as_ref);
                if match (previous, current.as_ref()) {
                    (Some(previous), Some(current)) => !Arc::ptr_eq(previous, current),
                    (None, None) => false,
                    _ => true,
                } {
                    continue;
                }
                let completion = match native {
                    // Continuations reuse a RunId. A previous waiting result
                    // must not complete a newly admitted execution, even when
                    // both admissions share the same millisecond timestamp.
                    Some(run)
                        if record.resumed_interaction_id.is_some_and(|id| {
                            run.result.as_ref().is_some_and(|result| {
                                result
                                    .interactions
                                    .iter()
                                    .any(|interaction| interaction.id == id)
                            })
                        }) =>
                    {
                        if observed_at_ms.saturating_sub(record.run_admitted_at_ms) < 30_000 {
                            None
                        } else {
                            Some((fw::ThreadStatus::Failed, Err(
                                "Thread continuation was admitted but did not start; previous execution remains intact".into(),
                            )))
                        }
                    }
                    Some(run) if run.result.is_some() => {
                        let result = fw::ThreadResult::from_run(run_id, &run.result.unwrap());
                        Some((fw::ThreadStatus::from(&result.status), Ok(result)))
                    }
                    Some(run)
                        if run.status == fw::ManagedRunStatus::Running
                            && run.lease_until_ms > observed_at_ms =>
                    {
                        None
                    }
                    Some(run) => {
                        let status = if run.status == fw::ManagedRunStatus::Cancelled {
                            fw::ThreadStatus::Cancelled
                        } else {
                            fw::ThreadStatus::Failed
                        };
                        Some((status, Err(run.error.unwrap_or_else(|| {
                            "Thread execution needs recovery; verify external actions before continuing".into()
                        }))))
                    }
                    None if observed_at_ms.saturating_sub(record.run_admitted_at_ms) < 30_000 => {
                        None
                    }
                    None => Some((
                        fw::ThreadStatus::Failed,
                        Err("Thread was admitted but its native run did not start".into()),
                    )),
                };
                let Some(completion) = completion else {
                    continue;
                };
                record.summary.status = completion.0.clone();
                record.completion = Some(completion.clone());
                record.feedback_pending = record.owner.kind == "ASSISTANT";
                if let Err(error) = self.persist_thread(&mut record, false).await {
                    // Another process or a newly materialized thread won the CAS.
                    if matches!(
                        error.primary(),
                        fw::AgentError::State(fw::StateError::Conflict)
                    ) {
                        continue;
                    }
                    return Err(error);
                }
                if let Some(thread) = current {
                    let (_, receiver) = watch::channel(Some(completion));
                    let mut state = thread.state.lock().unwrap();
                    state.summary = record.summary.clone();
                    state.completion = receiver;
                    state.record = record;
                }
                recovered += 1;
            }
        }
        self.flush_thread_feedback().await?;
        Ok(recovered)
    }
    pub async fn flush_thread_feedback(&self) -> Result<usize, fw::AgentError> {
        let Some(sink) = self.0.thread_feedback.get() else {
            return Ok(0);
        };
        let pending = self.shared_database().await?.call(|connection| {
            let mut query = connection.prepare("SELECT payload_json FROM collaboration_thread_feedback WHERE delivered=0 ORDER BY created_at,event_key").map_err(|e| fw::StateError::Backend(e.to_string()))?;
            query.query_map([],|row| row.get::<_,String>(0)).map_err(|e| fw::StateError::Backend(e.to_string()))?.collect::<Result<Vec<_>,_>>().map_err(|e| fw::StateError::Backend(e.to_string()))
        }).await.map_err(failure)?;
        let mut delivered = 0;
        for payload in pending {
            let feedback: ThreadFeedback = serde_json::from_str(&payload).map_err(failure)?;
            sink.publish(feedback.clone()).await.map_err(failure)?;
            self.shared_database().await?.call(move |connection| {
                connection.execute("UPDATE collaboration_thread_feedback SET delivered=1 WHERE event_key=?1", [feedback.event_key]).map_err(|error| fw::StateError::Backend(error.to_string()))?;
                Ok::<_,fw::StateError>(())
            }).await.map_err(failure)?;
            delivered += 1;
        }
        Ok(delivered)
    }
}
