use crate::shared::{
    event_bus::config::EventBusConfig,
    id::{IdGenerator, SnowflakeId},
};

use super::validation::{
    attr_bytes, bounded_optional, bounded_required, now_millis, validate_actor, validate_attr_list,
    validate_request, validate_subject,
};
use super::{
    BusActorRef, BusEventAttr, BusEventDraft, BusEventValidationError, BusOutcome, BusPriority,
    BusRequestRef, BusRouteClass, BusSubjectRef,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusEvent {
    pub event_id: SnowflakeId,
    pub event_name: String,
    pub route_class: BusRouteClass,
    pub priority: BusPriority,
    pub producer: String,
    pub occurred_at_ms: i64,
    pub outcome: BusOutcome,
    pub subject: Option<BusSubjectRef>,
    pub actor: Option<BusActorRef>,
    pub request: Option<BusRequestRef>,
    pub summary: Option<String>,
    pub attrs: Vec<BusEventAttr>,
    pub payload_fields: Vec<BusEventAttr>,
}

impl BusEvent {
    pub fn from_draft(
        draft: BusEventDraft,
        generator: &dyn IdGenerator,
        config: &EventBusConfig,
    ) -> Result<Self, BusEventValidationError> {
        let event_name = bounded_required(
            "event_name",
            draft.event_name.as_ref(),
            config.max_name_bytes,
        )?;
        let producer = bounded_required(
            "producer",
            draft.producer.as_ref(),
            config.max_producer_bytes,
        )?;
        let summary = bounded_optional(
            "summary",
            draft.summary.as_deref(),
            config.max_summary_bytes,
        )?;
        let subject = draft
            .subject
            .map(|subject| validate_subject(subject, config))
            .transpose()?;
        let actor = draft
            .actor
            .map(|actor| validate_actor(actor, config))
            .transpose()?;
        let request = draft
            .request
            .map(|request| validate_request(request, config))
            .transpose()?;
        let attrs = validate_attr_list(
            "attrs",
            draft.attrs,
            config.max_attrs,
            config.max_attr_key_bytes,
            config.max_attr_value_bytes,
            None,
        )?;
        let payload_fields = validate_attr_list(
            "payload",
            draft.payload_fields,
            config.max_payload_fields,
            config.max_attr_key_bytes,
            config.max_attr_value_bytes,
            Some(config.max_payload_bytes),
        )?;
        let event_id = generator
            .next_id()
            .map_err(BusEventValidationError::IdGenerationFailed)?;

        Ok(Self {
            event_id,
            event_name,
            route_class: draft.route_class,
            priority: draft.priority,
            producer,
            occurred_at_ms: draft.occurred_at_ms.unwrap_or_else(now_millis),
            outcome: draft.outcome,
            subject,
            actor,
            request,
            summary,
            attrs,
            payload_fields,
        })
    }

    pub fn stream_key(&self) -> Option<&str> {
        self.request
            .as_ref()
            .and_then(|request| request.stream_key.as_deref())
    }

    pub fn approx_size_bytes(&self) -> usize {
        let mut total = self.event_name.len() + self.producer.len() + 64;
        if let Some(subject) = &self.subject {
            total += subject.subject_type.len();
            total += subject
                .subject_code
                .as_ref()
                .map_or(0, std::string::String::len);
        }
        if let Some(actor) = &self.actor {
            total += actor.user_code.len() + actor.username.len();
        }
        if let Some(request) = &self.request {
            total += request
                .request_id
                .as_ref()
                .map_or(0, std::string::String::len);
            total += request
                .trace_id
                .as_ref()
                .map_or(0, std::string::String::len);
            total += request.route.as_ref().map_or(0, std::string::String::len);
            total += request
                .session_code
                .as_ref()
                .map_or(0, std::string::String::len);
            total += request
                .stream_key
                .as_ref()
                .map_or(0, std::string::String::len);
        }
        total += self.summary.as_ref().map_or(0, std::string::String::len);
        total += attr_bytes(&self.attrs);
        total += attr_bytes(&self.payload_fields);
        total
    }
}
