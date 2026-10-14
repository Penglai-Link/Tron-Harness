//! Control is a lookup; the SDK handle and wait remain the execution owner.
use super::FrameworkHost;
use crate::domain::agent::{
    AgentError, AgentNativeInteraction, AgentNativeInteractionKind, AgentNativeRunOutcome,
    AgentNativeRunStatus, AgentRunContext,
};
use eliteforge_agent_core as fw;
use eliteforge_agent_framework::{Agent, RunControl, RunHandle};
use eliteforge_agent_official_plugin_agui::AgUiStream;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

#[derive(Default)]
pub(super) struct NativeRuns {
    entries: Mutex<BTreeMap<String, Arc<Mutex<NativeControl>>>>,
    drained: tokio::sync::Notify,
}
#[derive(Default)]
struct NativeControl {
    cancelled: bool,
    sdk: Option<(Agent, RunControl)>,
}
#[derive(Clone)]
pub(super) struct NativeRegistration(Arc<Registered>);
struct Registered {
    host: Weak<super::Host>,
    key: String,
    control: Arc<Mutex<NativeControl>>,
}
impl Drop for Registered {
    fn drop(&mut self) {
        if let Some(host) = self.host.upgrade() {
            let mut entries = host
                .control
                .entries
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if entries
                .get(&self.key)
                .is_some_and(|entry| Arc::ptr_eq(entry, &self.control))
            {
                entries.remove(&self.key);
            }
            drop(entries);
            host.control.drained.notify_waiters();
        }
    }
}
impl NativeRegistration {
    pub(super) fn ensure_active(&self) -> Result<(), AgentError> {
        if self
            .0
            .control
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cancelled
        {
            Err(AgentError::Cancelled)
        } else {
            Ok(())
        }
    }
    pub(super) fn run_id(&self) -> String {
        self.0
            .control
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sdk
            .as_ref()
            .map(|(_, control)| control.run_id().to_string())
            .unwrap_or_default()
    }
    pub(super) fn snapshot(
        &self,
        result: &fw::RunResult,
    ) -> Result<fw::RunExecutionSnapshot, fw::AgentError> {
        let state = self.0.control.lock().unwrap_or_else(|e| e.into_inner());
        let (agent, control) = state
            .sdk
            .as_ref()
            .ok_or_else(|| fw::AgentError::Run("native run control is missing".into()))?;
        agent.execution_snapshot(control, result)
    }
    fn attach(&self, agent: Agent, control: RunControl) -> Result<(), fw::AgentError> {
        let mut state = self.0.control.lock().unwrap_or_else(|e| e.into_inner());
        if state.cancelled {
            agent.cancel(&control)?;
        }
        state.sdk = Some((agent, control));
        Ok(())
    }
}
impl FrameworkHost {
    pub(crate) fn native_run_observer(&self) -> Arc<dyn Fn(&str) -> bool + Send + Sync> {
        let host = Arc::downgrade(&self.0);
        Arc::new(move |key| {
            host.upgrade().is_some_and(|host| {
                host.control
                    .entries
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .contains_key(key)
            })
        })
    }
    pub fn bind_presentation(
        &self,
        service: crate::application::agent::AgentAguiStreamService,
    ) -> Result<(), String> {
        let mut binding = self
            .0
            .presentation
            .write()
            .unwrap_or_else(|e| e.into_inner());
        self.ensure_open()?;
        if binding.is_some() {
            return Err("presentation is already installed".into());
        }
        *binding = Some(service);
        Ok(())
    }
    pub(super) fn reserve_native(&self, key: &str) -> Result<NativeRegistration, AgentError> {
        self.ensure_open()
            .map_err(AgentError::InvalidConfiguration)?;
        let mut entries = self
            .0
            .control
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if entries.contains_key(key) {
            return Err(AgentError::SessionBusy(key.into()));
        }
        let control = Arc::new(Mutex::new(NativeControl::default()));
        entries.insert(key.into(), control.clone());
        Ok(NativeRegistration(Arc::new(Registered {
            host: Arc::downgrade(&self.0),
            key: key.into(),
            control,
        })))
    }
    /// Queue one steering message into a still-executing native run keyed by
    /// its run code. Best-effort push used for detached child completion
    /// notifications; returns whether the run admitted the message.
    pub fn steer_native_run(&self, key: &str, message: &str) -> bool {
        let control = self
            .0
            .control
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned();
        if let Some(control) = control {
            let state = control.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((_, sdk)) = &state.sdk {
                return sdk.steer(message.to_owned()).is_ok();
            }
        }
        false
    }
    /// Cancel without releasing the SDK owner or the product reservation.
    pub fn cancel_run(&self, key: &str) -> Result<(), AgentError> {
        let control = self
            .0
            .control
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned();
        if let Some(control) = control {
            let mut state = control.lock().unwrap_or_else(|e| e.into_inner());
            state.cancelled = true;
            if let Some((agent, control)) = &state.sdk {
                agent.cancel(control).map_err(super::map_error)?;
            }
        }
        Ok(())
    }
    /// Stop new work and cancel every retained native run before process teardown.
    pub fn cancel_all_runs(&self) {
        self.0
            .closed
            .store(true, std::sync::atomic::Ordering::Release);
        let keys: Vec<_> = self
            .0
            .control
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect();
        for key in keys {
            let _ = self.cancel_run(&key);
        }
    }
    pub(super) async fn wait_native(
        &self,
        agent: Agent,
        handle: RunHandle,
        stream: AgUiStream,
        context: &AgentRunContext,
        registration: NativeRegistration,
    ) -> Result<fw::RunResult, fw::AgentError> {
        registration.attach(agent.clone(), agent.control(&handle)?)?;
        let context = context.clone();
        let service = self
            .0
            .presentation
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        // The detached owner retains wait/drain even when its caller is dropped.
        crate::app::telemetry::spawn_traced(async move {
            let _registration = registration;
            let (result, ()) = tokio::join!(
                agent.wait(handle),
                super::super::presentation::consume_native_agui(stream, &context, service)
            );
            result
        })
        .await
        .map_err(|error| fw::AgentError::Run(format!("native wait owner failed: {error}")))?
    }
    pub(super) async fn wait_native_child(
        &self,
        agent: Agent,
        handle: eliteforge_agent_framework::ChildRunHandle,
        stream: AgUiStream,
        context: &AgentRunContext,
        registration: NativeRegistration,
    ) -> Result<fw::RunResult, fw::AgentError> {
        registration.attach(agent, handle.control())?;
        let context = context.clone();
        let service = self
            .0
            .presentation
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        crate::app::telemetry::spawn_traced(async move {
            let _registration = registration;
            let (result, ()) = tokio::join!(
                handle.wait(),
                super::super::presentation::consume_native_agui(stream, &context, service)
            );
            result
        })
        .await
        .map_err(|error| fw::AgentError::Run(format!("native child wait owner failed: {error}")))?
    }
    pub(super) async fn drain_native(&self) {
        loop {
            let notified = self.0.control.drained.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self
                .0
                .control
                .entries
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .is_empty()
            {
                break;
            }
            notified.await;
        }
    }
}
pub(super) fn native_outcome(
    run_id: String,
    result: &fw::RunResult,
) -> Result<AgentNativeRunOutcome, AgentError> {
    Ok(AgentNativeRunOutcome {
        run_id,
        status: match result.status {
            fw::RunStatus::Completed => AgentNativeRunStatus::Completed,
            fw::RunStatus::AwaitingInput => AgentNativeRunStatus::AwaitingInput,
            fw::RunStatus::StepLimit => AgentNativeRunStatus::StepLimit,
            fw::RunStatus::ToolCallLimit => AgentNativeRunStatus::ToolCallLimit,
        },
        interactions: result
            .interactions
            .iter()
            .map(|request| {
                Ok(AgentNativeInteraction {
                    id: request.id.to_string(),
                    kind: match request.kind {
                        fw::InteractionKind::Input => AgentNativeInteractionKind::Input,
                        fw::InteractionKind::Approval { invocation_id } => {
                            AgentNativeInteractionKind::Approval {
                                invocation_id: invocation_id.to_string(),
                            }
                        }
                    },
                    prompt: serde_json::to_value(&request.prompt)
                        .map_err(|error| AgentError::InvalidConfiguration(error.to_string()))?,
                    response_schema: request.response_schema.clone(),
                    presentation: request.presentation.clone(),
                    expires_at_unix_ms: request.expires_at_unix_ms,
                })
            })
            .collect::<Result<Vec<_>, AgentError>>()?,
    })
}

impl FrameworkHost {
    pub(crate) fn presentation_service(
        &self,
    ) -> Result<Option<crate::application::agent::AgentAguiStreamService>, String> {
        self.0
            .presentation
            .read()
            .map(|service| service.clone())
            .map_err(|_| "native presentation binding unavailable".into())
    }
}
