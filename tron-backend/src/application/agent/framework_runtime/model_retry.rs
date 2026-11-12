//! Session retry policy; dispatch, partial-output fencing and cancellation stay in Framework.
use eliteforge_agent_core as fw;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
pub(super) struct SessionModelRetry {
    failures: AtomicUsize,
    exhausted: Mutex<BTreeSet<fw::ModelRef>>,
}

impl SessionModelRetry {
    pub(super) fn failed(&self) {
        let _ = self
            .failures
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_add(1))
            });
    }

    pub(super) fn succeeded(&self) {
        self.failures.store(0, Ordering::Relaxed);
    }

    pub(super) fn exhaust(&self, model: &fw::ModelRef) {
        self.exhausted
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(model.clone());
    }

    pub(super) async fn wait(&self) {
        let failures = self.failures.load(Ordering::Relaxed);
        if failures == 0 {
            return;
        }
        let seconds = (2_u64 << failures.saturating_sub(1).min(4)).min(30);
        tracing::info!(
            attempt = failures.saturating_add(1),
            delay_seconds = seconds,
            "retrying session model request"
        );
        // Framework polls this future against the run cancellation token. Dropping
        // it cancels the sleep, including when the user stops during backoff.
        tokio::time::sleep(Duration::from_secs(seconds)).await;
    }

    pub(super) fn attach(self: Arc<Self>, request: &mut fw::RunRequest) {
        request.limits.max_model_retries = fw::RunLimits::UNLIMITED_COUNT;
        request.limits.max_model_calls_per_step = None;
        // Framework requires a finite, representable deadline. This removes the
        // normal three-day expiry for interactive sessions without overflowing
        // its clock or persisted millisecond deadline. Parent deadlines still win.
        request.limits.run_timeout = Duration::from_secs(100 * 365 * 24 * 60 * 60);
        // Framework requires a finite model deadline as well. Bind it to the
        // session lifetime; cancellation and parent deadlines remain effective.
        request.limits.model_timeout = request.limits.run_timeout;
        request.extensions.insert(self);
    }
}

#[derive(Default)]
struct Selection {
    current: Option<fw::ModelRef>,
    retries: BTreeMap<String, usize>,
}

/// Rotate through the currently authorized compatible candidates after a
/// transient failure, then revisit them on the next pass. Framework separately
/// excludes quota-exhausted models and rejects errors after partial output.
pub(super) struct SessionAutoRouter {
    state: Mutex<Selection>,
    retry: Arc<SessionModelRetry>,
}

impl SessionAutoRouter {
    pub(super) fn new(retry: Arc<SessionModelRetry>) -> Self {
        Self {
            state: Mutex::default(),
            retry,
        }
    }
}

impl fw::ChatAutoRouter for SessionAutoRouter {
    fn select(
        &self,
        context: &fw::AutoRouteContext<'_>,
        candidates: &[fw::AutoRouteCandidate],
    ) -> Result<fw::ModelRef, fw::AgentError> {
        let exhausted = self
            .retry
            .exhausted
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let candidates = candidates
            .iter()
            .filter(|candidate| !exhausted.contains(&candidate.model))
            .collect::<Vec<_>>();
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let previous_retries = state
            .retries
            .insert(context.call_key.into(), context.transient_failures)
            .unwrap_or(0);
        let current = state.current.as_ref().and_then(|model| {
            candidates
                .iter()
                .position(|candidate| &candidate.model == model)
        });
        let index = if context.transient_failures > previous_retries {
            context
                .previous
                .and_then(|model| {
                    candidates
                        .iter()
                        .position(|candidate| &candidate.model == model)
                })
                .map(|index| (index + 1) % candidates.len())
        } else {
            current
        }
        .unwrap_or(0);
        let model = candidates
            .get(index)
            .ok_or_else(|| fw::AgentError::Run("no authorized compatible model candidates".into()))?
            .model
            .clone();
        state.current = Some(model.clone());
        Ok(model)
    }
}
