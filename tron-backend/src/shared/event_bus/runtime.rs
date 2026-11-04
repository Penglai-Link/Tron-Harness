use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
    thread,
};

use crate::shared::id::IdGenerator;

use super::{
    config::EventBusConfig,
    event::{
        BusEvent, BusEventDraft, BusEventValidationError, BusIngressLane, BusRuntimeStatsSnapshot,
        PublishOutcome,
    },
    sink::EventSink,
};

#[derive(Default)]
pub struct BusRuntimeStats {
    accepted: AtomicUsize,
    rejected_invalid: AtomicUsize,
    rejected_oversize: AtomicUsize,
    dropped_critical_full: AtomicUsize,
    dropped_standard_full: AtomicUsize,
    dropped_disconnected: AtomicUsize,
    sink_filtered: AtomicUsize,
    sink_enqueued: AtomicUsize,
    sink_dropped: AtomicUsize,
}

impl BusRuntimeStats {
    pub fn snapshot(&self) -> BusRuntimeStatsSnapshot {
        BusRuntimeStatsSnapshot {
            accepted: self.accepted.load(Ordering::Relaxed),
            rejected_invalid: self.rejected_invalid.load(Ordering::Relaxed),
            rejected_oversize: self.rejected_oversize.load(Ordering::Relaxed),
            dropped_critical_full: self.dropped_critical_full.load(Ordering::Relaxed),
            dropped_standard_full: self.dropped_standard_full.load(Ordering::Relaxed),
            dropped_disconnected: self.dropped_disconnected.load(Ordering::Relaxed),
            sink_filtered: self.sink_filtered.load(Ordering::Relaxed),
            sink_enqueued: self.sink_enqueued.load(Ordering::Relaxed),
            sink_dropped: self.sink_dropped.load(Ordering::Relaxed),
        }
    }

    fn increment_accepted(&self) {
        self.accepted.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_rejected_invalid(&self) {
        self.rejected_invalid.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_rejected_oversize(&self) {
        self.rejected_oversize.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_dropped_full(&self, lane: BusIngressLane) {
        match lane {
            BusIngressLane::Critical => {
                self.dropped_critical_full.fetch_add(1, Ordering::Relaxed);
            }
            BusIngressLane::Standard => {
                self.dropped_standard_full.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn increment_dropped_disconnected(&self) {
        self.dropped_disconnected.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_sink_filtered(&self) {
        self.sink_filtered.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_sink_enqueued(&self) {
        self.sink_enqueued.fetch_add(1, Ordering::Relaxed);
    }

    fn increment_sink_dropped(&self) {
        self.sink_dropped.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct EventBusHandle {
    inner: Option<Arc<EventBusInner>>,
}

struct EventBusInner {
    config: EventBusConfig,
    stats: Arc<BusRuntimeStats>,
    id_generator: Arc<dyn IdGenerator>,
    critical_tx: SyncSender<Arc<BusEvent>>,
    standard_tx: SyncSender<Arc<BusEvent>>,
    sinks: Arc<Vec<SinkRegistration>>,
}

struct SinkRegistration {
    sink: Arc<dyn EventSink>,
    sender: SyncSender<Arc<BusEvent>>,
}

pub struct EventBusRuntimeBuilder {
    config: EventBusConfig,
    id_generator: Arc<dyn IdGenerator>,
    sinks: Vec<(Arc<dyn EventSink>, usize)>,
    start_workers: bool,
}

impl EventBusHandle {
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    pub fn publish(&self, draft: BusEventDraft) -> PublishOutcome {
        let Some(inner) = &self.inner else {
            return PublishOutcome::Disabled;
        };
        let rejected_stream = draft.stream_key_ref().map(str::to_owned);
        let reject = || {
            if let Some(key) = &rejected_stream {
                for registration in inner.sinks.iter() {
                    registration
                        .sink
                        .delivery_rejected(key, "presentation event rejected");
                }
            }
        };
        let event = match BusEvent::from_draft(draft, inner.id_generator.as_ref(), &inner.config) {
            Ok(event) => event,
            Err(BusEventValidationError::Oversize {
                field,
                actual,
                limit,
            }) => {
                reject();
                inner.stats.increment_rejected_oversize();
                return PublishOutcome::RejectedOversize {
                    field,
                    actual,
                    limit,
                };
            }
            Err(BusEventValidationError::IdGenerationFailed(_)) => {
                reject();
                inner.stats.increment_rejected_invalid();
                return PublishOutcome::RejectedInvalid { field: "event_id" };
            }
            Err(
                BusEventValidationError::MissingField(field)
                | BusEventValidationError::ForbiddenField(field)
                | BusEventValidationError::InvalidField(field),
            ) => {
                reject();
                inner.stats.increment_rejected_invalid();
                return PublishOutcome::RejectedInvalid { field };
            }
        };
        let event_id = event.event_id;
        let lane = event.priority.ingress_lane();
        let sender = match lane {
            BusIngressLane::Critical => &inner.critical_tx,
            BusIngressLane::Standard => &inner.standard_tx,
        };
        match sender.try_send(Arc::new(event)) {
            Ok(()) => {
                inner.stats.increment_accepted();
                PublishOutcome::Accepted { event_id }
            }
            Err(TrySendError::Full(event)) => {
                notify_delivery_failure(
                    &event,
                    &inner.sinks,
                    "presentation ingress capacity exceeded",
                );
                inner.stats.increment_dropped_full(lane);
                PublishOutcome::DroppedIngressFull { lane }
            }
            Err(TrySendError::Disconnected(event)) => {
                notify_delivery_failure(&event, &inner.sinks, "presentation ingress disconnected");
                inner.stats.increment_dropped_disconnected();
                PublishOutcome::DroppedIngressDisconnected { lane }
            }
        }
    }

    pub fn stats_snapshot(&self) -> BusRuntimeStatsSnapshot {
        self.inner
            .as_ref()
            .map_or_else(BusRuntimeStatsSnapshot::default, |inner| {
                inner.stats.snapshot()
            })
    }
}

impl EventBusRuntimeBuilder {
    pub fn new(config: EventBusConfig, id_generator: Arc<dyn IdGenerator>) -> Self {
        Self {
            config,
            id_generator,
            sinks: Vec::new(),
            start_workers: true,
        }
    }

    pub fn register_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.sinks
            .push((sink, self.config.default_sink_capacity.max(1)));
        self
    }

    pub fn register_sink_with_capacity(
        mut self,
        sink: Arc<dyn EventSink>,
        capacity: usize,
    ) -> Self {
        self.sinks.push((sink, capacity.max(1)));
        self
    }



    pub fn build(self) -> EventBusHandle {
        if !self.config.enabled {
            return EventBusHandle::disabled();
        }
        let stats = Arc::new(BusRuntimeStats::default());
        let (critical_tx, critical_rx) = sync_channel(self.config.critical_ingress_capacity.max(1));
        let (standard_tx, standard_rx) = sync_channel(self.config.standard_ingress_capacity.max(1));

        let mut sink_registrations = Vec::with_capacity(self.sinks.len());
        for (sink, capacity) in self.sinks {
            let (sender, receiver) = sync_channel(capacity);
            if self.start_workers {
                spawn_sink_worker(sink.clone(), receiver);
            }
            sink_registrations.push(SinkRegistration { sink, sender });
        }
        let sink_registrations = Arc::new(sink_registrations);
        if self.start_workers {
            spawn_lane_worker(
                "tron-event-bus-critical",
                critical_rx,
                sink_registrations.clone(),
                stats.clone(),
            );
            spawn_lane_worker(
                "tron-event-bus-standard",
                standard_rx,
                sink_registrations.clone(),
                stats.clone(),
            );
        } else {
            spawn_parked_receiver("tron-event-bus-hold-critical", critical_rx);
            spawn_parked_receiver("tron-event-bus-hold-standard", standard_rx);
        }

        EventBusHandle {
            inner: Some(Arc::new(EventBusInner {
                config: self.config,
                stats,
                id_generator: self.id_generator,
                critical_tx,
                standard_tx,
                sinks: sink_registrations,
            })),
        }
    }
}

fn spawn_lane_worker(
    name: &str,
    receiver: Receiver<Arc<BusEvent>>,
    sinks: Arc<Vec<SinkRegistration>>,
    stats: Arc<BusRuntimeStats>,
) {
    let name = name.to_string();
    thread::Builder::new()
        .name(name)
        .spawn(move || {
            while let Ok(event) = receiver.recv() {
                dispatch_to_sinks(&event, sinks.as_ref(), &stats);
            }
        })
        .expect("event bus lane worker should start");
}

fn spawn_sink_worker(sink: Arc<dyn EventSink>, receiver: Receiver<Arc<BusEvent>>) {
    let name = format!("tron-event-sink-{}", sink.name());
    thread::Builder::new()
        .name(name)
        .spawn(move || {
            while let Ok(event) = receiver.recv() {
                sink.handle(event);
            }
        })
        .expect("event sink worker should start");
}

fn spawn_parked_receiver(name: &str, receiver: Receiver<Arc<BusEvent>>) {
    let name = name.to_string();
    thread::Builder::new()
        .name(name)
        .spawn(move || {
            let _receiver = receiver;
            thread::park();
        })
        .expect("event bus parked receiver should start");
}

fn dispatch_to_sinks(event: &Arc<BusEvent>, sinks: &[SinkRegistration], stats: &BusRuntimeStats) {
    for registration in sinks {
        if !registration.sink.accepts(event.as_ref()) {
            stats.increment_sink_filtered();
            continue;
        }
        match registration.sender.try_send(event.clone()) {
            Ok(()) => stats.increment_sink_enqueued(),
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                registration
                    .sink
                    .delivery_failed(event, "presentation sink capacity exceeded or disconnected");
                stats.increment_sink_dropped()
            }
        }
    }
}

fn notify_delivery_failure(event: &BusEvent, sinks: &[SinkRegistration], detail: &str) {
    for registration in sinks {
        if registration.sink.accepts(event) {
            registration.sink.delivery_failed(event, detail);
        }
    }
}
