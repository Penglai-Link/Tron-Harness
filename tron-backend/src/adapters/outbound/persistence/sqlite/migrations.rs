pub(super) const REMOVE_CHATS_NAVIGATION_MIGRATION: &str =
    include_str!("../../../../../migrations/20261008_0001_remove_chats_navigation.sql");
pub(super) const NAVIGATION_MIGRATION: &str =
    include_str!("../../../../../migrations/20260829_0002_create_navigation_management.sql");
pub(super) const NAVIGATION_AGENTS_SPLIT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260902_0001_split_workspace_agents_navigation.sql");
pub(super) const CHAT_PANEL_NAVIGATION_MIGRATION: &str =
    include_str!("../../../../../migrations/20260915_0001_chat_panel_navigation.sql");
pub(super) const NAVIGATION_MENU_TREES_MIGRATION: &str =
    include_str!("../../../../../migrations/20260922_0001_create_navigation_menu_trees.sql");
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
pub(super) const TASK_FLOWS_MIGRATION: &str =
    include_str!("../../../../../migrations/20260921_0002_create_task_flows.sql");
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
pub(super) const PROMPT_AGENT_ROLE_CATEGORY_MIGRATION: &str =
    include_str!("../../../../../migrations/20261008_0003_add_agent_role_prompt_category.sql");
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
pub(super) const AGENT_MANAGEMENT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260529_0022_create_agent_management.sql");
pub(super) const INTERNAL_AGENT_DEFINITIONS_MIGRATION: &str =
    include_str!("../../../../../migrations/20261008_0002_create_internal_agent_definitions.sql");
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
pub(super) const PROJECT_CHAT_CONTEXT_MIGRATION: &str =
    include_str!("../../../../../migrations/20260911_0001_create_project_chat_context.sql");

pub(super) const AGENT_LOOPS_MIGRATION: &str =
    include_str!("../../../../../migrations/20261009_0001_create_agent_loops.sql");

pub(super) const BUILTIN_WORKFLOW_LOOPS_MIGRATION: &str =
    include_str!("../../../../../migrations/20261009_0002_builtin_workflow_loops.sql");

pub(super) const INTERNAL_AGENT_GROUPS_MIGRATION: &str =
    include_str!("../../../../../migrations/20261010_0001_create_internal_agent_groups.sql");
