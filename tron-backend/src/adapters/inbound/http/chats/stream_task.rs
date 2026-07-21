use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::task::JoinHandle;

use crate::application::agent::AgentAguiStreamService;

#[derive(Clone)]
pub(crate) struct ChatStreamForwardingGate {
    open: Arc<AtomicBool>,
}

impl ChatStreamForwardingGate {
    pub(crate) fn new() -> Self {
        Self {
            open: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    fn close(&self) {
        self.open.store(false, Ordering::Release);
    }
}

pub(crate) struct ChatStreamLifecycleGuard {
    run_task: Option<JoinHandle<()>>,
    forwarding_gate: ChatStreamForwardingGate,
    stream_service: AgentAguiStreamService,
    stream_key: Option<String>,
}

impl ChatStreamLifecycleGuard {
    pub(crate) fn new(
        run_task: JoinHandle<()>,
        forwarding_gate: ChatStreamForwardingGate,
        stream_service: AgentAguiStreamService,
        stream_key: String,
        _run_code: String,
    ) -> Self {
        Self {
            run_task: Some(run_task),
            forwarding_gate,
            stream_service,
            stream_key: Some(stream_key),
        }
    }

    pub(crate) fn complete(&mut self) {
        self.forwarding_gate.close();
        self.run_task.take();
        self.deactivate_stream();
    }

    fn deactivate_stream(&mut self) {
        if let Some(stream_key) = self.stream_key.take() {
            let _ = self.stream_service.deactivate(&stream_key);
        }
    }
}

impl Drop for ChatStreamLifecycleGuard {
    fn drop(&mut self) {
        self.forwarding_gate.close();
        self.run_task.take();
        self.deactivate_stream();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use tokio::sync::oneshot;

    use crate::{
        application::agent::AgentAguiReplayStore,
        domain::agent::{AgentAguiEvent, AgentAguiProtocolEvent},
        shared::event_bus::{AguiEventSink, EventBusConfig},
    };

    use super::*;

    struct NotifyOnDrop(Option<oneshot::Sender<()>>);

    impl Drop for NotifyOnDrop {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.send(());
            }
        }
    }

    #[derive(Default)]
    struct RecordingReplayStore {
        protocol_events: Mutex<Vec<AgentAguiProtocolEvent>>,
    }

    impl AgentAguiReplayStore for RecordingReplayStore {
        fn append_protocol_event(
            &self,
            _stream_key: &str,
            event: AgentAguiProtocolEvent,
        ) -> Result<(), String> {
            self.protocol_events
                .lock()
                .expect("protocol events mutex poisoned")
                .push(event);
            Ok(())
        }

        fn replay_agui_events(&self, _stream_key: &str) -> Result<Vec<AgentAguiEvent>, String> {
            Ok(Vec::new())
        }

        fn replay_protocol_events(
            &self,
            _stream_key: &str,
        ) -> Result<Vec<AgentAguiProtocolEvent>, String> {
            Ok(self
                .protocol_events
                .lock()
                .expect("protocol events mutex poisoned")
                .clone())
        }
    }

    fn stream_service() -> AgentAguiStreamService {
        let config = EventBusConfig::default();
        AgentAguiStreamService::with_replay_store(
            Arc::new(AguiEventSink::new(&config)),
            Arc::new(RecordingReplayStore::default()),
        )
    }

    #[tokio::test]
    async fn drop_detaches_run_task_closes_forwarding_and_deactivates_stream() {
        let service = stream_service();
        service
            .activate("SESSION-1")
            .expect("stream should activate");
        let forwarding_gate = ChatStreamForwardingGate::new();
        let forwarding_gate_for_assert = forwarding_gate.clone();
        let (started_sender, started_receiver) = oneshot::channel();
        let (dropped_sender, dropped_receiver) = oneshot::channel();
        let run_task = tokio::spawn(async move {
            let _notify = NotifyOnDrop(Some(dropped_sender));
            let _ = started_sender.send(());
            future::pending::<()>().await;
        });
        started_receiver
            .await
            .expect("run task should start before guard drop");

        let guard = ChatStreamLifecycleGuard::new(
            run_task,
            forwarding_gate,
            service.clone(),
            "SESSION-1".to_string(),
            "RUN-1".to_string(),
        );
        assert!(forwarding_gate_for_assert.is_open());

        drop(guard);

        assert!(!forwarding_gate_for_assert.is_open());
        assert!(
            tokio::time::timeout(Duration::from_millis(25), dropped_receiver)
                .await
                .is_err(),
            "dropping the observer must not abort the run task"
        );
        let snapshot = service
            .events("SESSION-1")
            .expect("stream snapshot should load after guard drop");
        assert_eq!(snapshot.active_streams, 0);
        let protocol_events = service
            .protocol_events("SESSION-1")
            .expect("protocol events should load after guard drop");
        assert!(protocol_events.is_empty());
    }
}
