//! Bounded parallel event workers. Independent sessions never share a run lock.
use super::DudeService;
use crate::domain::dude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[async_trait::async_trait]
pub trait DudeExecutor: Send + Sync {
    async fn execute(
        &self,
        event: DudeEvent,
        session: DudeSession,
    ) -> Result<DudeExecutionOutcome, DudeError>;
    /// Inspect durable native state before any retry. None proves no run was admitted.
    async fn recover(&self, _event: &DudeEvent) -> Result<Option<DudeExecutionOutcome>, DudeError> {
        Err(DudeError::Runtime(
            "Execution needs recovery; verify previous external actions before continuing".into(),
        ))
    }
}
pub struct DudeExecutionOutcome {
    pub summary: String,
    pub awaiting_input: bool,
    pub notify: bool,
}
#[derive(Clone)]
pub struct DudeDispatcher {
    service: DudeService,
    executor: Arc<dyn DudeExecutor>,
    started: Arc<AtomicBool>,
}
impl DudeDispatcher {
    pub fn new(service: DudeService, executor: Arc<dyn DudeExecutor>) -> Self {
        Self {
            service,
            executor,
            started: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn start(&self) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let worker = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
            let permits = Arc::new(tokio::sync::Semaphore::new(4));
            loop {
                interval.tick().await;
                if let Err(e) = worker
                    .service
                    .schedule_heartbeats(chrono::Utc::now().timestamp_millis())
                {
                    tracing::error!("Dude schedule: {e}");
                }
                while let Ok(permit) = permits.clone().try_acquire_owned() {
                    let event = match worker
                        .service
                        .claim(chrono::Utc::now().timestamp_millis(), 90_000)
                    {
                        Ok(Some(e)) => e,
                        Ok(None) => break,
                        Err(e) => {
                            tracing::error!("Dude claim: {e}");
                            break;
                        }
                    };
                    let w = worker.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        w.execute_claimed(event).await;
                    });
                }
            }
        });
    }
    /// One bounded dispatch is also used by integration tests and embedded hosts.
    pub async fn dispatch_once(&self, now: i64) -> Result<bool, DudeError> {
        self.service.schedule_heartbeats(now)?;
        let Some(event) = self.service.claim(now, 90_000)? else {
            return Ok(false);
        };
        self.execute_claimed(event).await;
        Ok(true)
    }
    async fn execute_claimed(&self, mut event: DudeEvent) {
        let session = match event
            .session_code
            .as_deref()
            .map(|code| self.service.session(code))
            .transpose()
        {
            Ok(Some(Some(s))) => Ok(s),
            Ok(_) => {
                let (purpose, key) = match event.source {
                    DudeEventSource::UserMessage => {
                        (DudeSessionPurpose::Interactive, "interactive".to_owned())
                    }
                    DudeEventSource::Heartbeat => {
                        (DudeSessionPurpose::Heartbeat, event.event_id.clone())
                    }
                    DudeEventSource::TaskReminder => (
                        DudeSessionPurpose::Task,
                        event
                            .task_code
                            .clone()
                            .unwrap_or_else(|| event.event_id.clone()),
                    ),
                    DudeEventSource::ThreadFeedback => (
                        DudeSessionPurpose::Feedback,
                        event
                            .task_code
                            .clone()
                            .unwrap_or_else(|| event.event_id.clone()),
                    ),
                    DudeEventSource::MemoryBatch => {
                        (DudeSessionPurpose::MemoryBatch, event.event_id.clone())
                    }
                };
                self.service.resolve_session(
                    &event.dude_code,
                    purpose,
                    &key,
                    event.workspace_code.clone(),
                )
            }
            Err(e) => Err(e),
        };
        let result = match session {
            Ok(s) => match self.service.attach_session(&event, &s.session_code) {
                Ok(()) => {
                    event.session_code = Some(s.session_code.clone());
                    if event.attempts > 1 {
                        match self.executor.recover(&event).await {
                            Ok(Some(outcome)) => {
                                return self.finish_outcome(event, Ok(outcome)).await;
                            }
                            Err(error) => return self.finish_outcome(event, Err(error)).await,
                            Ok(None) => {}
                        }
                    }
                    let execute = self.executor.execute(event.clone(), s);
                    tokio::pin!(execute);
                    let mut renew = tokio::time::interval(std::time::Duration::from_secs(20));
                    renew.tick().await;
                    loop {
                        tokio::select! {
                            result = &mut execute => break result,
                            _ = renew.tick() => {
                                if let Err(error) = self.service.renew(&event, chrono::Utc::now().timestamp_millis(), 90_000) {
                                    tracing::error!("Dude event lease could not renew: {error}");
                                }
                            }
                        }
                    }
                }
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };
        if let Err(error) = &result {
            if event.attempts < 3 && matches!(self.executor.recover(&event).await, Ok(None)) {
                let delay = 5_000 * i64::from(event.attempts.max(1)).pow(2);
                if self
                    .service
                    .retry(
                        &event,
                        chrono::Utc::now().timestamp_millis(),
                        delay,
                        &error.to_string(),
                    )
                    .is_ok()
                {
                    return;
                }
            }
        }
        self.finish_outcome(event, result).await;
    }
    async fn finish_outcome(
        &self,
        event: DudeEvent,
        result: Result<DudeExecutionOutcome, DudeError>,
    ) {
        let (status, summary, notify) = match result {
            Ok(o) => (
                if o.awaiting_input {
                    DudeEventStatus::AwaitingInput
                } else {
                    DudeEventStatus::Completed
                },
                o.summary,
                o.notify || o.awaiting_input,
            ),
            Err(e) => (DudeEventStatus::Failed, e.to_string(), true),
        };
        if let Err(e) = self.service.finish_with_notification(
            &event,
            status,
            &summary,
            chrono::Utc::now().timestamp_millis(),
            notify,
        ) {
            tracing::error!("Dude completion fence: {e}");
            return;
        }
    }
}
