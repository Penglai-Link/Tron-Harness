pub(super) const STM_REQUIRED_COLUMNS: &[(&str, &[&str])] = &[
    (
        "agent_transcript_events",
        &[
            "event_code",
            "session_code",
            "workspace_code",
            "seq",
            "run_code",
            "input_code",
            "message_code",
            "part_code",
            "event_type",
            "payload_json",
        ],
    ),
    (
        "agent_transcript_messages",
        &[
            "message_code",
            "session_code",
            "workspace_code",
            "seq",
            "role",
            "message_type",
            "status",
            "agent_code",
            "metadata_json",
            "completed_at",
        ],
    ),
    (
        "agent_transcript_parts",
        &[
            "part_code",
            "session_code",
            "workspace_code",
            "message_code",
            "part_type",
            "content_text",
            "content_json",
            "provider_metadata_json",
            "visibility",
        ],
    ),
    (
        "agent_transcript_context_epochs",
        &[
            "epoch_code",
            "session_code",
            "baseline_seq",
            "replacement_seq",
            "snapshot_json",
            "revision",
            "updated_at",
        ],
    ),
    (
        "agent_session_todos",
        &[
            "todo_code",
            "session_code",
            "workspace_code",
            "position",
            "content",
            "status",
            "priority",
            "created_at",
            "updated_at",
        ],
    ),
    (
        "agent_session_families",
        &[
            "family_code",
            "family_kind",
            "family_key",
            "workspace_code",
            "agent_code",
            "status",
            "load_policy",
            "created_at",
            "updated_at",
            "last_activity_at",
        ],
    ),
    (
        "agent_session_family_members",
        &[
            "family_code",
            "session_code",
            "membership_role",
            "member_kind",
            "origin_kind",
            "model_context_enabled",
            "joined_at",
            "last_activity_at",
        ],
    ),
    (
        "agent_session_family_snapshots",
        &[
            "snapshot_code",
            "family_code",
            "session_code",
            "status",
            "summary_markdown",
            "error_code",
            "error_message",
        ],
    ),
    (
        "agent_session_goals",
        &[
            "goal_code",
            "session_code",
            "status",
            "revision",
            "last_input_code",
        ],
    ),
    (
        "agent_session_goal_criteria",
        &[
            "criterion_code",
            "goal_code",
            "session_code",
            "position",
            "text",
            "status",
            "created_at",
            "updated_at",
        ],
    ),
    (
        "agent_session_goal_events",
        &[
            "event_code",
            "goal_code",
            "session_code",
            "workspace_code",
            "event_kind",
            "event_seq",
            "created_at",
        ],
    ),
    (
        "agent_session_run_leases",
        &[
            "session_code",
            "workspace_code",
            "run_code",
            "input_code",
            "started_at",
            "updated_at",
        ],
    ),
];

pub(super) const STM_REQUIRED_INDEXES: &[&str] = &[
    "idx_agent_transcript_events_session_seq",
    "idx_agent_transcript_events_session_seq_unique",
    "idx_agent_transcript_inputs_session_idempotency",
    "idx_agent_transcript_messages_session_seq",
    "idx_agent_transcript_messages_session_seq_unique",
    "idx_agent_transcript_parts_message_seq",
    "idx_agent_transcript_parts_message_seq_unique",
    "idx_agent_transcript_context_epochs_session_revision",
    "idx_agent_transcript_context_epochs_session_revision_unique",
    "idx_agent_session_todos_session_position",
    "idx_agent_session_family_members_session_enabled",
    "idx_agent_session_family_snapshots_family_status",
    "idx_agent_session_family_snapshots_session_latest",
    "idx_agent_session_family_snapshots_session_run_summary",
    "idx_agent_session_goals_one_open",
    "idx_agent_session_goal_criteria_goal_position",
    "idx_agent_session_goal_events_goal_seq",
    "idx_agent_session_todos_session_todo_code",
    "idx_agent_session_run_leases_session",
    "idx_agent_session_run_leases_run",
];

pub(super) struct StmRequiredColumnDefinition {
    pub(super) table: &'static str,
    pub(super) column: &'static str,
    pub(super) data_type: &'static str,
    pub(super) not_null: bool,
    pub(super) allowed_default_values: &'static [Option<&'static str>],
}

pub(super) const STM_REQUIRED_COLUMN_DEFINITIONS: &[StmRequiredColumnDefinition] = &[
    StmRequiredColumnDefinition {
        table: "agent_session_family_snapshots",
        column: "family_code",
        data_type: "TEXT",
        not_null: false,
        allowed_default_values: &[None],
    },
    StmRequiredColumnDefinition {
        table: "agent_transcript_context_epochs",
        column: "replacement_seq",
        data_type: "INTEGER",
        not_null: true,
        allowed_default_values: &[None, Some("0")],
    },
    StmRequiredColumnDefinition {
        table: "agent_transcript_context_epochs",
        column: "revision",
        data_type: "INTEGER",
        not_null: true,
        allowed_default_values: &[Some("0")],
    },
    StmRequiredColumnDefinition {
        table: "agent_transcript_context_epochs",
        column: "updated_at",
        data_type: "INTEGER",
        not_null: true,
        allowed_default_values: &[None, Some("0")],
    },
    StmRequiredColumnDefinition {
        table: "agent_session_goals",
        column: "status",
        data_type: "TEXT",
        not_null: true,
        allowed_default_values: &[None, Some("PAUSED")],
    },
    StmRequiredColumnDefinition {
        table: "agent_session_goals",
        column: "updated_at",
        data_type: "INTEGER",
        not_null: true,
        allowed_default_values: &[None, Some("0")],
    },
    StmRequiredColumnDefinition {
        table: "agent_session_goals",
        column: "revision",
        data_type: "INTEGER",
        not_null: true,
        allowed_default_values: &[Some("0")],
    },
    StmRequiredColumnDefinition {
        table: "agent_session_goal_events",
        column: "event_seq",
        data_type: "INTEGER",
        not_null: true,
        allowed_default_values: &[None, Some("0")],
    },
];

pub(super) struct StmRequiredIndexDefinition {
    pub(super) name: &'static str,
    pub(super) unique: bool,
    pub(super) fragments: &'static [&'static str],
}

pub(super) struct StmJsonColumn {
    pub(super) table: &'static str,
    pub(super) column: &'static str,
}

pub(super) const STM_JSON_COLUMNS: &[StmJsonColumn] = &[
    StmJsonColumn {
        table: "agent_transcript_inputs",
        column: "prompt_json",
    },
    StmJsonColumn {
        table: "agent_transcript_inputs",
        column: "metadata_json",
    },
    StmJsonColumn {
        table: "agent_transcript_events",
        column: "payload_json",
    },
    StmJsonColumn {
        table: "agent_transcript_events",
        column: "metadata_json",
    },
    StmJsonColumn {
        table: "agent_transcript_messages",
        column: "error_json",
    },
    StmJsonColumn {
        table: "agent_transcript_messages",
        column: "metadata_json",
    },
    StmJsonColumn {
        table: "agent_transcript_parts",
        column: "content_json",
    },
    StmJsonColumn {
        table: "agent_transcript_parts",
        column: "provider_metadata_json",
    },
    StmJsonColumn {
        table: "agent_transcript_context_epochs",
        column: "snapshot_json",
    },
    StmJsonColumn {
        table: "agent_session_families",
        column: "load_policy_json",
    },
    StmJsonColumn {
        table: "agent_session_families",
        column: "metadata_json",
    },
    StmJsonColumn {
        table: "agent_session_family_members",
        column: "load_limits_json",
    },
    StmJsonColumn {
        table: "agent_session_family_snapshots",
        column: "metadata_json",
    },
    StmJsonColumn {
        table: "agent_session_goals",
        column: "metadata_json",
    },
    StmJsonColumn {
        table: "agent_session_goal_events",
        column: "payload_json",
    },
];

pub(super) struct StmAllowedValues {
    pub(super) table: &'static str,
    pub(super) column: &'static str,
    pub(super) values: &'static [&'static str],
}

pub(super) const STM_ALLOWED_VALUES: &[StmAllowedValues] = &[
    StmAllowedValues {
        table: "agent_transcript_messages",
        column: "role",
        values: &[
            "system",
            "developer",
            "user",
            "assistant",
            "tool",
            "checkpoint",
        ],
    },
    StmAllowedValues {
        table: "agent_transcript_messages",
        column: "message_type",
        values: &["normal", "tool_result", "compaction"],
    },
    StmAllowedValues {
        table: "agent_transcript_messages",
        column: "status",
        values: &["pending", "in_progress", "completed", "failed", "cancelled"],
    },
    StmAllowedValues {
        table: "agent_transcript_parts",
        column: "part_type",
        values: &[
            "text",
            "reasoning",
            "reasoning_summary",
            "tool_call",
            "tool_result",
            "context",
            "file",
            "compaction_summary",
            "compaction_recent",
        ],
    },
    StmAllowedValues {
        table: "agent_transcript_parts",
        column: "status",
        values: &["pending", "in_progress", "completed", "failed", "cancelled"],
    },
    StmAllowedValues {
        table: "agent_transcript_parts",
        column: "visibility",
        values: &["model", "ui", "internal", "audit"],
    },
    StmAllowedValues {
        table: "agent_session_todos",
        column: "status",
        values: &["pending", "in_progress", "completed", "cancelled"],
    },
    StmAllowedValues {
        table: "agent_session_todos",
        column: "priority",
        values: &["high", "medium", "low"],
    },
    StmAllowedValues {
        table: "agent_session_families",
        column: "family_kind",
        values: &[
            "WORKSPACE_CHAT",
            "WORKSPACE_HEARTBEAT",
            "REQUIREMENT",
            "TASK",
            "FORK",
        ],
    },
    StmAllowedValues {
        table: "agent_session_families",
        column: "status",
        values: &["ACTIVE", "PAUSED", "COMPLETED", "DISABLED", "ARCHIVED"],
    },
    StmAllowedValues {
        table: "agent_session_family_snapshots",
        column: "snapshot_kind",
        values: &["RUN_HANDOFF", "SESSION_SUMMARY"],
    },
    StmAllowedValues {
        table: "agent_session_family_snapshots",
        column: "status",
        values: &["PENDING", "IN_PROGRESS", "COMPLETED", "FAILED", "EXPIRED"],
    },
    StmAllowedValues {
        table: "agent_session_goals",
        column: "status",
        values: &[
            "ACTIVE",
            "PAUSED",
            "BLOCKED",
            "COMPLETED",
            "CANCELLED",
            "CLEARED",
        ],
    },
    StmAllowedValues {
        table: "agent_session_goal_criteria",
        column: "status",
        values: &["ACTIVE", "REMOVED"],
    },
];

pub(super) struct StmOrphanCheck {
    pub(super) sql: &'static str,
}

pub(super) const STM_ORPHAN_CHECKS: &[StmOrphanCheck] = &[
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_transcript_parts p
              LEFT JOIN agent_transcript_messages m ON m.message_code = p.message_code
              WHERE m.message_code IS NULL",
    },
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_transcript_events e
              LEFT JOIN agent_transcript_inputs i ON i.input_code = e.input_code
              WHERE e.input_code IS NOT NULL
                AND i.input_code IS NULL",
    },
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_transcript_events e
              LEFT JOIN agent_transcript_messages m ON m.message_code = e.message_code
              WHERE e.message_code IS NOT NULL
                AND m.message_code IS NULL",
    },
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_transcript_events e
              LEFT JOIN agent_transcript_parts p ON p.part_code = e.part_code
              WHERE e.part_code IS NOT NULL
                AND p.part_code IS NULL",
    },
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_session_goal_criteria c
              LEFT JOIN agent_session_goals g ON g.goal_code = c.goal_code
              WHERE g.goal_code IS NULL",
    },
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_session_goal_events e
              LEFT JOIN agent_session_goals g ON g.goal_code = e.goal_code
              WHERE g.goal_code IS NULL",
    },
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_session_family_members m
              LEFT JOIN agent_session_families f ON f.family_code = m.family_code
              WHERE f.family_code IS NULL",
    },
    StmOrphanCheck {
        sql: "SELECT count(1)
              FROM agent_session_family_snapshots s
              LEFT JOIN agent_session_families f ON f.family_code = s.family_code
              WHERE s.family_code IS NOT NULL
                AND f.family_code IS NULL",
    },
];

pub(super) const STM_REQUIRED_INDEX_DEFINITIONS: &[StmRequiredIndexDefinition] = &[
    StmRequiredIndexDefinition {
        name: "idx_agent_transcript_events_session_seq_unique",
        unique: true,
        fragments: &["ON agent_transcript_events (session_code, seq)"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_transcript_messages_session_seq_unique",
        unique: true,
        fragments: &["ON agent_transcript_messages (session_code, seq)"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_transcript_parts_message_seq_unique",
        unique: true,
        fragments: &["ON agent_transcript_parts (message_code, seq)"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_transcript_inputs_session_idempotency",
        unique: true,
        fragments: &[
            "ON agent_transcript_inputs (session_code, idempotency_key)",
            "WHERE idempotency_key IS NOT NULL",
        ],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_transcript_context_epochs_session_revision_unique",
        unique: true,
        fragments: &["ON agent_transcript_context_epochs (session_code, revision)"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_todos_session_position",
        unique: true,
        fragments: &["ON agent_session_todos (session_code, position"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_todos_session_todo_code",
        unique: true,
        fragments: &["ON agent_session_todos (session_code, todo_code)"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_goal_criteria_goal_position",
        unique: true,
        fragments: &["ON agent_session_goal_criteria (goal_code, position"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_goal_events_goal_seq",
        unique: true,
        fragments: &["ON agent_session_goal_events (goal_code, event_seq"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_run_leases_session",
        unique: true,
        fragments: &["ON agent_session_run_leases (session_code)"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_run_leases_run",
        unique: true,
        fragments: &["ON agent_session_run_leases (run_code)"],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_goals_one_open",
        unique: true,
        fragments: &[
            "ON agent_session_goals (session_code)",
            "WHERE status IN ('ACTIVE', 'PAUSED', 'BLOCKED')",
        ],
    },
    StmRequiredIndexDefinition {
        name: "idx_agent_session_family_snapshots_session_run_summary",
        unique: true,
        fragments: &[
            "ON agent_session_family_snapshots (session_code, run_code)",
            "WHERE snapshot_kind = 'SESSION_SUMMARY' AND run_code IS NOT NULL",
        ],
    },
];
