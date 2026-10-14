//! List queries read precomputed counters; full span payloads belong to detail reads.
use super::*;

pub(super) fn sorts(q: &TraceQuery) -> Vec<TraceSort> {
    if q.sorts.is_empty() {
        vec![TraceSort {
            field: q.sort_by,
            direction: q.sort_direction,
        }]
    } else {
        q.sorts.clone()
    }
}

pub(super) fn needs_metrics(sorts: &[TraceSort]) -> bool {
    sorts.iter().any(|sort| {
        matches!(
            sort.field,
            TraceSortField::Spans | TraceSortField::Tokens | TraceSortField::Errors
        )
    })
}

/// Only allowlisted column names and enum directions enter SQL; filters stay bound.
pub(super) fn order(sorts: &[TraceSort]) -> String {
    let mut fields = sorts
        .iter()
        .map(|sort| {
            let field = match sort.field {
                TraceSortField::StartedAt => "r.started_at",
                TraceSortField::Duration => "r.duration_ms",
                TraceSortField::Spans => "COALESCE(m.span_count,0)",
                TraceSortField::Tokens => {
                    "(COALESCE(m.input_tokens,0) + COALESCE(m.output_tokens,0))"
                }
                TraceSortField::Errors => "COALESCE(m.error_count,0)",
            };
            let direction = match sort.direction {
                TraceSortDirection::Asc => "ASC",
                TraceSortDirection::Desc => "DESC",
            };
            format!("{field} {direction}")
        })
        .collect::<Vec<_>>();
    if !sorts
        .iter()
        .any(|sort| matches!(sort.field, TraceSortField::StartedAt))
    {
        fields.push("r.started_at DESC".into());
    }
    fields.extend(["r.trace_id ASC".into(), "r.root_span_id ASC".into()]);
    fields.join(",")
}

pub(super) const METRIC_COLUMNS: &str = "COALESCE(m.span_count,0),COALESCE(m.error_count,0),COALESCE(m.model_count,0),COALESCE(m.tool_count,0),COALESCE(m.input_tokens,0),COALESCE(m.cached_input_tokens,0),COALESCE(m.output_tokens,0)";

pub(super) struct Metrics {
    span_count: usize,
    error_count: usize,
    model_count: usize,
    tool_count: usize,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
}
impl Metrics {
    pub(super) fn read(row: &rusqlite::Row<'_>, start: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            span_count: row.get(start)?,
            error_count: row.get(start + 1)?,
            model_count: row.get(start + 2)?,
            tool_count: row.get(start + 3)?,
            input_tokens: row.get(start + 4)?,
            cached_input_tokens: row.get(start + 5)?,
            output_tokens: row.get(start + 6)?,
        })
    }
    pub(super) fn apply(self, summary: &mut TraceSummary) {
        summary.span_count = self.span_count;
        summary.error_count = self.error_count;
        summary.model_count = self.model_count;
        summary.tool_count = self.tool_count;
        summary.input_tokens = self.input_tokens;
        summary.cached_input_tokens = self.cached_input_tokens;
        summary.output_tokens = self.output_tokens;
    }
}
