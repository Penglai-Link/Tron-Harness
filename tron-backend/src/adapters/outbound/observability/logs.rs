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
    fn format_event(&self, ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>, event: &tracing::Event<'_>) -> std::fmt::Result {
        use opentelemetry::trace::TraceContextExt;
        use tracing_subscriber::fmt::format::Writer;
        let mut line = String::new();
        tracing_subscriber::fmt::format().json().flatten_event(true)
            .format_event(ctx, Writer::new(&mut line), event)?;
        let context = opentelemetry::Context::current();
        let span = context.span();
        let ids = span.span_context();
        if !ids.is_valid() { return writer.write_str(&line); }
        let mut value: Value = serde_json::from_str(&line).map_err(|_| std::fmt::Error)?;
        if let Some(object) = value.as_object_mut() {
            object.entry("trace_id").or_insert_with(|| Value::String(ids.trace_id().to_string()));
            object.entry("span_id").or_insert_with(|| Value::String(ids.span_id().to_string()));
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
    let before = q.before_log_id.as_deref().and_then(crate::application::observability::log_position);
    let after = q.after_log_id.as_deref().and_then(crate::application::observability::log_position);
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
                        && record.timestamp >= from && record.timestamp < to && q.matches(&record) {
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
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn application_events_automatically_include_the_active_otel_identifiers() {
        use opentelemetry::{Context, trace::{TraceContextExt, SpanContext, TraceId, SpanId, TraceFlags, TraceState}};
        let dir = tempfile::tempdir().unwrap();
        let context = Context::new().with_remote_span_context(SpanContext::new(
            TraceId::from_hex("11111111111111111111111111111111").unwrap(),
            SpanId::from_hex("2222222222222222").unwrap(), TraceFlags::SAMPLED, true, TraceState::default()));
        tracing::subscriber::with_default(tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer().json().event_format(CorrelatedJson)
                .with_writer(daily_appender(dir.path(), 30).unwrap())), || {
            let _context = context.attach();
            tracing::info!("plain model or tool event");
        });
        let page = query_log_files(dir.path(), LogQuery {
            current: 1, size: 15, trace_id: Some("11111111111111111111111111111111".into()), ..Default::default()
        }).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.records[0].span_id.as_deref(), Some("2222222222222222"));
    }
    fn query(date: &str, zone: &str) -> LogQuery {
        LogQuery {
            date: Some(date.into()),
            time_zone: Some(zone.into()),
            current: 1,
            size: 15,
            ..Default::default()
        }
    }
    fn write_record(directory: &Path, time: &str, message: &str) {
        let date = &time[..10];
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join(format!("tron-backend.{date}.jsonl")))
            .unwrap();
        let mut writer = std::io::BufWriter::new(file);
        writeln!(writer,"{}",serde_json::json!({"timestamp":time,"level":"INFO","target":"test","message":message,"trace_id":"a".repeat(32),"span_id":"b".repeat(16)})).unwrap();
    }
    #[test]
    fn workspace_filter_keeps_platform_logs_in_all_scope_and_scoped_logs_in_their_workspace() {
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), "2026-10-11T01:00:00Z", "platform");
        let file = std::fs::OpenOptions::new().append(true).open(dir.path().join("tron-backend.2026-10-11.jsonl")).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        for workspace in ["alpha", "beta"] {
            writeln!(writer, "{}", serde_json::json!({"timestamp":"2026-10-11T01:00:01Z", "level":"INFO", "target":"test", "message": workspace, "tron.workspace.code":workspace})).unwrap();
        }
        writer.flush().unwrap();
        let mut q = query("2026-10-11", "UTC");
        assert_eq!(query_log_files(dir.path(), q.clone()).unwrap().total, 3);
        q.workspace_code = Some("alpha".into());
        let page = query_log_files(dir.path(), q).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.records[0].message, "alpha");
    }
    #[test]
    fn cursor_pagination_stays_stable_after_appends_and_uses_numeric_lines() {
        let dir = tempfile::tempdir().unwrap();
        for line in 1..=12 {
            write_record(dir.path(), "2026-10-11T01:00:00Z", &format!("line-{line}"));
        }
        let mut q = query("2026-10-11", "UTC");
        q.size = 3;
        let first = query_log_files(dir.path(), q.clone()).unwrap();
        assert_eq!(first.records.iter().map(|r| r.message.as_str()).collect::<Vec<_>>(), ["line-12", "line-11", "line-10"]);
        q.before_log_id = Some(first.records.last().unwrap().log_id.clone());
        write_record(dir.path(), "2026-10-11T01:00:01Z", "new-after-query");
        let older = query_log_files(dir.path(), q.clone()).unwrap();
        assert_eq!(older.total, 9);
        assert_eq!(older.records.iter().map(|r| r.message.as_str()).collect::<Vec<_>>(), ["line-9", "line-8", "line-7"]);
        let mut forward = query("2026-10-11", "UTC");
        forward.size = 3;
        forward.after_log_id = Some("tron-backend.2026-10-11.jsonl:6".into());
        let newer = query_log_files(dir.path(), forward.clone()).unwrap();
        assert_eq!(newer.records.iter().map(|r| r.message.as_str()).collect::<Vec<_>>(), ["line-9", "line-8", "line-7"]);
        forward.after_log_id = Some(newer.records[0].log_id.clone());
        let next = query_log_files(dir.path(), forward.clone()).unwrap();
        assert_eq!(next.records.iter().map(|r| r.message.as_str()).collect::<Vec<_>>(), ["line-12", "line-11", "line-10"]);
        forward.before_log_id = Some(first.records[0].log_id.clone());
        assert!(query_log_files(dir.path(), forward).is_err());
        q.before_log_id = Some("invalid-cursor".into());
        assert!(query_log_files(dir.path(), q).is_err());
    }
    #[test]
    fn reads_user_day_across_utc_files_and_reads_new_appends_without_cache() {
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), "2026-10-10T15:59:59Z", "outside-before");
        write_record(dir.path(), "2026-10-10T16:00:00Z", "start");
        write_record(dir.path(), "2026-10-11T15:59:59Z", "end");
        write_record(dir.path(), "2026-10-11T16:00:00Z", "outside-after");
        let mut q = query("2026-10-11", "Asia/Shanghai");
        q.size = 1;
        let page = query_log_files(dir.path(), q.clone()).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.records[0].message, "end");
        q.current = 2;
        assert_eq!(
            query_log_files(dir.path(), q).unwrap().records[0].message,
            "start"
        );
        write_record(dir.path(), "2026-10-11T12:00:00Z", "new");
        let mut q = query("2026-10-11", "Asia/Shanghai");
        q.keyword = Some("NEW".into());
        q.trace_id = Some("A".repeat(32));
        q.span_id = Some("B".repeat(16));
        let page = query_log_files(dir.path(), q).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.records[0].message, "new");
        assert_eq!(
            query_log_files(dir.path(), query("2026-10-01", "UTC"))
                .unwrap()
                .total,
            0
        );
    }
    #[test]
    fn handles_dst_boundaries_and_rejects_invalid_dates_and_zones() {
        let (_, _, from, to) = day_range(&query("2026-03-08", "America/New_York")).unwrap();
        assert_eq!(to - from, 23 * 3600 * 1000);
        let (_, _, from, to) = day_range(&query("2026-11-01", "America/New_York")).unwrap();
        assert_eq!(to - from, 25 * 3600 * 1000);
        for q in [
            query("../../outside", "UTC"),
            query("2026-02-30", "UTC"),
            query("2026-10-11", "Unknown/Zone"),
        ] {
            assert!(day_range(&q).is_err());
        }
    }
    #[test]
    fn standard_logger_writes_persistent_json_with_span_fields() {
        let dir = tempfile::tempdir().unwrap();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_writer(daily_appender(dir.path(), 30).unwrap()),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("startup");
            let span = tracing::info_span!(
                "run",
                trace_id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                span_id = "bbbbbbbbbbbbbbbb"
            );
            let _guard = span.enter();
            tracing::error!(attempt = 2, "failed");
        });
        // The subscriber is gone; queries reconstruct all records from disk.
        let page = query_log_files(
            dir.path(),
            LogQuery {
                current: 1,
                size: 15,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(
            page.records[0].trace_id.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(page.records[0].span_id.as_deref(), Some("bbbbbbbbbbbbbbbb"));
        assert_eq!(page.records[0].attributes["attempt"], "2");
        assert!(page.records[1].trace_id.is_none());
    }
    #[test]
    fn library_cleans_old_matching_files_and_preserves_unrelated_files() {
        let dir = tempfile::tempdir().unwrap();
        for day in 1..=5 {
            std::fs::write(
                dir.path()
                    .join(format!("tron-backend.2000-01-{day:02}.jsonl")),
                "old",
            )
            .unwrap();
        }
        std::fs::write(dir.path().join("keep.txt"), "unrelated").unwrap();
        drop(daily_appender(dir.path(), 3).unwrap());
        let count = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|f| f.file_name().to_string_lossy().ends_with(".jsonl"))
            .count();
        assert!(count <= 3);
        assert!(dir.path().join("keep.txt").exists());
    }
}
