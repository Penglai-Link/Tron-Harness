export type PrototypeArchetype =
  | "auth"
  | "dashboard"
  | "data-table"
  | "form"
  | "form-inspector"
  | "graph"
  | "kanban"
  | "split-pane"
  | "chat"
  | "detail"
  | "permission-grid";

export interface PrototypeRoute {
  prototype: string;
  route: string;
  activeNav: string;
  title: string;
  archetype: PrototypeArchetype;
}

export const prototypeRoutes: PrototypeRoute[] = [
  { prototype: "add_model_tron_standardized_navigation", route: "/models/new", activeNav: "Models", title: "Add Model", archetype: "form-inspector" },
  { prototype: "app_management_light", route: "/apps", activeNav: "Apps", title: "App Management", archetype: "data-table" },
  { prototype: "audit_logs_tron_standardized_navigation", route: "/admin/audit-logs", activeNav: "Audit Logs", title: "Audit Logs", archetype: "data-table" },
  { prototype: "automations_management_light", route: "/automations", activeNav: "Automations", title: "Automations", archetype: "split-pane" },
  { prototype: "chat_sessions_updated_tron_1", route: "/chats/8842-alpha", activeNav: "Chats", title: "Chat Sessions", archetype: "chat" },
  { prototype: "chat_sessions_updated_tron_2", route: "/chats/8842-alpha?variant=execution", activeNav: "Chats", title: "Chat Session Detail", archetype: "chat" },
  { prototype: "create_automation_tron_standardized", route: "/automations/new", activeNav: "Automations", title: "Create Automation", archetype: "form" },
  { prototype: "create_new_prompt_tron", route: "/prompts/new", activeNav: "Prompts", title: "Create New Prompt", archetype: "form-inspector" },
  { prototype: "create_new_skill_tron_standardized", route: "/skills/new", activeNav: "Skills", title: "Create New Skill", archetype: "form-inspector" },
  { prototype: "create_task_full_width_console_layout", route: "/tasks/new", activeNav: "Tasks", title: "Create Task", archetype: "form" },
  { prototype: "dashboard_light", route: "/dashboard", activeNav: "Dashboard", title: "Dashboard", archetype: "dashboard" },
  { prototype: "import_application_tron_unified_navigation_1", route: "/apps/import?source=github", activeNav: "Apps", title: "Import Application", archetype: "form" },
  { prototype: "import_application_tron_unified_navigation_2", route: "/apps/import?source=local", activeNav: "Apps", title: "Import Application Review", archetype: "form" },
  { prototype: "knowledge_graph_tron", route: "/knowledge-graph", activeNav: "Graph", title: "Knowledge Graph", archetype: "graph" },
  { prototype: "knowledge_graph_tron_domain_filtering", route: "/knowledge-graph/domain-filtering", activeNav: "Graph", title: "Knowledge Graph Domain Filtering", archetype: "graph" },
  { prototype: "login_tron_light_theme_centered", route: "/login", activeNav: "Login", title: "Login", archetype: "auth" },
  { prototype: "main_agent_chat_tron", route: "/agent", activeNav: "Main Agent", title: "Main Agent", archetype: "chat" },
  { prototype: "memory_management_tron_data_isolation_1", route: "/memory", activeNav: "Memory", title: "Memory Repository", archetype: "data-table" },
  { prototype: "memory_management_tron_data_isolation_2", route: "/memory/details", activeNav: "Memory", title: "Memory Details", archetype: "split-pane" },
  { prototype: "model_registry_tron", route: "/models", activeNav: "Models", title: "Model Registry", archetype: "split-pane" },
  { prototype: "observability_dashboard_light", route: "/observability", activeNav: "Observability", title: "Observability Dashboard", archetype: "dashboard" },
  { prototype: "prompt_management_tron_standardized", route: "/prompts", activeNav: "Prompts", title: "Prompt Management", archetype: "split-pane" },
  { prototype: "resources_management_tron", route: "/resources", activeNav: "Resources", title: "Resources", archetype: "data-table" },
  { prototype: "role_matrix_tron_standardized", route: "/admin/roles", activeNav: "Roles", title: "Role Matrix", archetype: "permission-grid" },
  { prototype: "skills_mcp_management_tron", route: "/skills", activeNav: "Skills", title: "Skills and MCP", archetype: "split-pane" },
  { prototype: "task_details_lumina_workspace_updated_hierarchy", route: "/tasks/detail", activeNav: "Tasks", title: "Task Details", archetype: "detail" },
  { prototype: "task_management_light", route: "/tasks", activeNav: "Tasks", title: "Task Management", archetype: "kanban" },
  { prototype: "user_permissions_tron", route: "/admin/users", activeNav: "Users", title: "User Permissions", archetype: "data-table" },
  { prototype: "variable_management_tron_standardized_1", route: "/variables", activeNav: "Variables", title: "Variable Management", archetype: "split-pane" },
  { prototype: "variable_management_tron_standardized_2", route: "/variables/details", activeNav: "Variables", title: "Variable Details", archetype: "split-pane" },
] as const;
