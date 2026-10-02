use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};

use super::{BusEvent, EventBusConfig};

pub trait EventSink: Send + Sync {
    fn name(&self) -> &'static str;
    fn accepts(&self, event: &BusEvent) -> bool;
    fn handle(&self, event: Arc<BusEvent>);
    /// Nonblocking feedback when an accepted record could not reach this sink.
    fn delivery_failed(&self, _event: &BusEvent, _detail: &str) {}
    /// Rejected drafts never become events; report only their routing key, without payload.
    fn delivery_rejected(&self, _stream_key: &str, _detail: &str) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AguiReplaySnapshot {
    pub active_streams: usize,
    pub total_streams: usize,
    pub total_replay_bytes: usize,
    pub replay_evicted: usize,
}

pub struct AguiEventSink {
    replay_events_per_stream: usize,
    replay_bytes_per_stream: usize,
    global_replay_bytes: usize,
    stream_idle_ttl: std::time::Duration,
    state: Mutex<AguiSinkState>,
}

struct AguiSinkState {
    streams: BTreeMap<String, ReplayStream>,
    total_replay_bytes: usize,
    replay_evicted: usize,
}

struct ReplayStream {
    watcher_count: usize,
    last_touched: Instant,
    total_bytes: usize,
    entries: VecDeque<ReplayEntry>,
}

struct ReplayEntry {
    event: Arc<BusEvent>,
    bytes: usize,
    captured_at: Instant,
}

impl AguiEventSink {
    pub fn new(config: &EventBusConfig) -> Self {
        Self {
            replay_events_per_stream: config.agui_replay_events_per_stream.max(1),
            replay_bytes_per_stream: config.agui_replay_bytes_per_stream.max(1),
            global_replay_bytes: config.agui_global_replay_bytes.max(1),
            stream_idle_ttl: config.agui_stream_idle_ttl(),
            state: Mutex::new(AguiSinkState {
                streams: BTreeMap::new(),
                total_replay_bytes: 0,
                replay_evicted: 0,
            }),
        }
    }

    pub fn activate_stream(&self, stream_key: impl Into<String>) {
        let now = Instant::now();
        let mut state = self.state.lock().expect("agui sink mutex poisoned");
        self.cleanup_idle_locked(&mut state, now);
        let stream = state
            .streams
            .entry(stream_key.into())
            .or_insert_with(|| ReplayStream {
                watcher_count: 0,
                last_touched: now,
                total_bytes: 0,
                entries: VecDeque::new(),
            });
        stream.watcher_count += 1;
        stream.last_touched = now;
    }

    pub fn deactivate_stream(&self, stream_key: &str) {
        let now = Instant::now();
        let mut state = self.state.lock().expect("agui sink mutex poisoned");
        if let Some(stream) = state.streams.get_mut(stream_key) {
            stream.watcher_count = stream.watcher_count.saturating_sub(1);
            stream.last_touched = now;
        }
        self.cleanup_idle_locked(&mut state, now);
    }

    pub fn replay(&self, stream_key: &str) -> Vec<BusEvent> {
        let now = Instant::now();
        let mut state = self.state.lock().expect("agui sink mutex poisoned");
        self.cleanup_idle_locked(&mut state, now);
        state
            .streams
            .get(stream_key)
            .map(|stream| {
                stream
                    .entries
                    .iter()
                    .map(|entry| entry.event.as_ref().clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> AguiReplaySnapshot {
        let now = Instant::now();
        let mut state = self.state.lock().expect("agui sink mutex poisoned");
        self.cleanup_idle_locked(&mut state, now);
        AguiReplaySnapshot {
            active_streams: state
                .streams
                .values()
                .filter(|stream| stream.watcher_count > 0)
                .count(),
            total_streams: state.streams.len(),
            total_replay_bytes: state.total_replay_bytes,
            replay_evicted: state.replay_evicted,
        }
    }

    fn cleanup_idle_locked(&self, state: &mut AguiSinkState, now: Instant) {
        let stale_keys = state
            .streams
            .iter()
            .filter(|(_, stream)| {
                stream.watcher_count == 0
                    && now.duration_since(stream.last_touched) >= self.stream_idle_ttl
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in stale_keys {
            if let Some(stream) = state.streams.remove(&key) {
                state.total_replay_bytes =
                    state.total_replay_bytes.saturating_sub(stream.total_bytes);
            }
        }
    }

    fn evict_from_stream(stream: &mut ReplayStream) -> Option<usize> {
        let entry = stream.entries.pop_front()?;
        stream.total_bytes = stream.total_bytes.saturating_sub(entry.bytes);
        Some(entry.bytes)
    }

    fn enforce_global_budget_locked(&self, state: &mut AguiSinkState) {
        while state.total_replay_bytes > self.global_replay_bytes {
            let oldest_key = state
                .streams
                .iter()
                .filter_map(|(key, stream)| {
                    stream
                        .entries
                        .front()
                        .map(|entry| (key.clone(), entry.captured_at))
                })
                .min_by_key(|(_, captured_at)| *captured_at)
                .map(|(key, _)| key);
            let Some(oldest_key) = oldest_key else {
                break;
            };
            let mut evicted_bytes = 0usize;
            if let Some(stream) = state.streams.get_mut(&oldest_key) {
                if let Some(bytes) = Self::evict_from_stream(stream) {
                    evicted_bytes = bytes;
                }
            }
            if evicted_bytes > 0 {
                state.total_replay_bytes = state.total_replay_bytes.saturating_sub(evicted_bytes);
                state.replay_evicted += 1;
            }
            let remove_empty = state
                .streams
                .get(&oldest_key)
                .is_some_and(|stream| stream.entries.is_empty() && stream.watcher_count == 0);
            if remove_empty {
                state.streams.remove(&oldest_key);
            }
        }
    }
}

impl EventSink for AguiEventSink {
    fn name(&self) -> &'static str {
        "agui_stream"
    }

    fn accepts(&self, event: &BusEvent) -> bool {
        event.stream_key().is_some()
    }

    fn handle(&self, event: Arc<BusEvent>) {
        let Some(stream_key) = event.stream_key().map(str::to_string) else {
            return;
        };
        let now = Instant::now();
        let mut state = self.state.lock().expect("agui sink mutex poisoned");
        self.cleanup_idle_locked(&mut state, now);
        let Some(_) = state.streams.get(&stream_key) else {
            return;
        };
        let entry_bytes = event.approx_size_bytes();
        let mut evicted_bytes = 0usize;
        let mut evicted_count = 0usize;
        let remove_empty_stream;
        {
            let stream = state
                .streams
                .get_mut(&stream_key)
                .expect("stream must exist after presence check");
            stream.entries.push_back(ReplayEntry {
                event,
                bytes: entry_bytes,
                captured_at: now,
            });
            stream.total_bytes += entry_bytes;
            stream.last_touched = now;

            while stream.entries.len() > self.replay_events_per_stream
                || stream.total_bytes > self.replay_bytes_per_stream
            {
                if let Some(bytes) = Self::evict_from_stream(stream) {
                    evicted_bytes += bytes;
                    evicted_count += 1;
                } else {
                    break;
                }
            }
            remove_empty_stream = stream.entries.is_empty() && stream.watcher_count == 0;
        }
        state.total_replay_bytes += entry_bytes;
        state.total_replay_bytes = state.total_replay_bytes.saturating_sub(evicted_bytes);
        state.replay_evicted += evicted_count;
        if remove_empty_stream {
            state.streams.remove(&stream_key);
        }
        self.enforce_global_budget_locked(&mut state);
    }
}
