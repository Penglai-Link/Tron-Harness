//! Durable Thread completion delivery independent of the parent Run.
use super::*;

/// Persisted routing envelope delivered independently of the originating Run/UI.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadFeedback {
    pub event_key: String,
    pub owner: crate::domain::agent::AgentCollaborationOwner,
    pub thread_code: String,
    pub origin_session_code: String,
    pub initiator_agent_code: Option<String>,
    pub status: fw::ThreadStatus,
    pub result: Option<fw::ThreadResult>,
    pub error: Option<String>,
}
#[async_trait::async_trait]
pub trait ThreadFeedbackSink: Send + Sync {
    /// Implementations durably enqueue idempotently by event_key before returning.
    async fn publish(&self, feedback: ThreadFeedback) -> Result<(), String>;
}

impl FrameworkHost {
    /// The product service must authenticate access to the platform Dude before calling.
    pub async fn dude_threads(
        &self,
        dude_code: &str,
    ) -> Result<Vec<fw::ThreadSummary>, fw::AgentError> {
        Ok(self
            .thread_records()
            .await?
            .into_iter()
            .filter(|record| record.owner.kind == "DUDE" && record.owner.code == dude_code)
            .map(|record| record.summary)
            .collect())
    }
    /// Closed Threads keep their native history; ordinary user chat APIs cannot resolve them.
    pub async fn dude_thread_history(
        &self,
        dude_code: &str,
        id: &str,
        query: fw::HistoryQuery,
    ) -> Result<fw::SessionHistory, fw::AgentError> {
        self.thread_record(id)
            .await?
            .filter(|record| record.owner.kind == "DUDE" && record.owner.code == dude_code)
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
        for record in self.thread_records().await? {
            let cached = self
                .0
                .collaboration_threads
                .read()
                .unwrap()
                .get(&record.summary.thread_id)
                .cloned();
            let _transition = if let Some(thread) = cached.as_ref() {
                Some(thread.transition.lock().await)
            } else {
                None
            };
            let Some(mut record) = self.thread_record(&record.summary.thread_id).await? else {
                continue;
            };
            if record.summary.status != fw::ThreadStatus::Running {
                continue;
            }
            let Some(run_id) = record.summary.latest_run_id else {
                continue;
            };
            let native = store.get_run(run_id).await;
            let completion = match native {
                Ok(run) if run.result.is_some() => {
                    let result = fw::ThreadResult::from_run(run_id, &run.result.unwrap());
                    Some((fw::ThreadStatus::from(&result.status), Ok(result)))
                }
                Ok(run) if run.status == fw::ManagedRunStatus::Running && run.lease_until_ms > fw::lifecycle_time_ms() => None,
                Ok(run) => Some((if run.status == fw::ManagedRunStatus::Cancelled { fw::ThreadStatus::Cancelled } else { fw::ThreadStatus::Failed }, Err(run.error.unwrap_or_else(|| "Thread execution needs recovery; verify external actions before continuing".into())))),
                Err(fw::StateError::NotFound) if fw::lifecycle_time_ms().saturating_sub(record.run_admitted_at_ms) < 30_000 => None,
                Err(fw::StateError::NotFound) => Some((fw::ThreadStatus::Failed, Err("Thread was admitted but its native run did not start".into()))),
                Err(error) => return Err(error.into()),
            };
            let Some(completion) = completion else {
                continue;
            };
            record.summary.status = completion.0.clone();
            record.completion = Some(completion.clone());
            record.feedback_pending = record.owner.kind == "DUDE";
            self.persist_thread(&mut record, false).await?;
            if let Some(thread) = self
                .0
                .collaboration_threads
                .read()
                .unwrap()
                .get(&record.summary.thread_id)
                .cloned()
            {
                let (_, receiver) = watch::channel(Some(completion));
                let mut state = thread.state.lock().unwrap();
                state.summary = record.summary.clone();
                state.completion = receiver;
                state.record = record;
            }
            recovered += 1;
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
