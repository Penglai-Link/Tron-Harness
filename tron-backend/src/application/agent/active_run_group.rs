//! Product session admission linked to an SDK root continuation control.
use super::*;
use eliteforge_agent_framework::{Agent, RunControl};

pub(crate) struct ActiveRunGroupGuard {
    registry: Weak<ActiveRunRegistryInner>,
    session: String,
    group: Arc<ActiveRunGroup>,
}
#[derive(Clone)]
pub(crate) struct ActiveRunGroupAdmission(Arc<ActiveRunGroup>);
pub(super) struct ActiveRunGroup {
    state: Mutex<GroupState>,
}
struct GroupState {
    run_code: String,
    adapter: String,
    finishing: bool,
    native: Option<(Agent, RunControl)>,
    cancellation: Option<tokio_util::sync::CancellationToken>,
}
impl ActiveRunRegistry {
    pub(crate) fn continuation_group(
        &self,
        session: &str,
        initial_run: &str,
        adapter: &str,
    ) -> Result<ActiveRunGroupGuard, ActiveRunError> {
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
            .contains_key(session)
        {
            return Err(ActiveRunError::AlreadyActive);
        }
        let mut groups = self
            .inner
            .groups
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if groups.contains_key(session) {
            return Err(ActiveRunError::AlreadyActive);
        }
        let group = Arc::new(ActiveRunGroup {
            state: Mutex::new(GroupState {
                run_code: initial_run.into(),
                adapter: adapter.into(),
                finishing: false,
                native: None,
                cancellation: None,
            }),
        });
        groups.insert(session.into(), group.clone());
        Ok(ActiveRunGroupGuard {
            registry: Arc::downgrade(&self.inner),
            session: session.into(),
            group,
        })
    }
    pub(crate) fn bind_continuation_context(
        &self,
        session: &str,
        parent: &eliteforge_agent_core::RunContext,
    ) -> Result<(), ActiveRunError> {
        let admission = parent
            .extensions
            .get::<ActiveRunGroupAdmission>()
            .ok_or(ActiveRunError::RegistryUnavailable)?;
        let group = self
            .inner
            .groups
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .get(session)
            .cloned()
            .ok_or(ActiveRunError::NoActiveRun)?;
        if !Arc::ptr_eq(&group, &admission.0) {
            return Err(ActiveRunError::RegistryUnavailable);
        }
        let mut state = group
            .state
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        state.cancellation = Some(parent.cancellation.clone());
        if state.finishing {
            parent.cancellation.cancel();
            return Err(ActiveRunError::RunFinishing);
        }
        Ok(())
    }
    pub(super) fn admit_group_child(
        &self,
        session: &str,
        run_code: &str,
        adapter: &str,
    ) -> Result<(), ActiveRunError> {
        let group = self
            .inner
            .groups
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .get(session)
            .cloned();
        let Some(group) = group else { return Ok(()) };
        let admitted = eliteforge_agent_framework::current_continuation_parent()
            .and_then(|p| p.extensions.get::<ActiveRunGroupAdmission>());
        if !admitted.is_some_and(|a| Arc::ptr_eq(&a.0, &group)) {
            return Err(ActiveRunError::AlreadyActive);
        }
        let mut state = group
            .state
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if state.finishing {
            return Err(ActiveRunError::RunFinishing);
        }
        state.run_code = run_code.into();
        state.adapter = adapter.into();
        Ok(())
    }
    pub(super) fn group_snapshot(
        &self,
        session: &str,
    ) -> Result<Option<ActiveRunSnapshot>, ActiveRunError> {
        let group = self
            .inner
            .groups
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .get(session)
            .cloned();
        group
            .map(|g| {
                let s = g
                    .state
                    .lock()
                    .map_err(|_| ActiveRunError::RegistryUnavailable)?;
                Ok(snapshot(&s))
            })
            .transpose()
    }
    pub(super) fn stop_group(
        &self,
        session: &str,
        expected: &str,
    ) -> Result<Option<ActiveRunSnapshot>, ActiveRunError> {
        let group = self
            .inner
            .groups
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?
            .get(session)
            .cloned();
        group
            .map(|g| {
                let mut s = g
                    .state
                    .lock()
                    .map_err(|_| ActiveRunError::RegistryUnavailable)?;
                if s.run_code != expected {
                    return Err(ActiveRunError::RunMismatch {
                        active_run_code: s.run_code.clone(),
                    });
                }
                if s.finishing {
                    return Err(ActiveRunError::RunFinishing);
                }
                s.finishing = true;
                if let Some(token) = &s.cancellation {
                    token.cancel();
                }
                if let Some((agent, control)) = &s.native {
                    let _ = agent.cancel(control);
                }
                Ok(snapshot(&s))
            })
            .transpose()
    }
    pub(super) fn stop_groups(&self) -> Result<usize, ActiveRunError> {
        let groups = self
            .inner
            .groups
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        let mut count = 0;
        for group in groups.values() {
            let mut s = group
                .state
                .lock()
                .map_err(|_| ActiveRunError::RegistryUnavailable)?;
            if !s.finishing {
                s.finishing = true;
                count += 1;
                if let Some(token) = &s.cancellation {
                    token.cancel();
                }
                if let Some((agent, control)) = &s.native {
                    let _ = agent.cancel(control);
                }
            }
        }
        Ok(count)
    }
}
fn snapshot(s: &GroupState) -> ActiveRunSnapshot {
    ActiveRunSnapshot {
        run_code: s.run_code.clone(),
        runtime_adapter_key: s.adapter.clone(),
        supports_steer: false,
        operation_kind: None,
        state: if s.finishing {
            ActiveRunState::Finishing
        } else {
            ActiveRunState::Running
        },
    }
}
impl ActiveRunGroupGuard {
    pub(crate) fn attach(&self, extensions: &mut eliteforge_agent_core::Extensions) {
        extensions.insert(ActiveRunGroupAdmission(self.group.clone()));
    }
    pub(crate) fn bind(&self, agent: Agent, control: RunControl) -> Result<(), ActiveRunError> {
        let mut s = self
            .group
            .state
            .lock()
            .map_err(|_| ActiveRunError::RegistryUnavailable)?;
        if s.finishing {
            let _ = agent.cancel(&control);
        }
        s.native = Some((agent, control));
        Ok(())
    }
}
impl Drop for ActiveRunGroupGuard {
    fn drop(&mut self) {
        if let Ok(s) = self.group.state.lock() {
            if let Some(token) = &s.cancellation {
                token.cancel();
            }
            if let Some((agent, control)) = &s.native {
                let _ = agent.cancel(control);
            }
        }
        if let Some(registry) = self.registry.upgrade() {
            if let Ok(mut groups) = registry.groups.lock() {
                if groups
                    .get(&self.session)
                    .is_some_and(|g| Arc::ptr_eq(g, &self.group))
                {
                    groups.remove(&self.session);
                }
            }
        }
    }
}
