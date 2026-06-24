use super::repository_init_backfills_core::ColumnBackfill;

pub(super) const STM_TRANSCRIPT_COLUMN_BACKFILLS: &[ColumnBackfill] = &[
    ColumnBackfill {
        table: "agent_transcript_events",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_transcript_events",
        column: "run_code",
        ddl: "run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_events",
        column: "message_code",
        ddl: "message_code TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_events",
        column: "part_code",
        ddl: "part_code TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_events",
        column: "metadata_json",
        ddl: "metadata_json TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "run_code",
        ddl: "run_code TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "agent_code",
        ddl: "agent_code TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "model_provider",
        ddl: "model_provider TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "model_name",
        ddl: "model_name TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "model_protocol",
        ddl: "model_protocol TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "error_json",
        ddl: "error_json TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "reasoning_tokens",
        ddl: "reasoning_tokens INTEGER",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "metadata_json",
        ddl: "metadata_json TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_messages",
        column: "completed_at",
        ddl: "completed_at INTEGER",
    },
    ColumnBackfill {
        table: "agent_transcript_parts",
        column: "workspace_code",
        ddl: "workspace_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_transcript_parts",
        column: "session_code",
        ddl: "session_code TEXT NOT NULL DEFAULT ''",
    },
    ColumnBackfill {
        table: "agent_transcript_parts",
        column: "provider_metadata_json",
        ddl: "provider_metadata_json TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_parts",
        column: "visibility",
        ddl: "visibility TEXT NOT NULL DEFAULT 'model'",
    },
    ColumnBackfill {
        table: "agent_transcript_parts",
        column: "completed_at",
        ddl: "completed_at INTEGER",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "replacement_seq",
        ddl: "replacement_seq INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "agent_code",
        ddl: "agent_code TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "prompt_prefix_message_count",
        ddl: "prompt_prefix_message_count INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "prompt_prefix_hash",
        ddl: "prompt_prefix_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "prompt_prefix_preview",
        ddl: "prompt_prefix_preview TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "system_hash",
        ddl: "system_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "developer_hash",
        ddl: "developer_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "tool_hash",
        ddl: "tool_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "skill_hash",
        ddl: "skill_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "variable_hash",
        ddl: "variable_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "execution_scope_hash",
        ddl: "execution_scope_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "memory_hash",
        ddl: "memory_hash TEXT",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "revision",
        ddl: "revision INTEGER NOT NULL DEFAULT 0",
    },
    ColumnBackfill {
        table: "agent_transcript_context_epochs",
        column: "updated_at",
        ddl: "updated_at INTEGER NOT NULL DEFAULT 0",
    },
];
