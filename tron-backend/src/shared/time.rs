use chrono::{SecondsFormat, TimeZone, Utc};

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn current_timestamp_seconds() -> i64 {
    Utc::now().timestamp()
}

pub fn timestamp_to_millis(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.timestamp_millis())
        .unwrap_or_else(|_| Utc::now().timestamp_millis())
}

pub fn timestamp_from_millis(value: i64) -> String {
    Utc.timestamp_millis_opt(value)
        .single()
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn option_timestamp_to_millis(value: Option<&str>) -> Option<i64> {
    value.map(timestamp_to_millis)
}

pub fn option_timestamp_from_millis(value: Option<i64>) -> Option<String> {
    value.map(timestamp_from_millis)
}
