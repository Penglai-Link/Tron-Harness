pub(super) const USERS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0001_create_user_management_users.sql");
pub(super) const ASSIGNMENTS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0002_create_user_management_assignments.sql");
pub(super) const PASSWORD_RESET_MIGRATION: &str =
    include_str!("../../../../../migrations/20260511_0003_create_user_password_reset_events.sql");
pub(super) const AUTH_REFRESH_SESSIONS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0004_create_auth_refresh_sessions.sql");
pub(super) const AUTH_REFRESH_SESSIONS_LDAP_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260609_0001_add_ldap_auth_method_to_auth_refresh_sessions.sql"
);
pub(super) const AUTH_OAUTH_IDENTITIES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0005_create_user_oauth_identities.sql");
pub(super) const ROLES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0004_create_role_management_roles.sql");
pub(super) const ROLE_PERMISSIONS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260515_0005_create_role_management_permissions.sql");
pub(super) const WORKSPACES_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260516_0006_create_workspace_management_workspaces.sql"
);
pub(super) const WORKSPACE_ACCESS_GRANTS_MIGRATION: &str = include_str!(
    "../../../../../migrations/20260516_0007_create_workspace_management_access_grants.sql"
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
pub(super) const MODELS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260520_0012_create_model_management_models.sql");
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
pub(super) const SKILL_REGISTRY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260519_0011_create_skill_registry.sql");
pub(super) const BOOTSTRAP_PLATFORM_ADMIN_MIGRATION: &str =
    include_str!("../../../../../migrations/20260526_0017_bootstrap_platform_admin.sql");
pub(super) const AGENT_RUN_HISTORY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260527_0017_create_agent_run_history.sql");
pub(super) const AGENT_RUN_TOKEN_USAGE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260528_0020_add_agent_run_token_usage.sql");
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
pub(super) const AGENT_RUNTIME_EVENT_REPLAY_MIGRATION: &str =
    include_str!("../../../../../migrations/20260530_0026_create_agent_runtime_event_replay.sql");
pub(super) const CHAT_MESSAGE_TOKEN_USAGE_MIGRATION: &str =
    include_str!("../../../../../migrations/20260528_0021_add_chat_message_token_usage.sql");
pub(super) const AGENT_MANAGEMENT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260529_0022_create_agent_management.sql");
pub(super) const RAG_FOUNDATION_MIGRATION: &str =
    include_str!("../../../../../migrations/20260530_0022_create_rag_foundation.sql");
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
pub(super) const OPEN_APPS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260611_0001_create_platform_open_apps.sql");
