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
  | "permission-grid"
  | "settings";

export type AppRouteLayout = "console" | "settings";

export interface AppRouteShellConfig {
  contentClassName?: string;
  searchPlaceholder?: string;
}

export interface AppRoute {
  pageId: string;
  route: string;
  activeNav: string;
  title: string;
  archetype: AppPageArchetype;
  layout?: AppRouteLayout;
  settingsNav?: string;
  shellConfig?: AppRouteShellConfig;
}

export const appRoutes: AppRoute[] = [
  { pageId: "agents.index", route: "/agents", activeNav: "Agents", title: "Agents", archetype: "data-table", shellConfig: { searchPlaceholder: "Search agents..." } },
  { pageId: "models.new", route: "/models/new", activeNav: "Models", title: "Create Workspace Model", archetype: "form" },
  { pageId: "models.detail", route: "/models/detail", activeNav: "Models", title: "Workspace Model Detail", archetype: "detail" },
  { pageId: "apps.index", route: "/apps", activeNav: "Apps", title: "App Management", archetype: "data-table", shellConfig: { searchPlaceholder: "Search apps, repositories, and tags..." } },
  { pageId: "apps.new", route: "/apps/new", activeNav: "Apps", title: "Create App", archetype: "form", shellConfig: { searchPlaceholder: "Search apps, repositories, and tags..." } },
  { pageId: "apps.detail", route: "/apps/detail", activeNav: "Apps", title: "App Detail", archetype: "detail", shellConfig: { searchPlaceholder: "Search apps, repositories, and tags..." } },
  { pageId: "apps.edit", route: "/apps/edit", activeNav: "Apps", title: "Edit App", archetype: "form", shellConfig: { searchPlaceholder: "Search apps, repositories, and tags..." } },
  { pageId: "admin.auditLogs", route: "/admin/audit-logs", activeNav: "Audit Logs", title: "Audit Logs", archetype: "data-table", shellConfig: { searchPlaceholder: "Search users, roles, logs..." } },
  { pageId: "automations.index", route: "/automations", activeNav: "Automations", title: "Automations", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search automations..." } },
  { pageId: "chats.index", route: "/chats", activeNav: "Chats", title: "Chat Management", archetype: "chat", shellConfig: { contentClassName: "!p-0", searchPlaceholder: "Search sessions..." } },
  { pageId: "chats.reasoning", route: "/chats/8842-alpha", activeNav: "Chats", title: "Chat Sessions", archetype: "chat", shellConfig: { contentClassName: "!p-0", searchPlaceholder: "Search tasks..." } },
  { pageId: "chats.execution", route: "/chats/8842-alpha?variant=execution", activeNav: "Chats", title: "Chat Session Detail", archetype: "chat", shellConfig: { contentClassName: "!p-0", searchPlaceholder: "Search tasks..." } },
  { pageId: "automations.new", route: "/automations/new", activeNav: "Automations", title: "Create Automation", archetype: "form", shellConfig: { searchPlaceholder: "Search automations..." } },
  { pageId: "automations.detail", route: "/automations/detail", activeNav: "Automations", title: "Automation Detail", archetype: "detail", shellConfig: { searchPlaceholder: "Search automations..." } },
  { pageId: "prompts.new", route: "/prompts/new", activeNav: "Prompts", title: "Create New Prompt", archetype: "form-inspector", shellConfig: { searchPlaceholder: "Search prompts..." } },
  { pageId: "skills.bind", route: "/skills/bind", activeNav: "Skills", title: "Bind Skill", archetype: "form-inspector", shellConfig: { searchPlaceholder: "Search installed skills..." } },
  { pageId: "tasks.new", route: "/tasks/new", activeNav: "Tasks", title: "Create Task", archetype: "form", shellConfig: { searchPlaceholder: "Search tasks..." } },
  { pageId: "dashboard.index", route: "/dashboard", activeNav: "Dashboard", title: "Dashboard", archetype: "dashboard", shellConfig: { searchPlaceholder: "Search Tron..." } },
  { pageId: "knowledgeGraph.index", route: "/knowledge-graph", activeNav: "KnowledgeGraph", title: "Knowledge Graph", archetype: "graph", shellConfig: { searchPlaceholder: "Search concepts, relationships, and domains..." } },
  { pageId: "knowledgeGraph.domainFiltering", route: "/knowledge-graph/domain-filtering", activeNav: "KnowledgeGraph", title: "Knowledge Graph Domain Filtering", archetype: "graph", shellConfig: { searchPlaceholder: "Search concepts, relationships, and domains..." } },
  { pageId: "knowledgeBase.index", route: "/knowledge-base", activeNav: "KnowledgeBase", title: "Knowledge Base", archetype: "data-table", shellConfig: { searchPlaceholder: "Search knowledge bases, corpora, and sync status..." } },
  { pageId: "auth.login", route: "/login", activeNav: "Login", title: "Login", archetype: "auth" },
  { pageId: "agent.main", route: "/agent", activeNav: "Main Agent", title: "Main Agent", archetype: "chat", shellConfig: { searchPlaceholder: "Search Tron..." } },
  { pageId: "memory.index", route: "/memory", activeNav: "Memory", title: "Memory Repository", archetype: "data-table", shellConfig: { searchPlaceholder: "Search Tron..." } },
  { pageId: "memory.details", route: "/memory/details", activeNav: "Memory", title: "Memory Details", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search Tron..." } },
  { pageId: "models.index", route: "/models", activeNav: "Models", title: "Model Registry", archetype: "data-table" },
  { pageId: "observability.index", route: "/observability", activeNav: "Observability", title: "Observability Dashboard", archetype: "dashboard", shellConfig: { searchPlaceholder: "Search traces, agents, metrics..." } },
  { pageId: "prompts.index", route: "/prompts", activeNav: "Prompts", title: "Prompt Management", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search anything..." } },
  { pageId: "resources.index", route: "/resources", activeNav: "Resources", title: "Resources", archetype: "data-table", shellConfig: { searchPlaceholder: "Search resources..." } },
  { pageId: "admin.roles", route: "/admin/roles", activeNav: "Roles", title: "Role Matrix", archetype: "permission-grid", shellConfig: { searchPlaceholder: "Search users or roles..." } },
  { pageId: "admin.workspace", route: "/admin/workspace", activeNav: "Workspace", title: "Workspace Management", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search workspaces..." } },
  { pageId: "admin.userDetails", route: "/admin/users/detail", activeNav: "Users", title: "User Detail", archetype: "detail", shellConfig: { searchPlaceholder: "Search users..." } },
  { pageId: "admin.usersNew", route: "/admin/users/new", activeNav: "Users", title: "Create User", archetype: "form", shellConfig: { searchPlaceholder: "Search users..." } },
  { pageId: "skills.index", route: "/skills", activeNav: "Skills", title: "Workspace Skills", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search installed skills..." } },
  { pageId: "tasks.detail", route: "/tasks/detail", activeNav: "Tasks", title: "Task Details", archetype: "detail", shellConfig: { searchPlaceholder: "Search task context..." } },
  { pageId: "tasks.index", route: "/tasks", activeNav: "Tasks", title: "Task Management", archetype: "kanban", shellConfig: { searchPlaceholder: "Search tasks..." } },
  { pageId: "admin.users", route: "/admin/users", activeNav: "Users", title: "Users", archetype: "data-table", shellConfig: { searchPlaceholder: "Search users..." } },
  { pageId: "variables.index", route: "/variables", activeNav: "Variables", title: "Variable Management", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search variables..." } },
  { pageId: "variables.details", route: "/variables/details", activeNav: "Variables", title: "Variable Details", archetype: "split-pane" },
  { pageId: "settings.general", route: "/settings/general", activeNav: "Settings", title: "General Settings", archetype: "settings", layout: "settings", settingsNav: "General" },
  { pageId: "settings.agents", route: "/settings/agents", activeNav: "Settings", title: "Agent Management", archetype: "settings", layout: "settings", settingsNav: "Agents" },
  { pageId: "settings.models", route: "/settings/models", activeNav: "Settings", title: "Platform Models", archetype: "settings", layout: "settings", settingsNav: "Models" },
  { pageId: "settings.modelsNew", route: "/settings/models/new", activeNav: "Settings", title: "Create Platform Model", archetype: "settings", layout: "settings", settingsNav: "Models" },
  { pageId: "settings.modelsDetail", route: "/settings/models/detail", activeNav: "Settings", title: "Platform Model Detail", archetype: "settings", layout: "settings", settingsNav: "Models" },
  { pageId: "settings.variables", route: "/settings/variables", activeNav: "Settings", title: "Platform Variables", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.variablesNew", route: "/settings/variables/new", activeNav: "Settings", title: "Create Platform Variable", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.variablesDetail", route: "/settings/variables/detail", activeNav: "Settings", title: "Platform Variable Detail", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.variablesEdit", route: "/settings/variables/edit", activeNav: "Settings", title: "Edit Platform Variable", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.skills", route: "/settings/skills", activeNav: "Settings", title: "Skill Center", archetype: "settings", layout: "settings", settingsNav: "Skill Center" },
  { pageId: "settings.skillsSync", route: "/settings/skills/sync", activeNav: "Settings", title: "Sync Skills", archetype: "settings", layout: "settings", settingsNav: "Skill Center" },
] as const;
