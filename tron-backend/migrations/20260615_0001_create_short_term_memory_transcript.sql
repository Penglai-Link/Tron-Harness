CREATE TABLE IF NOT EXISTS agent_transcript_inputs (
    id INTEGER PRIMARY KEY,
    input_code TEXT NOT NULL UNIQUE,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    run_code TEXT,
    input_kind TEXT NOT NULL,
    delivery TEXT NOT NULL,
    status TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    idempotency_key TEXT,
    prompt_json TEXT NOT NULL,
    metadata_json TEXT,
    admitted_seq INTEGER,
    promoted_seq INTEGER,
    created_at INTEGER NOT NULL,
    promoted_at INTEGER,
    consumed_at INTEGER,
    cancelled_at INTEGER,
    UNIQUE(session_code, input_code)
);

CREATE TABLE IF NOT EXISTS agent_transcript_events (
    id INTEGER PRIMARY KEY,
    event_code TEXT NOT NULL UNIQUE,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    seq INTEGER NOT NULL,
    run_code TEXT,
    input_code TEXT,
    message_code TEXT,
    part_code TEXT,
    event_type TEXT NOT NULL,
    event_source TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    metadata_json TEXT,
    occurred_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(session_code, seq)
);

CREATE TABLE IF NOT EXISTS agent_transcript_messages (
    id INTEGER PRIMARY KEY,
    message_code TEXT NOT NULL UNIQUE,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    run_code TEXT,
    seq INTEGER NOT NULL,
    role TEXT NOT NULL,
    message_type TEXT NOT NULL,
    status TEXT NOT NULL,
    agent_code TEXT,
    model_code TEXT,
    model_provider TEXT,
    model_name TEXT,
    model_protocol TEXT,
    finish_reason TEXT,
    error_json TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    reasoning_tokens INTEGER,
    total_tokens INTEGER,
    metadata_json TEXT,
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    UNIQUE(session_code, seq)
);

CREATE TABLE IF NOT EXISTS agent_transcript_parts (
    id INTEGER PRIMARY KEY,
    part_code TEXT NOT NULL UNIQUE,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    message_code TEXT NOT NULL,
    seq INTEGER NOT NULL,
    part_type TEXT NOT NULL,
    status TEXT NOT NULL,
    content_text TEXT,
    content_json TEXT,
    provider_metadata_json TEXT,
    visibility TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    UNIQUE(message_code, seq)
);

CREATE TABLE IF NOT EXISTS agent_transcript_context_epochs (
    id INTEGER PRIMARY KEY,
    epoch_code TEXT NOT NULL UNIQUE,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    baseline_seq INTEGER NOT NULL,
    replacement_seq INTEGER NOT NULL,
    agent_code TEXT,
    prompt_prefix_message_count INTEGER NOT NULL DEFAULT 0,
    prompt_prefix_hash TEXT,
    prompt_prefix_preview TEXT,
    snapshot_json TEXT NOT NULL,
    system_hash TEXT,
    developer_hash TEXT,
    tool_hash TEXT,
    skill_hash TEXT,
    variable_hash TEXT,
    execution_scope_hash TEXT,
    memory_hash TEXT,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_session_todos (
    id INTEGER PRIMARY KEY,
    todo_code TEXT NOT NULL,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    position INTEGER NOT NULL,
    content TEXT NOT NULL,
    status TEXT NOT NULL,
    priority TEXT NOT NULL,
    created_by_run_code TEXT,
    updated_by_run_code TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(session_code, todo_code),
    UNIQUE(session_code, position)
);

CREATE TABLE IF NOT EXISTS agent_session_families (
    id INTEGER PRIMARY KEY,
    family_code TEXT NOT NULL UNIQUE,
    family_kind TEXT NOT NULL,
    family_key TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    agent_code TEXT NOT NULL,
    user_code TEXT,
    app_code TEXT,
    requirement_code TEXT,
    task_code TEXT,
    parent_family_code TEXT,
    root_session_code TEXT,
    active_session_code TEXT,
    active_run_code TEXT,
    latest_snapshot_code TEXT,
    title TEXT,
    status TEXT NOT NULL,
    load_policy TEXT NOT NULL,
    load_policy_json TEXT,
    metadata_json TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_activity_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_session_family_members (
    id INTEGER PRIMARY KEY,
    family_code TEXT NOT NULL,
    session_code TEXT NOT NULL,
    membership_role TEXT NOT NULL,
    parent_session_code TEXT,
    member_kind TEXT NOT NULL,
    origin_kind TEXT NOT NULL,
    model_context_enabled INTEGER NOT NULL,
    load_weight_override REAL,
    load_limits_json TEXT,
    first_run_code TEXT,
    last_run_code TEXT,
    joined_at INTEGER NOT NULL,
    last_activity_at INTEGER NOT NULL,
    UNIQUE(family_code, session_code)
);

CREATE TABLE IF NOT EXISTS agent_session_family_snapshots (
    id INTEGER PRIMARY KEY,
    snapshot_code TEXT NOT NULL UNIQUE,
    family_code TEXT,
    session_code TEXT NOT NULL,
    run_code TEXT,
    snapshot_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    source_start_seq INTEGER,
    source_end_seq INTEGER,
    source_checkpoint_message_code TEXT,
    summary_markdown TEXT,
    token_estimate INTEGER,
    error_code TEXT,
    error_message TEXT,
    metadata_json TEXT,
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    expires_at INTEGER
);

CREATE TABLE IF NOT EXISTS agent_session_goals (
    id INTEGER PRIMARY KEY,
    goal_code TEXT NOT NULL UNIQUE,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    user_code TEXT,
    task_code TEXT,
    active_family_code TEXT,
    objective_markdown TEXT NOT NULL,
    status TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 0,
    runs_used INTEGER NOT NULL DEFAULT 0,
    max_runs INTEGER NOT NULL,
    last_run_code TEXT,
    last_input_code TEXT,
    last_verdict TEXT,
    last_reason TEXT,
    blocked_reason TEXT,
    required_user_input TEXT,
    consecutive_judge_failures INTEGER NOT NULL DEFAULT 0,
    created_by_run_code TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    paused_at INTEGER,
    blocked_at INTEGER,
    completed_at INTEGER,
    cancelled_at INTEGER,
    cleared_at INTEGER,
    metadata_json TEXT,
    UNIQUE(session_code, goal_code)
);

CREATE TABLE IF NOT EXISTS agent_session_goal_criteria (
    id INTEGER PRIMARY KEY,
    criterion_code TEXT NOT NULL UNIQUE,
    goal_code TEXT NOT NULL,
    session_code TEXT NOT NULL,
    position INTEGER NOT NULL,
    text TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    removed_at INTEGER,
    UNIQUE(goal_code, position)
);

CREATE TABLE IF NOT EXISTS agent_session_goal_events (
    id INTEGER PRIMARY KEY,
    event_code TEXT NOT NULL UNIQUE,
    goal_code TEXT NOT NULL,
    session_code TEXT NOT NULL,
    workspace_code TEXT NOT NULL,
    run_code TEXT,
    input_code TEXT,
    event_kind TEXT NOT NULL,
    event_seq INTEGER NOT NULL,
    summary TEXT,
    payload_json TEXT,
    created_at INTEGER NOT NULL,
    UNIQUE(goal_code, event_seq)
);

CREATE TABLE IF NOT EXISTS agent_session_run_leases (
    id INTEGER PRIMARY KEY,
    session_code TEXT NOT NULL UNIQUE,
    workspace_code TEXT NOT NULL,
    run_code TEXT NOT NULL UNIQUE,
    input_code TEXT,
    started_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_transcript_inputs_session_status
    ON agent_transcript_inputs (session_code, status, priority DESC, created_at ASC, id ASC);
CREATE INDEX IF NOT EXISTS idx_agent_transcript_inputs_workspace_created
    ON agent_transcript_inputs (workspace_code, created_at DESC, id DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_transcript_inputs_session_idempotency
    ON agent_transcript_inputs (session_code, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_transcript_events_session_seq
    ON agent_transcript_events (session_code, seq ASC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_transcript_events_session_seq_unique
    ON agent_transcript_events (session_code, seq);
CREATE INDEX IF NOT EXISTS idx_agent_transcript_events_run
    ON agent_transcript_events (run_code, seq ASC);
CREATE INDEX IF NOT EXISTS idx_agent_transcript_messages_session_seq
    ON agent_transcript_messages (session_code, seq ASC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_transcript_messages_session_seq_unique
    ON agent_transcript_messages (session_code, seq);
CREATE INDEX IF NOT EXISTS idx_agent_transcript_messages_run
    ON agent_transcript_messages (run_code, seq ASC);
CREATE INDEX IF NOT EXISTS idx_agent_transcript_parts_message_seq
    ON agent_transcript_parts (message_code, seq ASC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_transcript_parts_message_seq_unique
    ON agent_transcript_parts (message_code, seq);
CREATE INDEX IF NOT EXISTS idx_agent_transcript_parts_session_message
    ON agent_transcript_parts (session_code, message_code, seq ASC);
CREATE INDEX IF NOT EXISTS idx_agent_transcript_context_epochs_session_revision
    ON agent_transcript_context_epochs (session_code, revision DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_transcript_context_epochs_session_revision_unique
    ON agent_transcript_context_epochs (session_code, revision);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_todos_session_position
    ON agent_session_todos (session_code, position ASC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_todos_session_todo_code
    ON agent_session_todos (session_code, todo_code);

CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_families_kind_key
    ON agent_session_families (family_kind, family_key);
CREATE INDEX IF NOT EXISTS idx_agent_session_families_workspace_agent_activity
    ON agent_session_families (workspace_code, agent_code, status, last_activity_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_families_task
    ON agent_session_families (workspace_code, agent_code, task_code, status, last_activity_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_family_members_family_activity
    ON agent_session_family_members (family_code, last_activity_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_family_members_session
    ON agent_session_family_members (session_code);
CREATE INDEX IF NOT EXISTS idx_agent_session_family_members_session_enabled
    ON agent_session_family_members (session_code, model_context_enabled, last_activity_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_family_snapshots_family_created
    ON agent_session_family_snapshots (family_code, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_family_snapshots_session_created
    ON agent_session_family_snapshots (session_code, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_family_snapshots_family_status
    ON agent_session_family_snapshots (family_code, status, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_family_snapshots_session_latest
    ON agent_session_family_snapshots (session_code, snapshot_kind, status, source_end_seq DESC, id DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_family_snapshots_session_run_summary
    ON agent_session_family_snapshots (session_code, run_code)
    WHERE snapshot_kind = 'SESSION_SUMMARY' AND run_code IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_agent_session_goals_session_status
    ON agent_session_goals (session_code, status, updated_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_goals_workspace_updated
    ON agent_session_goals (workspace_code, updated_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_agent_session_goals_task_status
    ON agent_session_goals (task_code, status, updated_at DESC, id DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_goals_one_open
    ON agent_session_goals (session_code)
    WHERE status IN ('ACTIVE', 'PAUSED', 'BLOCKED');
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_goal_criteria_goal_position
    ON agent_session_goal_criteria (goal_code, position ASC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_goal_events_goal_seq
    ON agent_session_goal_events (goal_code, event_seq ASC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_run_leases_session
    ON agent_session_run_leases (session_code);
CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_session_run_leases_run
    ON agent_session_run_leases (run_code);
