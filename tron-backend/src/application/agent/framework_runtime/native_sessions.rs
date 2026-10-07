//! Host-owned native references. This branch never contains external checkpoints.
use super::external::ExternalExecution;
use crate::{application::agent::AgentInvocationError, domain::agent::AgentRunResult};
use eliteforge_agent_core as fw;

use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(super) struct Bindings(Mutex<Vec<Arc<tokio::sync::Mutex<Binding>>>>);
pub(super) struct BindingGuard {
    binding: Arc<tokio::sync::Mutex<Binding>>,
    pub session: Option<fw::ExternalSessionRef>,
}
impl Bindings {
    pub fn track(&self, binding: Binding) -> BindingGuard {
        let session = binding.session.clone();
        let binding = Arc::new(tokio::sync::Mutex::new(binding));
        let mut owned = self.0.lock().unwrap();
        owned.retain(|entry| !entry.try_lock().is_ok_and(|entry| entry.finished));
        owned.push(binding.clone());
        BindingGuard { binding, session }
    }
    pub async fn close(&self) -> eliteforge_agent_plugin_core::ShutdownReport {
        let owned = std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()));
        let mut owners = tokio::task::JoinSet::new();
        for binding in owned {
            owners.spawn(async move { binding.lock().await.failed().await });
        }
        let mut report = eliteforge_agent_plugin_core::ShutdownReport::default();
        while let Some(result) = owners.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => report.errors.push(format!(
                    "external session binding close: {}",
                    error.message()
                )),
                Err(error) => {
                    report.panics += usize::from(error.is_panic());
                    report
                        .errors
                        .push(format!("external session binding close owner: {error}"));
                }
            }
        }
        report
    }
}
impl BindingGuard {
    pub async fn failed(&self) -> Result<(), AgentInvocationError> {
        self.binding.lock().await.failed().await
    }
    pub async fn save(&self, result: &AgentRunResult) -> Result<(), AgentInvocationError> {
        self.binding.lock().await.save(result).await
    }
}
impl Drop for BindingGuard {
    fn drop(&mut self) {
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let binding = self.binding.clone();
            runtime.spawn(async move {
                let _ = binding.lock().await.failed().await;
            });
        }
        // Host keeps a strong reference until finish, so close can drain even
        // when this asynchronous Drop cleanup has not been polled yet.
    }
}

pub(super) struct Binding {
    lease: Box<dyn fw::ConversationSession>,
    scope: String,
    runtime: String,
    saved_session: Option<String>,
    finished: bool,
    pub session: Option<fw::ExternalSessionRef>,
}
fn error(value: impl std::fmt::Display) -> AgentInvocationError {
    AgentInvocationError::RuntimeFailed(value.to_string())
}
impl Binding {
    pub async fn open(
        store: &dyn fw::ConversationStore,
        conversation: &str,
        execution: &ExternalExecution,
        required: bool,
        bindings: &Bindings,
    ) -> Result<BindingGuard, AgentInvocationError> {
        let runtime = execution.transport.key().code().to_owned();
        let scope = execution
            .session_scope()
            .ok_or_else(|| error("external scope is missing"))?;
        let lease = store
            .open_branch(
                &fw::ConversationKey::new(conversation).branch(format!("tron.external/{runtime}")),
            )
            .await
            .map_err(error)?;
        let mut guard = bindings.track(Self {
            lease,
            scope,
            runtime,
            saved_session: None,
            finished: false,
            session: None,
        });
        let result = guard
            .binding
            .lock()
            .await
            .initialize(store, conversation, execution, required, bindings)
            .await;
        if let Err(error) = result {
            guard.failed().await?;
            return Err(error);
        }
        guard.session = guard.binding.lock().await.session.clone();
        Ok(guard)
    }
    async fn initialize(
        &mut self,
        _store: &dyn fw::ConversationStore,
        _conversation: &str,
        execution: &ExternalExecution,
        required: bool,
        _bindings: &Bindings,
    ) -> Result<(), AgentInvocationError> {
        let snapshot = self.lease.snapshot().await.map_err(error)?;
        let requested = execution.context.system_agent_session_id.as_deref();
        let records = snapshot
            .messages
            .iter()
            .filter_map(|m| m.model_context.as_ref())
            .filter(|m| {
                m.format == "tron.external-session.v2"
                    && m.protocol == "external-agent"
                    && m.model == fw::ModelRef::new("tron.external", &self.runtime)
            })
            .collect::<Vec<_>>();
        let saved_session = records
            .iter()
            .rev()
            .find(|m| m.data["scope"].as_str() == Some(&self.scope))
            .and_then(|m| m.data["sessionId"].as_str())
            .map(str::to_owned);
        let matching = records.iter().rev().find(|m| {
            m.data["scope"].as_str() == Some(&self.scope)
                && requested.is_none_or(|id| m.data["sessionId"].as_str() == Some(id))
        });
        let mut session = matching
            .and_then(|m| m.data["sessionId"].as_str())
            .map(str::to_owned);
        let known = requested.is_some_and(|id| {
            records
                .iter()
                .any(|m| m.data["sessionId"].as_str() == Some(id))
        });
        if session.is_none() {
            if known && required {
                return Err(error("native session belongs to a different runtime scope"));
            }
            if !known {
                session = requested.map(str::to_owned);
            }
        }
        if required && session.is_none() {
            return Err(error("no compatible native session to resume"));
        }
        self.saved_session = saved_session;
        self.session = session.map(|session_id| fw::ExternalSessionRef {
            session_id,
            run_id: None,
        });
        Ok(())
    }
    pub async fn failed(&mut self) -> Result<(), AgentInvocationError> {
        self.finish(fw::ConversationOutcome::Failed).await
    }
    async fn finish(
        &mut self,
        outcome: fw::ConversationOutcome,
    ) -> Result<(), AgentInvocationError> {
        if !self.finished {
            self.lease.finish(outcome).await.map_err(error)?;
            self.finished = true;
        }
        Ok(())
    }
    pub async fn save(&mut self, result: &AgentRunResult) -> Result<(), AgentInvocationError> {
        if self.finished {
            return Err(error("native binding already closed"));
        }
        if let Some(id) = &result.system_agent_session_id {
            if self.saved_session.as_deref() == Some(id.as_str()) {
                return self.finish(fw::ConversationOutcome::Completed).await;
            }
            let mut message = fw::Message::assistant("");
            message.model_context = Some(fw::ModelMessageContext {
                model: fw::ModelRef::new("tron.external", &self.runtime),
                protocol: "external-agent".into(),
                format: "tron.external-session.v2".into(),
                data: serde_json::json!({"scope": self.scope, "sessionId": id}),
            });
            self.lease.append(&[message]).await.map_err(error)?;
        }
        self.finish(fw::ConversationOutcome::Completed).await
    }
}
