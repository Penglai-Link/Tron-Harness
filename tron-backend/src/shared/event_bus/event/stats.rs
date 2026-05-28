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
