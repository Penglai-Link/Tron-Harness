//! Application log query contract. Records are read from daily files, never cached.
use super::ObservabilityError;
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationLog {
    pub log_id: String,
    pub timestamp: i64,
    pub level: String,
    pub target: String,
    pub message: String,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub attributes: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Default)]
pub struct LogQuery {
    pub workspace_code: Option<String>,
    pub date: Option<String>,
    pub time_zone: Option<String>,
    pub keyword: Option<String>,
    pub level: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub current: usize,
    pub size: usize,
    pub before_log_id: Option<String>,
    pub after_log_id: Option<String>,
}
impl LogQuery {
    pub fn normalize(&mut self) -> Result<(), ObservabilityError> {
        let clean = |v: Option<String>| v.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        self.workspace_code = clean(self.workspace_code.take());
        self.keyword = clean(self.keyword.take()).map(|s| s.to_lowercase());
        self.level = clean(self.level.take());
        self.before_log_id = clean(self.before_log_id.take());
        self.after_log_id = clean(self.after_log_id.take());
        if self.after_log_id.as_ref().is_some_and(|id| log_position(id).is_none()) || (self.before_log_id.is_some() && self.after_log_id.is_some()) {
            return Err(ObservabilityError::InvalidLogQuery);
        }
        if self.before_log_id.as_ref().is_some_and(|id| log_position(id).is_none()) {
            return Err(ObservabilityError::InvalidLogQuery);
        }
        self.trace_id = clean(self.trace_id.take()).map(|s| s.to_ascii_lowercase());
        self.span_id = clean(self.span_id.take()).map(|s| s.to_ascii_lowercase());
        let valid_id = |id: &str, len| {
            id.len() == len
                && id.bytes().all(|b| b.is_ascii_hexdigit())
                && id.bytes().any(|b| b != b'0')
        };
        if self.current == 0
            || self.current > 1_000_000
            || !(1..=100).contains(&self.size)
            || self.keyword.as_ref().is_some_and(|s| s.len() > 256)
            || self.level.as_ref().is_some_and(|s| {
                !matches!(s.as_str(), "Trace" | "Debug" | "Info" | "Warn" | "Error")
            })
            || self.trace_id.as_ref().is_some_and(|s| !valid_id(s, 32))
            || self.span_id.as_ref().is_some_and(|s| !valid_id(s, 16))
        {
            return Err(ObservabilityError::InvalidLogQuery);
        }
        Ok(())
    }
    pub fn matches(&self, r: &ApplicationLog) -> bool {
        self.workspace_code.as_ref().is_none_or(|code| ["tron.workspace.code", "workspace_code", "workspaceCode"].iter().any(|key| r.attributes.get(*key) == Some(code)))
            && self.level.as_ref().is_none_or(|s| r.level == *s)
            && self
                .trace_id
                .as_ref()
                .is_none_or(|s| r.trace_id.as_ref() == Some(s))
            && self
                .span_id
                .as_ref()
                .is_none_or(|s| r.span_id.as_ref() == Some(s))
            && self.keyword.as_ref().is_none_or(|s| {
                r.message.to_lowercase().contains(s)
                    || r.target.to_lowercase().contains(s)
                    || r.attributes
                        .iter()
                        .any(|(k, v)| k.to_lowercase().contains(s) || v.to_lowercase().contains(s))
            })
    }
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationLogPage {
    pub date: String,
    pub time_zone: String,
    pub current: usize,
    pub size: usize,
    pub total: usize,
    pub records: Vec<ApplicationLog>,
}

/// File order plus numeric line number is stable while new records are appended.
pub fn log_position(id: &str) -> Option<(&str, u64)> {
    let (file, line) = id.rsplit_once(':')?;
    let date = file.strip_prefix("tron-backend.")?.strip_suffix(".jsonl")?;
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    if parsed.format("%Y-%m-%d").to_string() != date { return None; }
    let line: u64 = line.parse().ok()?;
    (line > 0).then_some((file, line))
}
