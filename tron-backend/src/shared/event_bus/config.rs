use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBusConfig {
    pub enabled: bool,
    pub critical_ingress_capacity: usize,
    pub standard_ingress_capacity: usize,
    pub default_sink_capacity: usize,
    pub max_name_bytes: usize,
    pub max_producer_bytes: usize,
    pub max_summary_bytes: usize,
    pub max_attrs: usize,
    pub max_attr_key_bytes: usize,
    pub max_attr_value_bytes: usize,
    pub max_payload_fields: usize,
    pub max_payload_bytes: usize,
    pub max_subject_type_bytes: usize,
    pub max_subject_code_bytes: usize,
    pub max_actor_user_code_bytes: usize,
    pub max_actor_username_bytes: usize,
    pub max_request_field_bytes: usize,
    pub agui_replay_events_per_stream: usize,
    pub agui_replay_bytes_per_stream: usize,
    pub agui_global_replay_bytes: usize,
    pub agui_stream_idle_ttl_seconds: u64,
}

impl EventBusConfig {
    pub fn agui_stream_idle_ttl(&self) -> Duration {
        Duration::from_secs(self.agui_stream_idle_ttl_seconds)
    }
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            critical_ingress_capacity: 128,
            standard_ingress_capacity: 512,
            default_sink_capacity: 256,
            max_name_bytes: 96,
            max_producer_bytes: 48,
            max_summary_bytes: 160,
            max_attrs: 8,
            max_attr_key_bytes: 32,
            max_attr_value_bytes: 8192,
            max_payload_fields: 8,
            max_payload_bytes: 8192,
            max_subject_type_bytes: 32,
            max_subject_code_bytes: 64,
            max_actor_user_code_bytes: 64,
            max_actor_username_bytes: 64,
            max_request_field_bytes: 64,
            agui_replay_events_per_stream: 32,
            agui_replay_bytes_per_stream: 64 * 1024,
            agui_global_replay_bytes: 2 * 1024 * 1024,
            agui_stream_idle_ttl_seconds: 30,
        }
    }
}
