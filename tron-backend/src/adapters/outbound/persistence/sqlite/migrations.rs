pub(super) const NAVIGATION_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260829_0002_create_navigation_management.sql"
);
pub(super) const NAVIGATION_AGENTS_SPLIT_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260902_0001_split_workspace_agents_navigation.sql"
);
pub(super) const CHAT_PANEL_NAVIGATION_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260915_0001_chat_panel_navigation.sql"
);
pub(super) const RESOURCE_MANAGEMENT_SIMPLIFY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260818_0001_simplify_resource_management.sql");
pub(super) const CHAT_SESSION_ATTACHMENTS_RESOURCE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260820_0001_create_chat_session_attachments.sql");
pub(super) const WORKSPACES_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260516_0006_create_workspace_management_workspaces.sql"
);
pub(super) const TASKS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260516_0008_create_task_management_tasks.sql");
pub(super) const TASKS_SUPERPOWER_RESET_MIGRATION: &str =
    include_str!("../../../../../migrations/20260519_0010_reset_task_management_superpower.sql");
pub(super) const AUTOMATIONS_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260519_0011_create_automation_management_automations.sql"
);
pub(super) const APPS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260519_0011_create_app_management_apps.sql");
pub(super) const VARIABLES_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260518_0009_create_variable_management_variables.sql"
);
pub(super) const PLATFORM_VARIABLES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260525_0016_create_platform_variables.sql");
pub(super) const VARIABLE_SENSITIVE_METADATA_MIGRATION: &str =
    include_str!("../../../../../migrations/20260602_0028_add_variable_sensitive_metadata.sql");
pub(super) const MODELS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260520_0012_create_model_management_models.sql");
pub(super) const PROMPTS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260713_0001_create_prompt_management.sql");
pub(super) const PROMPT_VARIABLE_METADATA_MIGRATION: &str =
    include_str!("../../../../../migrations/20260713_0002_add_prompt_variable_metadata.sql");
pub(super) const PROMPT_CATALOG_MIGRATION: &str =
    include_str!("../../../../../migrations/20260823_0001_prompt_catalog_i18n.sql");
pub(super) const PROMPT_ACTION_CATEGORY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260824_0001_add_action_prompt_category.sql");
pub(super) const PROMPT_VARIABLE_BINDINGS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260824_0002_add_prompt_variable_bindings.sql");
pub(super) const MODEL_CAPABILITIES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260522_0013_add_model_capabilities_to_models.sql");
pub(super) const MODEL_REMOVE_EXPLICIT_PLATFORM_RESTORE_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260522_0014_remove_explicit_workspace_platform_restore.sql"
);
pub(super) const MODEL_REMOVE_SEEDED_DATA_MIGRATION: &str =
    include_str!("../../../../../migrations/20260522_0015_remove_seeded_model_data.sql");
pub(super) const MODEL_CONTEXT_WINDOW_TOKENS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260528_0019_add_model_context_window_tokens.sql");
pub(super) const MODEL_RESPONSES_STATEFUL_ENABLED_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260604_0020_add_model_responses_stateful_enabled.sql"
);
pub(super) const MODEL_SUPPORTED_REASONING_EFFORTS_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260817_0001_add_model_supported_reasoning_efforts.sql"
);
pub(super) const MODEL_DEFAULT_REASONING_EFFORT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260817_0002_add_model_default_reasoning_effort.sql");
pub(super) const SKILL_REGISTRY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260519_0011_create_skill_registry.sql");
pub(super) const AGENT_RUN_HISTORY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260527_0017_create_agent_run_history.sql");
pub(super) const AGENT_RUN_TOKEN_USAGE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260528_0020_add_agent_run_token_usage.sql");
pub(super) const AGENT_RUN_REASONING_EFFORT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260817_0003_add_agent_run_reasoning_effort.sql");
pub(super) const CHAT_MANAGEMENT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260527_0018_create_chat_management.sql");
pub(super) const CHAT_SESSION_AGENT_CODE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260530_0023_add_chat_session_agent_code.sql");
pub(super) const CHAT_SESSION_SYSTEM_AGENT_SESSION_ID_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260530_0024_add_chat_session_system_agent_session_id.sql"
);
pub(super) const CONVERSATION_RUNTIME_SESSIONS_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260530_0025_create_conversation_runtime_sessions.sql"
);
pub(super) const CHAT_SESSION_SOURCE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260623_0030_add_chat_session_source.sql");
pub(super) const AGENT_RUNTIME_EVENT_REPLAY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260530_0026_create_agent_runtime_event_replay.sql");
pub(super) const CHAT_MESSAGE_TOKEN_USAGE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260528_0021_add_chat_message_token_usage.sql");
pub(super) const CHAT_MESSAGE_ATTACHMENTS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260625_0001_add_chat_message_attachments.sql");
pub(super) const AGENT_MANAGEMENT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260529_0022_create_agent_management.sql");
pub(super) const RAG_FOUNDATION_MIGRATION: &str =
    include_str!("../../../../../migrations/20260530_0022_create_rag_foundation.sql");
pub(super) const RAG_DOMAIN_FIELDS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260530_0023_add_rag_domain_fields.sql");
pub(super) const RAG_INGEST_PROFILES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260531_0024_add_rag_ingest_profiles.sql");
pub(super) const RAG_SEARCH_PROFILES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260531_0025_add_rag_search_profiles.sql");
pub(super) const RAG_OPTIONAL_MODEL_CODES_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260601_0026_make_rag_profile_model_codes_optional.sql"
);
pub(super) const RAG_STORAGE_BINDING_LAYOUT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260601_0027_add_rag_storage_binding_layout.sql");
pub(super) const RAG_MEMORY_EMBEDDED_SOURCE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260601_0028_embed_rag_memory_source.sql");
pub(super) const RAG_MEMORY_TARGETED_STORAGE_MIGRATION: &str = RAG_MEMORY_EMBEDDED_SOURCE_MIGRATION;
pub(super) const RAG_SKILL_CORPUS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260602_0030_create_skill_rag_corpus.sql");
pub(super) const RAG_SKILL_GRAPH_CORPUS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260604_0031_create_skill_graph_rag_corpus.sql");
pub(super) const RAG_STORAGE_BACKEND_FUSION_MIGRATION: &str =
    include_str!("../../../../../migrations/20260604_0032_rag_storage_backend_fusion.sql");
pub(super) const RAG_MEMORY_QUERY_PROFILE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260605_0032_add_memory_query_profile.sql");
pub(super) const RAG_MEMORY_TIMESTAMP_RETURN_FIELDS_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260616_0002_add_rag_memory_timestamp_return_fields.sql"
);
pub(super) const RAG_MEMORY_INGEST_VECTOR_DEGRADE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260616_0003_allow_memory_ingest_vector_degrade.sql");
pub(super) const RAG_PROJECT_DOC_MINERU_INGEST_PROFILE_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260618_0001_add_project_doc_mineru_ingest_profile.sql"
);
pub(super) const MEMORY_WORK_DOMAINS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260616_0001_create_memory_work_domains.sql");
pub(super) const AGENT_MANAGEMENT_RUNTIME_TYPE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260601_0027_add_agent_management_runtime_type.sql");
pub(super) const AGENT_MANAGEMENT_INTERNAL_AGENT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260602_0001_add_internal_agent_policy_tables.sql");
pub(super) const AGENT_MANAGEMENT_INTERNAL_AGENT_LIMITS_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260604_0001_add_internal_agent_limits_to_workspace_internal_agent_policies.sql"
);
pub(super) const AGENT_MANAGEMENT_INTERNAL_AGENT_PARALLEL_LIMIT_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260604_0002_add_internal_agent_parallel_limits_to_workspace_internal_agent_policies.sql"
);
pub(super) const MCP_GATEWAY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260605_0001_create_platform_mcp_gateway_configs.sql");
pub(super) const MCP_PLATFORM_SERVERS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260605_0002_create_platform_mcp_servers.sql");
pub(super) const MCP_PLATFORM_SERVERS_TRANSPORT_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260608_0004_add_platform_mcp_server_transport_columns.sql"
);
pub(super) const MCP_WORKSPACE_POLICIES_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260605_0003_create_workspace_mcp_server_policies.sql"
);
pub(super) const AGENT_MANAGEMENT_INTERNAL_AGENT_MCP_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260605_1001_add_internal_agent_mcp_access_to_workspace_internal_agent_policies.sql"
);
pub(super) const SHORT_TERM_MEMORY_TRANSCRIPT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260615_0001_create_short_term_memory_transcript.sql");
pub(super) const CHAT_SESSION_LEGACY_SOURCE_MIGRATION: &str =
    "ALTER TABLE chat_sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'USER_CHAT';";
pub(super) const CHAT_SESSION_VISIBILITY_MIGRATION: &str =
    "ALTER TABLE chat_sessions ADD COLUMN visibility TEXT NOT NULL DEFAULT 'VISIBLE';
CREATE INDEX IF NOT EXISTS idx_chat_sessions_visibility_updated
    ON chat_sessions (visibility, updated_at DESC, id DESC);";
pub(super) const CHAT_USER_INPUT_REQUESTS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260821_0001_create_chat_user_input_requests.sql");
pub(super) const PROJECT_CHAT_CONTEXT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260911_0001_create_project_chat_context.sql");
pub(super) const CHAT_SESSION_PROJECT_CODE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260911_0002_add_chat_session_project_code.sql");
