use super::*;
use crate::shared::event_bus::{BusEvent, BusRequestRef};
use crate::shared::id::FixedSnowflakeIdProvider;
use std::sync::{Arc, Mutex};
struct Feedback(Mutex<Vec<String>>);
impl EventSink for Feedback {
    fn name(&self) -> &'static str {
        "feedback_test"
    }
    fn accepts(&self, _: &BusEvent) -> bool {
        true
    }
    fn handle(&self, _: Arc<BusEvent>) {}
    fn delivery_failed(&self, _: &BusEvent, detail: &str) {
        self.0.lock().unwrap().push(detail.into());
    }
}
#[test]
fn bounded_ingress_failure_reports_to_the_affected_sink() {
    let feedback = Arc::new(Feedback(Mutex::new(Vec::new())));
    let bus = EventBusRuntimeBuilder::new(
        EventBusConfig {
            standard_ingress_capacity: 1,
            ..EventBusConfig::default()
        },
        Arc::new(FixedSnowflakeIdProvider::new(987)),
    )
    .register_sink(feedback.clone())
    .without_workers_for_testing()
    .build();
    let draft = || {
        BusEventDraft::agui("agent.context.assembled", "test")
            .request(BusRequestRef::with_stream_key("SESSION"))
    };
    assert!(matches!(
        bus.publish(draft()),
        PublishOutcome::Accepted { .. }
    ));
    assert!(matches!(
        bus.publish(draft()),
        PublishOutcome::DroppedIngressFull { .. }
    ));
    assert_eq!(feedback.0.lock().unwrap().len(), 1);
}
