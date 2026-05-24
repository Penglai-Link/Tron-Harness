use std::{
    borrow::Cow,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    domain::auth::AuthenticatedUser,
    shared::id::{IdGenerationError, IdGenerator, SnowflakeId},
};

use super::config::EventBusConfig;

const FORBIDDEN_FIELD_TOKENS: [&str; 13] = [
    "password",
    "secret",
    "token",
    "cookie",
    "authorization",
    "request_body",
    "response_body",
    "prompt",
    "completion",
    "plaintext",
    "oauth_code",
    "access_token",
    "refresh_token",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusRouteClass {
    Audit,
    Observe,
    Agui,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusPriority {
    Critical,
    Standard,
    Debug,
}

impl BusPriority {
    pub fn ingress_lane(self) -> BusIngressLane {
        match self {
            Self::Critical => BusIngressLane::Critical,
            Self::Standard | Self::Debug => BusIngressLane::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusOutcome {
    Success,
    Failure,
    Rejected,
    Started,
    Progress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusIngressLane {
    Critical,
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusActorRef {
    pub user_code: String,
    pub username: String,
}

impl From<&AuthenticatedUser> for BusActorRef {
    fn from(value: &AuthenticatedUser) -> Self {
        Self {
            user_code: value.user_code.clone(),
            username: value.username.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusSubjectRef {
    pub subject_type: String,
    pub subject_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusRequestRef {
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub route: Option<String>,
    pub session_code: Option<String>,
    pub stream_key: Option<String>,
}

impl BusRequestRef {
    pub fn with_stream_key(stream_key: impl Into<String>) -> Self {
        Self {
            request_id: None,
            trace_id: None,
            route: None,
            session_code: None,
            stream_key: Some(stream_key.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusEventAttr {
    pub key: String,
    pub value: String,
}

impl BusEventAttr {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BusEventDraft {
    event_name: Cow<'static, str>,
    route_class: BusRouteClass,
    priority: BusPriority,
    producer: Cow<'static, str>,
    outcome: BusOutcome,
    subject: Option<BusSubjectRef>,
    actor: Option<BusActorRef>,
    request: Option<BusRequestRef>,
    summary: Option<String>,
    attrs: Vec<BusEventAttr>,
    payload_fields: Vec<BusEventAttr>,
    occurred_at_ms: Option<i64>,
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

    pub fn route_class(&self) -> BusRouteClass {
        self.route_class
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishOutcome {
    Disabled,
    Accepted {
        event_id: SnowflakeId,
    },
    RejectedInvalid {
        field: &'static str,
    },
    RejectedOversize {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    DroppedIngressFull {
        lane: BusIngressLane,
    },
    DroppedIngressDisconnected {
        lane: BusIngressLane,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusEventValidationError {
    MissingField(&'static str),
    Oversize {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    ForbiddenField(&'static str),
    InvalidField(&'static str),
    IdGenerationFailed(IdGenerationError),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BusRuntimeStatsSnapshot {
    pub accepted: usize,
    pub rejected_invalid: usize,
    pub rejected_oversize: usize,
    pub dropped_critical_full: usize,
    pub dropped_standard_full: usize,
    pub dropped_disconnected: usize,
    pub sink_filtered: usize,
    pub sink_enqueued: usize,
    pub sink_dropped: usize,
}

fn bounded_required(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<String, BusEventValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BusEventValidationError::MissingField(field));
    }
    if trimmed.len() > max_bytes {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: trimmed.len(),
            limit: max_bytes,
        });
    }
    Ok(trimmed.to_string())
}

fn bounded_optional(
    field: &'static str,
    value: Option<&str>,
    max_bytes: usize,
) -> Result<Option<String>, BusEventValidationError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > max_bytes {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: value.len(),
            limit: max_bytes,
        });
    }
    Ok(Some(value.to_string()))
}

fn validate_subject(
    subject: BusSubjectRef,
    config: &EventBusConfig,
) -> Result<BusSubjectRef, BusEventValidationError> {
    Ok(BusSubjectRef {
        subject_type: bounded_required(
            "subject_type",
            &subject.subject_type,
            config.max_subject_type_bytes,
        )?,
        subject_code: bounded_optional(
            "subject_code",
            subject.subject_code.as_deref(),
            config.max_subject_code_bytes,
        )?,
    })
}

fn validate_actor(
    actor: BusActorRef,
    config: &EventBusConfig,
) -> Result<BusActorRef, BusEventValidationError> {
    Ok(BusActorRef {
        user_code: bounded_required(
            "actor_user_code",
            &actor.user_code,
            config.max_actor_user_code_bytes,
        )?,
        username: bounded_required(
            "actor_username",
            &actor.username,
            config.max_actor_username_bytes,
        )?,
    })
}

fn validate_request(
    request: BusRequestRef,
    config: &EventBusConfig,
) -> Result<BusRequestRef, BusEventValidationError> {
    Ok(BusRequestRef {
        request_id: bounded_optional(
            "request_id",
            request.request_id.as_deref(),
            config.max_request_field_bytes,
        )?,
        trace_id: bounded_optional(
            "trace_id",
            request.trace_id.as_deref(),
            config.max_request_field_bytes,
        )?,
        route: bounded_optional(
            "route",
            request.route.as_deref(),
            config.max_request_field_bytes,
        )?,
        session_code: bounded_optional(
            "session_code",
            request.session_code.as_deref(),
            config.max_request_field_bytes,
        )?,
        stream_key: bounded_optional(
            "stream_key",
            request.stream_key.as_deref(),
            config.max_request_field_bytes,
        )?,
    })
}

fn validate_attr_list(
    field: &'static str,
    attrs: Vec<BusEventAttr>,
    max_entries: usize,
    max_key_bytes: usize,
    max_value_bytes: usize,
    max_total_bytes: Option<usize>,
) -> Result<Vec<BusEventAttr>, BusEventValidationError> {
    if attrs.len() > max_entries {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: attrs.len(),
            limit: max_entries,
        });
    }
    let mut total_bytes = 0;
    let mut validated = Vec::with_capacity(attrs.len());
    for attr in attrs {
        let key = bounded_required("attr_key", &attr.key, max_key_bytes)?;
        ensure_safe_key(&key)?;
        let value = bounded_required("attr_value", &attr.value, max_value_bytes)?;
        total_bytes += key.len() + value.len();
        validated.push(BusEventAttr { key, value });
    }
    if let Some(limit) = max_total_bytes
        && total_bytes > limit
    {
        return Err(BusEventValidationError::Oversize {
            field,
            actual: total_bytes,
            limit,
        });
    }
    Ok(validated)
}

fn ensure_safe_key(key: &str) -> Result<(), BusEventValidationError> {
    let normalized = key
        .chars()
        .map(|ch| {
            if ch == '-' {
                '_'
            } else {
                ch.to_ascii_lowercase()
            }
        })
        .collect::<String>();
    if FORBIDDEN_FIELD_TOKENS
        .iter()
        .any(|token| normalized.contains(token))
    {
        return Err(BusEventValidationError::ForbiddenField("attr_key"));
    }
    Ok(())
}

fn attr_bytes(attrs: &[BusEventAttr]) -> usize {
    attrs
        .iter()
        .map(|attr| attr.key.len() + attr.value.len())
        .sum()
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_millis() as i64
}
