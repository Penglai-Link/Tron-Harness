-- Raise the old 5-minute PersonAI heartbeat default to 10 minutes. Only rows
-- still on the previous default are touched, so explicitly chosen intervals
-- survive. record_json is the read model, so keep it in sync with the indexed
-- column, and restart the schedule from now using the new interval.

UPDATE assistants
SET heartbeat_minutes = 10,
    record_json = json_set(record_json, '$.heartbeatMinutes', 10),
    next_heartbeat_at = CAST(strftime('%s', 'now') AS INTEGER) * 1000 + 600000
WHERE heartbeat_minutes = 5;