use std::{
    collections::{BTreeMap, VecDeque},
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex, Weak},
};

use tokio::sync::watch;
#[path = "active_run_group.rs"]
mod group;

use crate::domain::chat::ChatMessageAttachment;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentSteerInput {
    pub page_context: Option<super::dto::AgentPageContext>,
    pub annotations: Vec<eliteforge_agent_core::MessageAnnotation>,
    pub steer_code: String,
    pub requested_skills: Vec<String>,
    pub content: String,
    pub attachments: Vec<ChatMessageAttachment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveRunError {
    AlreadyActive,
    SessionArchived,
    NoActiveRun,
    RunMismatch { active_run_code: String },
    RuntimeUnsupported { runtime_adapter_key: String },
    RunStarting,
    RunFinishing,
    RegistryUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveRunState {
    Running,
    Finishing,
}

impl ActiveRunState {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Running => "RUNNING",
            Self::Finishing => "FINISHING",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRunSnapshot {
    pub run_code: String,
    pub runtime_adapter_key: String,
    pub supports_steer: bool,
    pub state: ActiveRunState,
}

impl Display for ActiveRunError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyActive => formatter.write_str("session already has an active run"),
            Self::SessionArchived => formatter.write_str("session is archived"),
            Self::NoActiveRun => formatter.write_str("session has no active run"),
            Self::RunMismatch { active_run_code } => write!(
                formatter,
                "expected run does not match active run `{active_run_code}`"
            ),
            Self::RuntimeUnsupported {
                runtime_adapter_key,
            } => write!(
                formatter,
                "runtime adapter `{runtime_adapter_key}` does not support steer"
            ),
            Self::RunStarting => formatter.write_str("active run is starting"),
            Self::RunFinishing => formatter.write_str("active run is finishing"),
            Self::RegistryUnavailable => formatter.write_str("active run registry is unavailable"),
        }
    }
}

impl Error for ActiveRunError {}

#[derive(Clone, Default)]
pub struct ActiveRunRegistry {
    inner: Arc<ActiveRunRegistryInner>,
}

#[derive(Default)]
struct ActiveRunRegistryInner {
    entries: Mutex<BTreeMap<String, Arc<ActiveRunEntry>>>,
    admission: Mutex<()>,
    archived_sessions: Mutex<std::collections::BTreeSet<String>>,
    groups: Mutex<BTreeMap<String, Arc<group::ActiveRunGroup>>>,
}

struct ActiveRunEntry {
    run_code: String,
    runtime_adapter_key: String,
    supports_steer: bool,
    cancellation_sender: watch::Sender<bool>,
    state: Mutex<ActiveRunMutableState>,
}

#[derive(Default)]
struct ActiveRunMutableState {
    starting: bool,
    finishing: bool,
    pending: VecDeque<AgentSteerInput>,
}

pub struct ActiveRunGuard {
    registry: Weak<ActiveRunRegistryInner>,
    entry: Arc<ActiveRunEntry>,
    session_code: String,
}

#[derive(Clone)]
pub struct ActiveRunControl {
    entry: Arc<ActiveRunEntry>,
}

pub struct ActiveRunCancellation {
    receiver: watch::Receiver<bool>,
}

impl ActiveRunRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        session_code: &str,
        run_code: &str,
        runtime_adapter_key: &str,
        supports_steer: bool,
    ) -> Result<ActiveRunGuard, ActiveRunError> {
        self.insert(
            session_code,
            run_code,
            runtime_adapter_key,
            supports_steer,
            false,
        )
    }

    pub fn reserve(
        &self,
        session_code: &str,
        run_code: &str,
        runtime_adapter_key: &str,
        supports_steer: bool,
    ) -> Result<ActiveRunGuard, ActiveRunError> {
        self.insert(
            session_code,
            run_code,
            runtime_adapter_key,
            supports_steer,
            true,
        )
    }

    fn insert(
        &self,
        session_code: &str,
        run_code: &str,
        runtime_adapter_key: &str,
        supports_steer: bool,
        starting: bool,
    ) -> Result<ActiveRunGuard, ActiveRunError> {
        let _admission = self
            .inner
            .admission
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        let mut entries = self
            .inner
            .entries
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if self
            .inner
            .archived_sessions
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .contains(session_code)
        {
            return Err(ActiveRunError::SessionArchived);
        }
        if entries.contains_key(session_code) {
            return Err(ActiveRunError::AlreadyActive);
        }
        self.admit_group_child(session_code, run_code, runtime_adapter_key)?;
        let (cancellation_sender, _) = watch::channel(false);
        let entry = Arc::new(ActiveRunEntry {
            run_code: run_code.to_string(),
            runtime_adapter_key: runtime_adapter_key.to_string(),
            supports_steer,
            cancellation_sender,
            state: Mutex::new(ActiveRunMutableState {
                starting,
                ..ActiveRunMutableState::default()
            }),
        });
        entries.insert(session_code.to_string(), entry.clone());
        Ok(ActiveRunGuard {
            registry: Arc::downgrade(&self.inner),
            entry,
            session_code: session_code.to_string(),
        })
    }

    /// Atomically block new runs for an idle session. Existing reservations
    /// win the race and make archiving fail with AlreadyActive.
    pub fn retire_session(&self, session_code: &str) -> Result<(), ActiveRunError> {
        let _admission = self
            .inner
            .admission
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if self
            .inner
            .entries
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .contains_key(session_code)
            || self.group_snapshot(session_code)?.is_some()
        {
            return Err(ActiveRunError::AlreadyActive);
        }
        self.inner
            .archived_sessions
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .insert(session_code.to_string());
        Ok(())
    }

    /// Reopen run admission after a successful restore or failed archive write.
    pub fn reactivate_session(&self, session_code: &str) -> Result<(), ActiveRunError> {
        let _admission = self
            .inner
            .admission
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        self.inner
            .archived_sessions
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .remove(session_code);
        Ok(())
    }

    pub fn is_retired(&self, session_code: &str) -> Result<bool, ActiveRunError> {
        self.inner
            .archived_sessions
            .lock()
            .map(|sessions| sessions.contains(session_code))
            .map_err(|_| ActiveRunError::RegistryUnavailable)
    }

    pub fn control(
        &self,
        session_code: &str,
        expected_run_code: &str,
    ) -> Result<ActiveRunControl, ActiveRunError> {
        let entry = self.entry(session_code)?;
        validate_expected_run(&entry, expected_run_code)?;
        Ok(ActiveRunControl { entry })
    }

    pub fn accept_steer<T, E, F>(
        &self,
        session_code: &str,
        expected_run_code: &str,
        accept: F,
    ) -> Result<Result<T, E>, ActiveRunError>
    where
        F: FnOnce() -> Result<(T, AgentSteerInput), E>,
    {
        let entry = self.entry(session_code)?;
        validate_expected_run(&entry, expected_run_code)?;
        entry.accept_steer(accept)
    }

    pub fn snapshot(
        &self,
        session_code: &str,
    ) -> Result<Option<ActiveRunSnapshot>, ActiveRunError> {
        loop {
            let entry = {
                self.inner
                    .entries
                    .lock()
                    .map_err(|_| ActiveRunError::RegistryUnavailable)?
                    .get(session_code)
                    .cloned()
            };
            let Some(entry) = entry else {
                return self.group_snapshot(session_code);
            };
            let (starting, finishing) = {
                let state = entry
                    .state
                    .lock()
                    .map_err(|_| ActiveRunError::RegistryUnavailable)?;
                (state.starting, state.finishing)
            };
            let still_current = self
                .inner
                .entries
                .lock()
                .map_err(|_| ActiveRunError::RegistryUnavailable)?
                .get(session_code)
                .is_some_and(|current| Arc::ptr_eq(current, &entry));
            if still_current {
                return Ok(Some(ActiveRunSnapshot {
                    run_code: entry.run_code.clone(),
                    runtime_adapter_key: entry.runtime_adapter_key.clone(),
                    supports_steer: entry.supports_steer && !starting && !finishing,
                    state: if finishing {
                        ActiveRunState::Finishing
                    } else {
                        ActiveRunState::Running
                    },
                }));
            }
        }
    }

    pub fn request_stop(
        &self,
        session_code: &str,
        expected_run_code: &str,
    ) -> Result<ActiveRunSnapshot, ActiveRunError> {
        let group = self.stop_group(session_code, expected_run_code)?;
        let entry = match self.entry(session_code) {
            Ok(entry) => entry,
            Err(ActiveRunError::NoActiveRun) if group.is_some() => {
                return Ok(group.expect("group snapshot"));
            }
            Err(error) => return Err(error),
        };
        validate_expected_run(&entry, expected_run_code)?;
        {
            let mut state = entry
                .state
                .lock()
                .map_err(|_| ActiveRunError::RegistryUnavailable)?;
            if state.finishing {
                return Err(ActiveRunError::RunFinishing);
            }
            state.finishing = true;
            state.pending.clear();
        }
        entry.cancellation_sender.send_replace(true);
        Ok(ActiveRunSnapshot {
            run_code: entry.run_code.clone(),
            runtime_adapter_key: entry.runtime_adapter_key.clone(),
            supports_steer: entry.supports_steer,
            state: ActiveRunState::Finishing,
        })
    }

    /// Cancel every in-process run before Electron tears down the profile
    /// sidecar. This gives the run executor a chance to persist an interrupted
    /// terminal state instead of leaving a run looking active after a switch.
    pub fn request_stop_all(&self) -> Result<usize, ActiveRunError> {
        let entries = self
            .inner
            .entries
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        let mut stopped = self.stop_groups()?;
        for entry in entries.values() {
            let mut state = entry
                .state
                .lock()
                .map_err(|_| ActiveRunError::RegistryUnavailable)?;
            if !state.finishing {
                state.finishing = true;
                state.pending.clear();
                entry.cancellation_sender.send_replace(true);
                stopped += 1;
            }
        }
        Ok(stopped)
    }

    pub fn activate_reservation(
        &self,
        guard: &ActiveRunGuard,
        session_code: &str,
        run_code: &str,
        runtime_adapter_key: &str,
        supports_steer: bool,
    ) -> Result<(), ActiveRunError> {
        let registry = guard
            .registry
            .upgrade()
            .ok_or(ActiveRunError::RegistryUnavailable)?;
        if !Arc::ptr_eq(&registry, &self.inner) || guard.session_code != session_code {
            return Err(ActiveRunError::RegistryUnavailable);
        }
        validate_expected_run(&guard.entry, run_code)?;
        if guard.entry.runtime_adapter_key != runtime_adapter_key
            || guard.entry.supports_steer != supports_steer
        {
            return Err(ActiveRunError::RegistryUnavailable);
        }
        let is_current = self
            .inner
            .entries
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .get(session_code)
            .is_some_and(|entry| Arc::ptr_eq(entry, &guard.entry));
        if !is_current {
            return Err(ActiveRunError::NoActiveRun);
        }
        let mut state = guard
            .entry
            .state
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if state.finishing {
            return Err(ActiveRunError::RunFinishing);
        }
        state.starting = false;
        Ok(())
    }

    fn entry(&self, session_code: &str) -> Result<Arc<ActiveRunEntry>, ActiveRunError> {
        self.inner
            .entries
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .get(session_code)
            .cloned()
            .ok_or(ActiveRunError::NoActiveRun)
    }
}

impl ActiveRunEntry {
    fn accept_steer<T, E, F>(&self, accept: F) -> Result<Result<T, E>, ActiveRunError>
    where
        F: FnOnce() -> Result<(T, AgentSteerInput), E>,
    {
        if !self.supports_steer {
            return Err(ActiveRunError::RuntimeUnsupported {
                runtime_adapter_key: self.runtime_adapter_key.clone(),
            });
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if state.finishing {
            return Err(ActiveRunError::RunFinishing);
        }
        if state.starting {
            return Err(ActiveRunError::RunStarting);
        }
        match accept() {
            Ok((value, input)) => {
                state.pending.push_back(input);
                Ok(Ok(value))
            }
            Err(error) => Ok(Err(error)),
        }
    }
}

impl ActiveRunControl {
    pub fn take_pending(&self) -> Result<Vec<AgentSteerInput>, ActiveRunError> {
        let mut state = self
            .entry
            .state
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if state.finishing {
            return Ok(Vec::new());
        }
        Ok(state.pending.drain(..).collect())
    }

    pub fn take_pending_or_begin_finishing(&self) -> Result<Vec<AgentSteerInput>, ActiveRunError> {
        let mut state = self
            .entry
            .state
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if state.finishing {
            return Ok(Vec::new());
        }
        if state.pending.is_empty() {
            state.finishing = true;
            return Ok(Vec::new());
        }
        Ok(state.pending.drain(..).collect())
    }
}

impl ActiveRunGuard {
    pub fn cancellation(&self) -> ActiveRunCancellation {
        ActiveRunCancellation {
            receiver: self.entry.cancellation_sender.subscribe(),
        }
    }
}

impl ActiveRunCancellation {
    pub async fn cancelled(&mut self) {
        loop {
            if *self.receiver.borrow_and_update() {
                return;
            }
            if self.receiver.changed().await.is_err() {
                return;
            }
        }
    }
}

impl Drop for ActiveRunGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.entry.state.lock() {
            state.finishing = true;
        }
        let Some(registry) = self.registry.upgrade() else {
            return;
        };
        let Ok(mut entries) = registry.entries.lock() else {
            return;
        };
        let matches_guard = entries
            .get(&self.session_code)
            .is_some_and(|entry| Arc::ptr_eq(entry, &self.entry));
        if matches_guard {
            entries.remove(&self.session_code);
        }
    }
}

fn validate_expected_run(
    entry: &ActiveRunEntry,
    expected_run_code: &str,
) -> Result<(), ActiveRunError> {
    if entry.run_code == expected_run_code {
        return Ok(());
    }
    Err(ActiveRunError::RunMismatch {
        active_run_code: entry.run_code.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    };

    use super::{ActiveRunError, ActiveRunRegistry, AgentSteerInput};

    #[test]
    fn captured_entry_rejects_acceptance_after_guard_closes_it() {
        let registry = ActiveRunRegistry::new();
        let guard = registry
            .register("SESSION-1", "RUN-1", "REACT", true)
            .expect("active run should register");
        let captured_entry = registry
            .entry("SESSION-1")
            .expect("acceptance should be able to capture the active entry");
        let start = Arc::new(Barrier::new(2));
        let callback_called = Arc::new(AtomicBool::new(false));
        let accept_task = {
            let start = start.clone();
            let callback_called = callback_called.clone();
            std::thread::spawn(move || {
                start.wait();
                let result = captured_entry.accept_steer(|| {
                    callback_called.store(true, Ordering::SeqCst);
                    Ok::<_, ()>((
                        (),
                        AgentSteerInput {
                            page_context: None,
                            annotations: Vec::new(),
                            requested_skills: Vec::new(),
                            steer_code: "MSG-1".into(),
                            content: "too late".into(),
                            attachments: Vec::new(),
                        },
                    ))
                });
                (result, captured_entry)
            })
        };

        drop(guard);
        start.wait();

        let (result, captured_entry) = accept_task.join().expect("acceptance task should join");
        assert_eq!(result, Err(ActiveRunError::RunFinishing));
        assert!(!callback_called.load(Ordering::SeqCst));
        assert!(
            captured_entry
                .state
                .lock()
                .expect("active run state should lock")
                .pending
                .is_empty()
        );
        assert!(matches!(
            registry.control("SESSION-1", "RUN-1"),
            Err(ActiveRunError::NoActiveRun)
        ));
    }
}
