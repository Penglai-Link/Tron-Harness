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
  { pageId: "chats.index", route: "/chats", activeNav: "Chats", title: "Chat Management", archetype: "chat", shellConfig: { contentClassName: "!h-[calc(100vh-3.5rem)] !overflow-hidden !p-0", searchPlaceholder: "Search sessions..." } },
  { pageId: "chats.reasoning", route: "/chats/8842-alpha", activeNav: "Chats", title: "Chat Sessions", archetype: "chat", shellConfig: { contentClassName: "!h-[calc(100vh-3.5rem)] !overflow-hidden !p-0", searchPlaceholder: "Search tasks..." } },
  { pageId: "chats.execution", route: "/chats/8842-alpha?variant=execution", activeNav: "Chats", title: "Chat Session Detail", archetype: "chat", shellConfig: { contentClassName: "!h-[calc(100vh-3.5rem)] !overflow-hidden !p-0", searchPlaceholder: "Search tasks..." } },
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
  { pageId: "memory.new", route: "/memory/new", activeNav: "Memory", title: "Create Memory", archetype: "form", shellConfig: { searchPlaceholder: "Search Tron..." } },
  { pageId: "memory.details", route: "/memory/details", activeNav: "Memory", title: "Memory Details", archetype: "detail", shellConfig: { searchPlaceholder: "Search Tron..." } },
  { pageId: "models.index", route: "/models", activeNav: "Models", title: "Model Registry", archetype: "data-table" },
  { pageId: "mcpServers.index", route: "/mcp-servers", activeNav: "MCP Servers", title: "MCP Servers", archetype: "data-table", shellConfig: { searchPlaceholder: "Search MCP servers..." } },
  { pageId: "observability.index", route: "/observability", activeNav: "Observability", title: "Observability Dashboard", archetype: "dashboard", shellConfig: { searchPlaceholder: "Search traces, agents, metrics..." } },
  { pageId: "prompts.index", route: "/prompts", activeNav: "Prompts", title: "Prompt Management", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search anything..." } },
  { pageId: "rag.index", route: "/rag", activeNav: "RAG", title: "RAG Management", archetype: "data-table", shellConfig: { searchPlaceholder: "Search RAG corpora, profiles, and sources..." } },
  { pageId: "rag.corpora.new", route: "/rag/corpora/new", activeNav: "RAG", title: "Create Corpus", archetype: "form", shellConfig: { searchPlaceholder: "Search RAG corpora, profiles, and sources..." } },
  { pageId: "rag.corpora.edit", route: "/rag/corpora/:corpusCode/edit", activeNav: "RAG", title: "Edit Corpus", archetype: "form", shellConfig: { searchPlaceholder: "Search RAG corpora, profiles, and sources..." } },
  { pageId: "rag.profiles.ingest.new", route: "/rag/profiles/ingest/new", activeNav: "RAG", title: "Create Ingest Profile", archetype: "form", shellConfig: { searchPlaceholder: "Search RAG corpora, profiles, and sources..." } },
  { pageId: "rag.profiles.ingest.edit", route: "/rag/profiles/ingest/:profileId/edit", activeNav: "RAG", title: "Edit Ingest Profile", archetype: "form", shellConfig: { searchPlaceholder: "Search RAG corpora, profiles, and sources..." } },
  { pageId: "rag.profiles.retrieval.new", route: "/rag/profiles/retrieval/new", activeNav: "RAG", title: "Create Retrieval Profile", archetype: "form", shellConfig: { searchPlaceholder: "Search RAG corpora, profiles, and sources..." } },
  { pageId: "rag.profiles.retrieval.edit", route: "/rag/profiles/retrieval/:profileId/edit", activeNav: "RAG", title: "Edit Retrieval Profile", archetype: "form", shellConfig: { searchPlaceholder: "Search RAG corpora, profiles, and sources..." } },
  { pageId: "resources.index", route: "/resources", activeNav: "Resources", title: "Resources", archetype: "data-table", shellConfig: { searchPlaceholder: "Search resources..." } },
  { pageId: "admin.roles", route: "/admin/roles", activeNav: "Roles", title: "Role Matrix", archetype: "permission-grid", shellConfig: { searchPlaceholder: "Search users or roles..." } },
  { pageId: "admin.workspace", route: "/admin/workspace", activeNav: "Workspace", title: "Workspace Management", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search workspaces..." } },
  { pageId: "admin.userDetails", route: "/admin/users/detail", activeNav: "Users", title: "User Detail", archetype: "detail", shellConfig: { searchPlaceholder: "Search users..." } },
  { pageId: "admin.usersNew", route: "/admin/users/new", activeNav: "Users", title: "Create User", archetype: "form", shellConfig: { searchPlaceholder: "Search users..." } },
  { pageId: "skills.index", route: "/skills", activeNav: "Skills", title: "Workspace Skills", archetype: "split-pane", shellConfig: { searchPlaceholder: "Search installed skills..." } },
  { pageId: "tasks.detail", route: "/tasks/detail", activeNav: "Tasks", title: "Task Details", archetype: "detail", shellConfig: { searchPlaceholder: "Search task context..." } },
  { pageId: "tasks.index", route: "/tasks", activeNav: "Tasks", title: "Task Management", archetype: "kanban", shellConfig: { searchPlaceholder: "Search tasks..." } },
  { pageId: "admin.users", route: "/admin/users", activeNav: "Users", title: "Users", archetype: "data-table", shellConfig: { searchPlaceholder: "Search users..." } },
  { pageId: "variables.index", route: "/variables", activeNav: "Variables", title: "Variable Management", archetype: "data-table", shellConfig: { searchPlaceholder: "Search variables..." } },
  { pageId: "variables.new", route: "/variables/new", activeNav: "Variables", title: "Create Variable", archetype: "form" },
  { pageId: "variables.details", route: "/variables/detail", activeNav: "Variables", title: "Variable Details", archetype: "detail" },
  { pageId: "variables.details", route: "/variables/details", activeNav: "Variables", title: "Variable Details", archetype: "detail" },
  { pageId: "settings.general", route: "/settings/general", activeNav: "Settings", title: "General Settings", archetype: "settings", layout: "settings", settingsNav: "General" },
  { pageId: "settings.agents", route: "/settings/agents", activeNav: "Settings", title: "Agent Management", archetype: "settings", layout: "settings", settingsNav: "Agents" },
  { pageId: "settings.mcpServers", route: "/settings/mcp-servers", activeNav: "Settings", title: "Platform MCP Servers", archetype: "settings", layout: "settings", settingsNav: "MCP Servers" },
  { pageId: "settings.mcpServersNew", route: "/settings/mcp-servers/new", activeNav: "Settings", title: "Connect MCP server", archetype: "settings", layout: "settings", settingsNav: "MCP Servers" },
  { pageId: "settings.mcpServersDetail", route: "/settings/mcp-servers/detail", activeNav: "Settings", title: "Platform MCP Server Detail", archetype: "detail", layout: "settings", settingsNav: "MCP Servers" },
  { pageId: "settings.mcpServersEdit", route: "/settings/mcp-servers/edit", activeNav: "Settings", title: "Edit Platform MCP Server", archetype: "settings", layout: "settings", settingsNav: "MCP Servers" },
  { pageId: "settings.tools", route: "/settings/tools", activeNav: "Settings", title: "Platform Tools", archetype: "settings", layout: "settings", settingsNav: "Tools" },
  { pageId: "settings.models", route: "/settings/models", activeNav: "Settings", title: "Platform Models", archetype: "settings", layout: "settings", settingsNav: "Models" },
  { pageId: "settings.modelsNew", route: "/settings/models/new", activeNav: "Settings", title: "Create Platform Model", archetype: "settings", layout: "settings", settingsNav: "Models" },
  { pageId: "settings.modelsDetail", route: "/settings/models/detail", activeNav: "Settings", title: "Platform Model Detail", archetype: "settings", layout: "settings", settingsNav: "Models" },
  { pageId: "settings.memoryDomains", route: "/settings/memory-domains", activeNav: "Settings", title: "Memory Domains", archetype: "settings", layout: "settings", settingsNav: "Memory Domains" },
  { pageId: "settings.openApi", route: "/settings/openapi", activeNav: "Settings", title: "Platform OpenAPI", archetype: "settings", layout: "settings", settingsNav: "OpenAPI" },
  { pageId: "settings.openApiNew", route: "/settings/openapi/new", activeNav: "Settings", title: "New Open App", archetype: "settings", layout: "settings", settingsNav: "OpenAPI" },
  { pageId: "settings.openApiDetail", route: "/settings/openapi/detail", activeNav: "Settings", title: "Open App Detail", archetype: "settings", layout: "settings", settingsNav: "OpenAPI" },
  { pageId: "settings.openApiEdit", route: "/settings/openapi/edit", activeNav: "Settings", title: "Edit Open App", archetype: "settings", layout: "settings", settingsNav: "OpenAPI" },
  { pageId: "settings.variables", route: "/settings/variables", activeNav: "Settings", title: "Platform Variables", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.variablesNew", route: "/settings/variables/new", activeNav: "Settings", title: "Create Platform Variable", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.variablesDetail", route: "/settings/variables/detail", activeNav: "Settings", title: "Platform Variable Detail", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.variablesEdit", route: "/settings/variables/edit", activeNav: "Settings", title: "Edit Platform Variable", archetype: "settings", layout: "settings", settingsNav: "Variables" },
  { pageId: "settings.skills", route: "/settings/skills", activeNav: "Settings", title: "Skill Center", archetype: "settings", layout: "settings", settingsNav: "Skill Center" },
  { pageId: "settings.skillsSync", route: "/settings/skills/sync", activeNav: "Settings", title: "Sync Skills", archetype: "settings", layout: "settings", settingsNav: "Skill Center" },
] as const;
