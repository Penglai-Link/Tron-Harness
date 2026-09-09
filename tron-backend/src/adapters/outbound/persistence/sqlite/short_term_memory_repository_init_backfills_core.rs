#[derive(Clone, Copy)]
pub(super) struct ColumnBackfill {
    pub(super) table: &'static str,
    pub(super) column: &'static str,
    pub(super) ddl: &'static str,
}

pub(super) const STM_CORE_COLUMN_BACKFILLS: &[ColumnBackfill] = &[
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "status",
        ddl: "status TEXT NOT NULL DEFAULT 'CONSUMED'",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "priority",
        ddl: "priority INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "run_code",
        ddl: "run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "metadata_json",
        ddl: "metadata_json TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "admitted_seq",
        ddl: "admitted_seq INTEGER",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "promoted_seq",
        ddl: "promoted_seq INTEGER",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "created_at",
        ddl: "created_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_transcript_inputs",
        column: "consumed_at",
        ddl: "consumed_at INTEGER",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "session_code",
        ddl: "session_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "position",
        ddl: "position INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "content",
        ddl: "content TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "status",
        ddl: "status TEXT NOT NULL DEFAULT 'pending'",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "priority",
        ddl: "priority TEXT NOT NULL DEFAULT 'medium'",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "created_by_run_code",
        ddl: "created_by_run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "updated_by_run_code",
        ddl: "updated_by_run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "created_at",
        ddl: "created_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_todos",
        column: "updated_at",
        ddl: "updated_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "family_kind",
        ddl: "family_kind TEXT NOT NULL DEFAULT 'WORKSPACE_CHAT'",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "family_key",
        ddl: "family_key TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "agent_code",
        ddl: "agent_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "task_code",
        ddl: "task_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "active_session_code",
        ddl: "active_session_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "active_run_code",
        ddl: "active_run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "latest_snapshot_code",
        ddl: "latest_snapshot_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "status",
        ddl: "status TEXT NOT NULL DEFAULT 'ACTIVE'",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "load_policy",
        ddl: "load_policy TEXT NOT NULL DEFAULT 'DEFAULT'",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "load_policy_json",
        ddl: "load_policy_json TEXT",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "metadata_json",
        ddl: "metadata_json TEXT",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "created_at",
        ddl: "created_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "updated_at",
        ddl: "updated_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_families",
        column: "last_activity_at",
        ddl: "last_activity_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "family_code",
        ddl: "family_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "session_code",
        ddl: "session_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "membership_role",
        ddl: "membership_role TEXT NOT NULL DEFAULT 'PRIMARY'",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "member_kind",
        ddl: "member_kind TEXT NOT NULL DEFAULT 'CHAT_SESSION'",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "origin_kind",
        ddl: "origin_kind TEXT NOT NULL DEFAULT 'DIRECT'",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "model_context_enabled",
        ddl: "model_context_enabled INTEGER NOT NULL DEFAULT 1",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "load_limits_json",
        ddl: "load_limits_json TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "joined_at",
        ddl: "joined_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_family_members",
        column: "last_activity_at",
        ddl: "last_activity_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "family_code",
        ddl: "family_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "session_code",
        ddl: "session_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "run_code",
        ddl: "run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "snapshot_kind",
        ddl: "snapshot_kind TEXT NOT NULL DEFAULT 'RUN_HANDOFF'",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "status",
        ddl: "status TEXT NOT NULL DEFAULT 'FAILED'",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "summary_markdown",
        ddl: "summary_markdown TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "error_code",
        ddl: "error_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "error_message",
        ddl: "error_message TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "metadata_json",
        ddl: "metadata_json TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "created_at",
        ddl: "created_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "source_start_seq",
        ddl: "source_start_seq INTEGER",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "source_end_seq",
        ddl: "source_end_seq INTEGER",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "source_checkpoint_message_code",
        ddl: "source_checkpoint_message_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "token_estimate",
        ddl: "token_estimate INTEGER",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "completed_at",
        ddl: "completed_at INTEGER",
    },
    ColumnBackfill {
        table: "agent_session_family_snapshots",
        column: "expires_at",
        ddl: "expires_at INTEGER",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "session_code",
        ddl: "session_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "task_code",
        ddl: "task_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "status",
        ddl: "status TEXT NOT NULL DEFAULT 'PAUSED'",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "revision",
        ddl: "revision INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "last_input_code",
        ddl: "last_input_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "last_verdict",
        ddl: "last_verdict TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "last_reason",
        ddl: "last_reason TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "blocked_reason",
        ddl: "blocked_reason TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "required_user_input",
        ddl: "required_user_input TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "consecutive_judge_failures",
        ddl: "consecutive_judge_failures INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "updated_at",
        ddl: "updated_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_goals",
        column: "metadata_json",
        ddl: "metadata_json TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "criterion_code",
        ddl: "criterion_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "goal_code",
        ddl: "goal_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "session_code",
        ddl: "session_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "position",
        ddl: "position INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "text",
        ddl: "text TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "status",
        ddl: "status TEXT NOT NULL DEFAULT 'ACTIVE'",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "created_at",
        ddl: "created_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "updated_at",
        ddl: "updated_at INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_goal_criteria",
        column: "removed_at",
        ddl: "removed_at INTEGER",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "event_code",
        ddl: "event_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "goal_code",
        ddl: "goal_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "session_code",
        ddl: "session_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "run_code",
        ddl: "run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "input_code",
        ddl: "input_code TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "event_kind",
        ddl: "event_kind TEXT NOT NULL DEFAULT 'UNKNOWN'",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "event_seq",
        ddl: "event_seq INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "summary",
        ddl: "summary TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "payload_json",
        ddl: "payload_json TEXT",
    },
    ColumnBackfill {
        table: "agent_session_goal_events",
        column: "created_at",
        ddl: "created_at INTEGER NOT NULL DEFAULT 0",
    },
];
