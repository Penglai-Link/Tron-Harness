use serde::Deserialize;

use crate::shared::event_bus::EventBusConfig;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct EventBusFileConfig {
    enabled: Option<bool>,
    critical_ingress_capacity: Option<usize>,
    standard_ingress_capacity: Option<usize>,
    default_sink_capacity: Option<usize>,
    max_name_bytes: Option<usize>,
    max_producer_bytes: Option<usize>,
    max_summary_bytes: Option<usize>,
    max_attrs: Option<usize>,
    max_attr_key_bytes: Option<usize>,
    max_attr_value_bytes: Option<usize>,
    max_payload_fields: Option<usize>,
    max_payload_bytes: Option<usize>,
    max_subject_type_bytes: Option<usize>,
    max_subject_code_bytes: Option<usize>,
    max_actor_user_code_bytes: Option<usize>,
    max_actor_username_bytes: Option<usize>,
    max_request_field_bytes: Option<usize>,
    agui_replay_events_per_stream: Option<usize>,
    agui_replay_bytes_per_stream: Option<usize>,
    agui_global_replay_bytes: Option<usize>,
    agui_stream_idle_ttl_seconds: Option<u64>,
}

impl EventBusConfig {
    pub(super) fn apply_overrides(&mut self, overrides: EventBusFileConfig) {
        if let Some(value) = overrides.enabled {
            self.enabled = value;
        }
        if let Some(value) = overrides.critical_ingress_capacity {
            self.critical_ingress_capacity = value;
        }
        if let Some(value) = overrides.standard_ingress_capacity {
            self.standard_ingress_capacity = value;
        }
        if let Some(value) = overrides.default_sink_capacity {
            self.default_sink_capacity = value;
        }
        if let Some(value) = overrides.max_name_bytes {
            self.max_name_bytes = value;
        }
        if let Some(value) = overrides.max_producer_bytes {
            self.max_producer_bytes = value;
        }
        if let Some(value) = overrides.max_summary_bytes {
            self.max_summary_bytes = value;
        }
        if let Some(value) = overrides.max_attrs {
            self.max_attrs = value;
        }
        if let Some(value) = overrides.max_attr_key_bytes {
            self.max_attr_key_bytes = value;
        }
        if let Some(value) = overrides.max_attr_value_bytes {
            self.max_attr_value_bytes = value;
        }
        if let Some(value) = overrides.max_payload_fields {
            self.max_payload_fields = value;
        }
        if let Some(value) = overrides.max_payload_bytes {
            self.max_payload_bytes = value;
        }
        if let Some(value) = overrides.max_subject_type_bytes {
            self.max_subject_type_bytes = value;
        }
        if let Some(value) = overrides.max_subject_code_bytes {
            self.max_subject_code_bytes = value;
        }
        if let Some(value) = overrides.max_actor_user_code_bytes {
            self.max_actor_user_code_bytes = value;
        }
        if let Some(value) = overrides.max_actor_username_bytes {
            self.max_actor_username_bytes = value;
        }
        if let Some(value) = overrides.max_request_field_bytes {
            self.max_request_field_bytes = value;
        }
        if let Some(value) = overrides.agui_replay_events_per_stream {
            self.agui_replay_events_per_stream = value;
        }
        if let Some(value) = overrides.agui_replay_bytes_per_stream {
            self.agui_replay_bytes_per_stream = value;
        }
        if let Some(value) = overrides.agui_global_replay_bytes {
            self.agui_global_replay_bytes = value;
        }
        if let Some(value) = overrides.agui_stream_idle_ttl_seconds {
            self.agui_stream_idle_ttl_seconds = value;
        }
    }
}
