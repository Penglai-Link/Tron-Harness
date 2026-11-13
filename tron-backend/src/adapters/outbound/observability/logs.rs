//! Standard tracing JSON output with library-managed rotation; streaming file queries.
use crate::{
    application::observability::{
        ApplicationLog, ApplicationLogPage, LogQuery, ObservabilityError,
    },
    config::AppConfig,
};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;

/// Correlate every event emitted inside an active OTel span, including model/tool logs.
#[derive(Default)]
struct CorrelatedJson;
impl<S, N> tracing_subscriber::fmt::format::FormatEvent<S, N> for CorrelatedJson
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::format::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        use opentelemetry::trace::TraceContextExt;
        use tracing_subscriber::fmt::format::Writer;
        let mut line = String::new();
        tracing_subscriber::fmt::format()
            .json()
            .flatten_event(true)
            .format_event(ctx, Writer::new(&mut line), event)?;
        let context = opentelemetry::Context::current();
        let span = context.span();
        let ids = span.span_context();
        if !ids.is_valid() {
            return writer.write_str(&line);
        }
        let mut value: Value = serde_json::from_str(&line).map_err(|_| std::fmt::Error)?;
        if let Some(object) = value.as_object_mut() {
            object
                .entry("trace_id")
                .or_insert_with(|| Value::String(ids.trace_id().to_string()));
            object
                .entry("span_id")
                .or_insert_with(|| Value::String(ids.span_id().to_string()));
        }
        writeln!(writer, "{value}")
    }
}

pub fn log_directory(config: &AppConfig) -> PathBuf {
    Path::new(&config.application_config_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("logs")
}
pub fn daily_appender(
    directory: &Path,
    max_files: usize,
) -> Result<RollingFileAppender, tracing_appender::rolling::InitError> {
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("tron-backend")
        .filename_suffix("jsonl")
        .max_log_files(max_files)
        .build(directory)
}
pub fn init_application_logging(
    config: &AppConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let max_files = std::env::var("TRON_BACKEND_LOG_MAX_FILES")
        .ok()
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(30);
    if !(1..=3650).contains(&max_files) {
        return Err("TRON_BACKEND_LOG_MAX_FILES must be between 1 and 3650".into());
    }
    // Direct writes make newly logged records immediately readable. No application log cache.
    let file = daily_appender(&log_directory(config), max_files)?;
    let console = (std::env::var("TRON_BACKEND_LOG_STDERR").as_deref() != Ok("false")).then(|| {
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(false)
    });
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,hyper=warn,reqwest=warn".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .event_format(CorrelatedJson)
                .with_ansi(false)
                .with_writer(file),
        )
        .with(console)
        .try_init()?;
    Ok(())
}

/// A local calendar day can span two UTC files and can be 23 or 25 hours at DST.
fn day_range(q: &LogQuery) -> Result<(String, Tz, i64, i64), ObservabilityError> {
    let invalid = || ObservabilityError::InvalidLogQuery;
    let zone: Tz = q
        .time_zone
        .as_deref()
        .unwrap_or("Asia/Shanghai")
        .parse()
        .map_err(|_| invalid())?;
    let date = q.date.clone().unwrap_or_else(|| {
        Utc::now()
            .with_timezone(&zone)
            .format("%Y-%m-%d")
            .to_string()
    });
    let day = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| invalid())?;
    if day.format("%Y-%m-%d").to_string() != date {
        return Err(invalid());
    }
    let start = |d: NaiveDate| {
        // Some time zones advance clocks at midnight. Use the first valid local instant.
        let midnight = d.and_hms_opt(0, 0, 0).unwrap();
        (0..1440)
            .find_map(|m| {
                zone.from_local_datetime(&(midnight + Duration::minutes(m)))
                    .earliest()
            })
            .ok_or_else(invalid)
    };
    let from = start(day)?.timestamp_millis();
    let to = start(day.succ_opt().ok_or_else(invalid)?)?.timestamp_millis();
    Ok((date, zone, from, to))
}
fn parse_line(line: &str, id: String) -> Option<ApplicationLog> {
    let v: Value = serde_json::from_str(line).ok()?;
    let timestamp = DateTime::parse_from_rfc3339(v.get("timestamp")?.as_str()?)
        .ok()?
        .timestamp_millis();
    let mut fields = BTreeMap::new();
    for scope in v
        .get("spans")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(object) = scope.as_object() {
            fields.extend(object.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }
    if let Some(object) = v.get("span").and_then(Value::as_object) {
        fields.extend(object.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    if let Some(object) = v.get("fields").and_then(Value::as_object) {
        fields.extend(object.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    if let Some(object) = v.as_object() {
        fields.extend(
            object
                .iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.as_str(),
                        "timestamp" | "level" | "target" | "fields" | "span" | "spans"
                    )
                })
                .map(|(k, v)| (k.clone(), v.clone())),
        );
    }
    let text = |v: Value| {
        v.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| v.to_string())
    };
    let message = fields.remove("message").map(text).unwrap_or_default();
    let mut id_field = |snake: &str, camel: &str, len: usize| {
        fields
            .remove(snake)
            .or_else(|| fields.remove(camel))
            .map(text)
            .filter(|s| {
                s.len() == len
                    && s.bytes().all(|b| b.is_ascii_hexdigit())
                    && s.bytes().any(|b| b != b'0')
            })
            .map(|s| s.to_ascii_lowercase())
    };
    let trace_id = id_field("trace_id", "traceId", 32);
    let span_id = id_field("span_id", "spanId", 16);
    let level = match v.get("level")?.as_str()? {
        "ERROR" => "Error",
        "WARN" => "Warn",
        "INFO" => "Info",
        "DEBUG" => "Debug",
        "TRACE" => "Trace",
        _ => return None,
    }
    .into();
    Some(ApplicationLog {
        log_id: id,
        timestamp,
        level,
        target: v
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .into(),
        message,
        trace_id,
        span_id,
        attributes: fields.into_iter().map(|(k, v)| (k, text(v))).collect(),
    })
}
pub fn query_log_files(
    directory: &Path,
    mut q: LogQuery,
) -> Result<ApplicationLogPage, ObservabilityError> {
    q.normalize()?;
    let (date, zone, from, to) = day_range(&q)?;
    let before = q
        .before_log_id
        .as_deref()
        .and_then(crate::application::observability::log_position);
    let after = q
        .after_log_id
        .as_deref()
        .and_then(crate::application::observability::log_position);
    let first = DateTime::from_timestamp_millis(from)
        .ok_or(ObservabilityError::InvalidLogQuery)?
        .date_naive();
    let last = DateTime::from_timestamp_millis(to - 1)
        .ok_or(ObservabilityError::InvalidLogQuery)?
        .date_naive();
    let mut files = Vec::new();
    let mut day = first;
    while day <= last {
        let path = directory.join(format!("tron-backend.{}.jsonl", day.format("%Y-%m-%d")));
        match File::open(&path) {
            Ok(file) => {
                let len = file
                    .metadata()
                    .map_err(|_| ObservabilityError::Storage)?
                    .len();
                files.push((path, file, len));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(ObservabilityError::Storage),
        }
        day = day.succ_opt().ok_or(ObservabilityError::InvalidLogQuery)?;
    }
    // Two streaming passes over a fixed byte snapshot: count, then selected page.
    // Appends during a request appear on the next refresh; no retained records.
    let mut total: usize = 0;
    let scan = |files: &mut Vec<(PathBuf, File, u64)>,
                visitor: &mut dyn FnMut(ApplicationLog)|
     -> Result<(), ObservabilityError> {
        use std::io::{Read, Seek};
        for (path, file, len) in files {
            file.rewind().map_err(|_| ObservabilityError::Storage)?;
            for (index, line) in BufReader::new(file.take(*len)).lines().enumerate() {
                let line = line.map_err(|_| ObservabilityError::Storage)?;
                if let Some(record) = parse_line(
                    &line,
                    format!(
                        "{}:{}",
                        path.file_name().unwrap().to_string_lossy(),
                        index + 1
                    ),
                ) {
                    let position = crate::application::observability::log_position(&record.log_id);
                    if before.is_none_or(|cursor| position.is_some_and(|p| p < cursor))
                        && after.is_none_or(|cursor| position.is_some_and(|p| p > cursor))
                        && record.timestamp >= from
                        && record.timestamp < to
                        && q.matches(&record)
                    {
                        visitor(record);
                    }
                }
            }
        }
        Ok(())
    };
    scan(&mut files, &mut |_| total += 1)?;
    let (begin, end) = if after.is_some() {
        let begin = ((q.current - 1) * q.size).min(total);
        (begin, (begin + q.size).min(total))
    } else {
        let end = total.saturating_sub((q.current - 1) * q.size);
        (end.saturating_sub(q.size), end)
    };
    let mut records = Vec::with_capacity(q.size);
    let mut index = 0;
    scan(&mut files, &mut |r| {
        if index >= begin && index < end {
            records.push(r);
        }
        index += 1;
    })?;
    records.reverse();
    Ok(ApplicationLogPage {
        date,
        time_zone: zone.to_string(),
        current: q.current,
        size: q.size,
        total,
        records,
    })
}
