use crate::domain::auth::AuthenticatedUser;

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
