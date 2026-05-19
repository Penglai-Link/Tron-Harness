use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use super::{BusEvent, BusRouteClass, EventSink};

pub struct RecordingEventSink {
    name: &'static str,
    route_class: Option<BusRouteClass>,
    inner: Mutex<Vec<Arc<BusEvent>>>,
    signal: Condvar,
}

impl RecordingEventSink {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            route_class: None,
            inner: Mutex::new(Vec::new()),
            signal: Condvar::new(),
        }
    }

    pub fn with_route_class(name: &'static str, route_class: BusRouteClass) -> Self {
        Self {
            name,
            route_class: Some(route_class),
            inner: Mutex::new(Vec::new()),
            signal: Condvar::new(),
        }
    }

    pub fn wait_for_len(&self, len: usize, timeout: Duration) -> bool {
        let guard = self.inner.lock().expect("recording sink mutex poisoned");
        let (guard, _) = self
            .signal
            .wait_timeout_while(guard, timeout, |events| events.len() < len)
            .expect("recording sink condvar wait should succeed");
        guard.len() >= len
    }

    pub fn events(&self) -> Vec<BusEvent> {
        self.inner
            .lock()
            .expect("recording sink mutex poisoned")
            .iter()
            .map(|event| event.as_ref().clone())
            .collect()
    }
}

impl EventSink for RecordingEventSink {
    fn name(&self) -> &'static str {
        self.name
    }

    fn accepts(&self, event: &BusEvent) -> bool {
        self.route_class.is_none_or(|route_class| route_class == event.route_class)
    }

    fn handle(&self, event: Arc<BusEvent>) {
        let mut guard = self.inner.lock().expect("recording sink mutex poisoned");
        guard.push(event);
        self.signal.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::id::FixedSnowflakeIdProvider;
    use crate::shared::event_bus::{
        AguiEventSink, BusEventDraft, BusIngressLane, BusOutcome, BusPriority, BusRequestRef,
        BusRuntimeStatsSnapshot, EventBusConfig, EventBusRuntimeBuilder, PublishOutcome,
    };

    #[test]
    fn event_bus_rejects_secret_like_payload_fields() {
        let config = EventBusConfig::default();
        let id_generator = Arc::new(FixedSnowflakeIdProvider::new(100));
        let handle = EventBusRuntimeBuilder::new(config, id_generator)
            .without_workers_for_testing()
            .build();

        let outcome = handle.publish(
            BusEventDraft::audit("user.password_reset", "user_service")
                .subject("user", "USR-001")
                .payload_field("temporaryPassword", "TronTmp123!"),
        );

        assert_eq!(
            outcome,
            PublishOutcome::RejectedInvalid { field: "attr_key" }
        );
    }

    #[test]
    fn event_bus_rejects_oversize_payload_fields() {
        let config = EventBusConfig {
            max_payload_bytes: 16,
            ..EventBusConfig::default()
        };
        let id_generator = Arc::new(FixedSnowflakeIdProvider::new(100));
        let handle = EventBusRuntimeBuilder::new(config, id_generator)
            .without_workers_for_testing()
            .build();

        let outcome = handle.publish(
            BusEventDraft::observe("agent.progress", "agent_runtime")
                .payload_field("status", "01234567890123456789"),
        );

        assert_eq!(
            outcome,
            PublishOutcome::RejectedOversize {
                field: "payload",
                actual: "status".len() + "01234567890123456789".len(),
                limit: 16,
            }
        );
    }

    #[test]
    fn event_bus_keeps_critical_capacity_when_standard_lane_is_full() {
        let config = EventBusConfig {
            critical_ingress_capacity: 1,
            standard_ingress_capacity: 1,
            ..EventBusConfig::default()
        };
        let id_generator = Arc::new(FixedSnowflakeIdProvider::new(1));
        let handle = EventBusRuntimeBuilder::new(config, id_generator)
            .without_workers_for_testing()
            .build();

        let first_standard = handle.publish(BusEventDraft::agui("agui.run.progress", "agent_runtime"));
        let second_standard = handle.publish(BusEventDraft::agui("agui.run.progress", "agent_runtime"));
        let critical = handle.publish(
            BusEventDraft::audit("user.created", "user_service").priority(BusPriority::Critical),
        );

        assert!(matches!(first_standard, PublishOutcome::Accepted { .. }));
        assert_eq!(
            second_standard,
            PublishOutcome::DroppedIngressFull {
                lane: BusIngressLane::Standard
            }
        );
        assert!(matches!(critical, PublishOutcome::Accepted { .. }));
    }

    #[test]
    fn event_bus_dispatches_to_registered_sinks() {
        let config = EventBusConfig::default();
        let id_generator = Arc::new(FixedSnowflakeIdProvider::new(10));
        let recording_sink = Arc::new(RecordingEventSink::new("recording"));
        let handle = EventBusRuntimeBuilder::new(config, id_generator)
            .register_sink(recording_sink.clone())
            .build();

        let outcome = handle.publish(
            BusEventDraft::audit("workspace.created", "workspace_service")
                .subject("workspace", "WS-001")
                .summary("Workspace created"),
        );

        assert!(matches!(outcome, PublishOutcome::Accepted { .. }));
        assert!(recording_sink.wait_for_len(1, Duration::from_millis(300)));
        let events = recording_sink.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "workspace.created");
    }

    #[test]
    fn event_bus_disabled_handle_is_safe_noop() {
        let handle = super::super::EventBusHandle::disabled();
        let outcome = handle.publish(BusEventDraft::observe("noop", "test"));
        assert_eq!(outcome, PublishOutcome::Disabled);
        assert_eq!(
            handle.stats_snapshot(),
            BusRuntimeStatsSnapshot::default()
        );
    }

    #[test]
    fn agui_sink_replay_is_bounded_and_evicts_idle_streams() {
        let config = EventBusConfig {
            agui_replay_events_per_stream: 2,
            agui_replay_bytes_per_stream: 512,
            agui_global_replay_bytes: 512,
            agui_stream_idle_ttl_seconds: 0,
            ..EventBusConfig::default()
        };
        let agui_sink = AguiEventSink::new(&config);
        let generator = FixedSnowflakeIdProvider::new(1);
        agui_sink.activate_stream("run-1");

        let event_1 = BusEvent::from_draft(
            BusEventDraft::agui("agui.run.progress", "agent_runtime")
                .outcome(BusOutcome::Progress)
                .request(BusRequestRef::with_stream_key("run-1"))
                .payload_field("step", "prepare"),
            &generator,
            &config,
        )
        .expect("event 1 should validate");
        let event_2 = BusEvent::from_draft(
            BusEventDraft::agui("agui.run.progress", "agent_runtime")
                .outcome(BusOutcome::Progress)
                .request(BusRequestRef::with_stream_key("run-1"))
                .payload_field("step", "execute"),
            &generator,
            &config,
        )
        .expect("event 2 should validate");
        let event_3 = BusEvent::from_draft(
            BusEventDraft::agui("agui.run.progress", "agent_runtime")
                .outcome(BusOutcome::Completed)
                .request(BusRequestRef::with_stream_key("run-1"))
                .payload_field("step", "finish"),
            &generator,
            &config,
        )
        .expect("event 3 should validate");

        agui_sink.handle(Arc::new(event_1));
        agui_sink.handle(Arc::new(event_2));
        agui_sink.handle(Arc::new(event_3));

        let replay = agui_sink.replay("run-1");
        assert_eq!(replay.len(), 2);
        assert_eq!(replay[0].payload_fields[0].value, "execute");
        assert_eq!(replay[1].payload_fields[0].value, "finish");

        agui_sink.deactivate_stream("run-1");
        let snapshot = agui_sink.snapshot();
        assert_eq!(snapshot.active_streams, 0);
        assert_eq!(snapshot.total_streams, 0);
        assert!(snapshot.replay_evicted >= 1);
    }
}
