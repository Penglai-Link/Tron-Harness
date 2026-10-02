CREATE TABLE IF NOT EXISTS agent_runtime_event_replay_failure (
    stream_key TEXT PRIMARY KEY,
    detail TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- AwaitingInput is a native RUN_FINISHED with interrupt outcome, and is excluded.
-- Only executions already present when the store opens can be interrupted by restart.
INSERT OR IGNORE INTO agent_runtime_event_replay_failure (stream_key, detail, created_at)
SELECT stream_key, 'native presentation execution ended without a terminal acknowledgment', CAST(strftime('%s', 'now') AS INTEGER) * 1000
FROM agent_runtime_event_replay
WHERE category = 'PROTOCOL'
  AND json_extract(payload_json, '$.data.metadata.framework.executionId') IS NOT NULL
GROUP BY stream_key, json_extract(payload_json, '$.data.metadata.framework.executionId')
HAVING SUM(CASE WHEN sse_event = 'RUN_STARTED' THEN 1 ELSE 0 END) > 0
   AND SUM(CASE WHEN sse_event IN ('RUN_FINISHED', 'RUN_ERROR') THEN 1 ELSE 0 END) = 0;
