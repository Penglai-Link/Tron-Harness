export type AppPageArchetype =
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

export interface AppRoute {
  pageId: string;
  route: string;
  activeNav: string;
  title: string;
  archetype: AppPageArchetype;
}

export const appRoutes: AppRoute[] = [
  { pageId: "models.new", route: "/models/new", activeNav: "Models", title: "Add Model", archetype: "form-inspector" },
  { pageId: "apps.index", route: "/apps", activeNav: "Apps", title: "App Management", archetype: "data-table" },
  { pageId: "admin.auditLogs", route: "/admin/audit-logs", activeNav: "Audit Logs", title: "Audit Logs", archetype: "data-table" },
  { pageId: "automations.index", route: "/automations", activeNav: "Automations", title: "Automations", archetype: "split-pane" },
  { pageId: "chats.reasoning", route: "/chats/8842-alpha", activeNav: "Chats", title: "Chat Sessions", archetype: "chat" },
  { pageId: "chats.execution", route: "/chats/8842-alpha?variant=execution", activeNav: "Chats", title: "Chat Session Detail", archetype: "chat" },
  { pageId: "automations.new", route: "/automations/new", activeNav: "Automations", title: "Create Automation", archetype: "form" },
  { pageId: "prompts.new", route: "/prompts/new", activeNav: "Prompts", title: "Create New Prompt", archetype: "form-inspector" },
  { pageId: "skills.new", route: "/skills/new", activeNav: "Skills", title: "Create New Skill", archetype: "form-inspector" },
  { pageId: "tasks.new", route: "/tasks/new", activeNav: "Tasks", title: "Create Task", archetype: "form" },
  { pageId: "dashboard.index", route: "/dashboard", activeNav: "Dashboard", title: "Dashboard", archetype: "dashboard" },
  { pageId: "apps.import.github", route: "/apps/import?source=github", activeNav: "Apps", title: "Import Application", archetype: "form" },
  { pageId: "apps.import.local", route: "/apps/import?source=local", activeNav: "Apps", title: "Import Application Review", archetype: "form" },
  { pageId: "knowledgeGraph.index", route: "/knowledge-graph", activeNav: "Graph", title: "Knowledge Graph", archetype: "graph" },
  { pageId: "knowledgeGraph.domainFiltering", route: "/knowledge-graph/domain-filtering", activeNav: "Graph", title: "Knowledge Graph Domain Filtering", archetype: "graph" },
  { pageId: "auth.login", route: "/login", activeNav: "Login", title: "Login", archetype: "auth" },
  { pageId: "agent.main", route: "/agent", activeNav: "Main Agent", title: "Main Agent", archetype: "chat" },
  { pageId: "memory.index", route: "/memory", activeNav: "Memory", title: "Memory Repository", archetype: "data-table" },
  { pageId: "memory.details", route: "/memory/details", activeNav: "Memory", title: "Memory Details", archetype: "split-pane" },
  { pageId: "models.index", route: "/models", activeNav: "Models", title: "Model Registry", archetype: "split-pane" },
  { pageId: "observability.index", route: "/observability", activeNav: "Observability", title: "Observability Dashboard", archetype: "dashboard" },
  { pageId: "prompts.index", route: "/prompts", activeNav: "Prompts", title: "Prompt Management", archetype: "split-pane" },
  { pageId: "resources.index", route: "/resources", activeNav: "Resources", title: "Resources", archetype: "data-table" },
  { pageId: "admin.roles", route: "/admin/roles", activeNav: "Roles", title: "Role Matrix", archetype: "permission-grid" },
  { pageId: "skills.index", route: "/skills", activeNav: "Skills", title: "Skills and MCP", archetype: "split-pane" },
  { pageId: "tasks.detail", route: "/tasks/detail", activeNav: "Tasks", title: "Task Details", archetype: "detail" },
  { pageId: "tasks.index", route: "/tasks", activeNav: "Tasks", title: "Task Management", archetype: "kanban" },
  { pageId: "admin.users", route: "/admin/users", activeNav: "Users", title: "User Permissions", archetype: "data-table" },
  { pageId: "variables.index", route: "/variables", activeNav: "Variables", title: "Variable Management", archetype: "split-pane" },
  { pageId: "variables.details", route: "/variables/details", activeNav: "Variables", title: "Variable Details", archetype: "split-pane" },
] as const;
