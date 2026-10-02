use std::borrow::Cow;

use super::{
    BusActorRef, BusEventAttr, BusOutcome, BusPriority, BusRequestRef, BusRouteClass, BusSubjectRef,
};

#[derive(Debug, Clone)]
pub struct BusEventDraft {
    pub(super) event_name: Cow<'static, str>,
    pub(super) route_class: BusRouteClass,
    pub(super) priority: BusPriority,
    pub(super) producer: Cow<'static, str>,
    pub(super) outcome: BusOutcome,
    pub(super) subject: Option<BusSubjectRef>,
    pub(super) actor: Option<BusActorRef>,
    pub(super) request: Option<BusRequestRef>,
    pub(super) summary: Option<String>,
    pub(super) attrs: Vec<BusEventAttr>,
    pub(super) payload_fields: Vec<BusEventAttr>,
    pub(super) occurred_at_ms: Option<i64>,
}

impl BusEventDraft {
    pub fn new(
        event_name: impl Into<Cow<'static, str>>,
        route_class: BusRouteClass,
        producer: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            event_name: event_name.into(),
            route_class,
            priority: BusPriority::Standard,
            producer: producer.into(),
            outcome: BusOutcome::Success,
            subject: None,
            actor: None,
            request: None,
            summary: None,
            attrs: Vec::new(),
            payload_fields: Vec::new(),
            occurred_at_ms: None,
        }
    }

    pub fn audit(
        event_name: impl Into<Cow<'static, str>>,
        producer: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(event_name, BusRouteClass::Audit, producer)
    }

    pub fn observe(
        event_name: impl Into<Cow<'static, str>>,
        producer: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(event_name, BusRouteClass::Observe, producer)
    }

    pub fn agui(
        event_name: impl Into<Cow<'static, str>>,
        producer: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(event_name, BusRouteClass::Agui, producer)
    }

    pub fn priority(mut self, priority: BusPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn outcome(mut self, outcome: BusOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn subject(
        mut self,
        subject_type: impl Into<String>,
        subject_code: impl Into<String>,
    ) -> Self {
        self.subject = Some(BusSubjectRef {
            subject_type: subject_type.into(),
            subject_code: Some(subject_code.into()),
        });
        self
    }

    pub fn subject_type_only(mut self, subject_type: impl Into<String>) -> Self {
        self.subject = Some(BusSubjectRef {
            subject_type: subject_type.into(),
            subject_code: None,
        });
        self
    }

    pub fn actor(mut self, actor: impl Into<BusActorRef>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn request(mut self, request: BusRequestRef) -> Self {
        self.request = Some(request);
        self
    }

    pub fn stream_key(mut self, stream_key: impl Into<String>) -> Self {
        let request = self.request.get_or_insert(BusRequestRef {
            request_id: None,
            trace_id: None,
            route: None,
            session_code: None,
            stream_key: None,
        });
        request.stream_key = Some(stream_key.into());
        self
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.push(BusEventAttr::new(key, value));
        self
    }

    pub fn payload_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.payload_fields.push(BusEventAttr::new(key, value));
        self
    }

    pub fn occurred_at_ms(mut self, occurred_at_ms: i64) -> Self {
        self.occurred_at_ms = Some(occurred_at_ms);
        self
    }

    pub fn stream_key_ref(&self) -> Option<&str> {
        self.request
            .as_ref()
            .and_then(|request| request.stream_key.as_deref())
    }

    pub fn route_class(&self) -> BusRouteClass {
        self.route_class
    }
}
