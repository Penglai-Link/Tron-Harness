CREATE TABLE IF NOT EXISTS agent_runtime_event_replay (
    id INTEGER PRIMARY KEY,
    bus_event_id TEXT NOT NULL,
    stream_key TEXT NOT NULL,
    run_code TEXT,
    occurred_at INTEGER NOT NULL,
    canonical_event_code TEXT NOT NULL,
    category TEXT NOT NULL,
    ordinal INTEGER NOT NULL DEFAULT 0,
    sse_event TEXT,
    payload_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (bus_event_id, category, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_agent_runtime_event_replay_stream_order
    ON agent_runtime_event_replay(stream_key, occurred_at, id);

CREATE INDEX IF NOT EXISTS idx_agent_runtime_event_replay_stream_category
    ON agent_runtime_event_replay(stream_key, category, occurred_at, id);

CREATE INDEX IF NOT EXISTS idx_agent_runtime_event_replay_run
    ON agent_runtime_event_replay(run_code);
