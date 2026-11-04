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
