import { Folder } from "lucide-react";
import { getNavigationIcon } from "@/shared/lib/navigationMenuMapper";
import type { TronNavSection } from "@/shared/types/ShellNavigation";

// Mirrors the built-in navigation seed; API trees remain authoritative.
export const tronNavigation: TronNavSection[] = [
  { id: "DUDES", label: "Dude", translationKey: "shell:nav.items.dudes", icon: getNavigationIcon("Bot") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "DUDES", label: "Dude", translationKey: "shell:nav.items.dudes", href: "/dudes", icon: getNavigationIcon("Bot") ?? Folder, childDisplay: "Inline" }
] },
  { id: "CHAT_NEW_SESSION", label: "New Chat", translationKey: "shell:chatSidebar.newChat", icon: getNavigationIcon("SquarePen") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "CHAT_NEW_SESSION", label: "New Chat", translationKey: "shell:chatSidebar.newChat", href: "", icon: getNavigationIcon("SquarePen") ?? Folder, childDisplay: "Inline", widgetCode: "chat.newSession" }
] },
  { id: "EXPLORE_MORE", label: "Explore more", translationKey: "shell:nav.sections.exploreMore", icon: getNavigationIcon("Ellipsis") ?? Folder, standalone: false, flyout: true, items: [
{ menuCode: "NOTIFICATIONS", label: "Notifications", translationKey: "shell:nav.items.notifications", href: "/notifications", icon: getNavigationIcon("Bell") ?? Folder, childDisplay: "Inline" },
{ menuCode: "CORE", label: "Core", translationKey: "shell:nav.sections.core", href: "", icon: getNavigationIcon("LayoutDashboard") ?? Folder, childDisplay: "Inline", isGroup: true, children: [
{ menuCode: "APPS", label: "Apps", translationKey: "shell:nav.items.apps", href: "/apps", icon: getNavigationIcon("FolderGit2") ?? Folder, childDisplay: "Inline" },
{ menuCode: "TASKS", label: "Tasks", translationKey: "shell:nav.items.tasks", href: "/tasks", icon: getNavigationIcon("ListTodo") ?? Folder, childDisplay: "Inline" },
{ menuCode: "AUTOMATIONS", label: "Automations", translationKey: "shell:nav.items.automations", href: "/automations", icon: getNavigationIcon("AlarmClock") ?? Folder, childDisplay: "Inline" },
{ menuCode: "VARIABLES", label: "Variables", translationKey: "shell:nav.items.variables", href: "/variables", icon: getNavigationIcon("KeyRound") ?? Folder, childDisplay: "Inline" }
] },
{ menuCode: "AGENT_OPERATIONS", label: "Agent Operations", translationKey: "shell:nav.sections.agentOperations", href: "", icon: getNavigationIcon("Bot") ?? Folder, childDisplay: "Inline", isGroup: true, children: [
{ menuCode: "MCP_SERVERS", label: "MCP Servers", translationKey: "shell:nav.items.mcpServers", href: "/mcp-servers", icon: getNavigationIcon("PlugZap") ?? Folder, childDisplay: "Inline" },
{ menuCode: "MODELS", label: "Models", translationKey: "shell:nav.items.models", href: "/models", icon: getNavigationIcon("Sparkles") ?? Folder, childDisplay: "Inline" }
] },
{ menuCode: "DATA", label: "Data", translationKey: "shell:nav.sections.data", href: "", icon: getNavigationIcon("Database") ?? Folder, childDisplay: "Inline", isGroup: true, children: [
{ menuCode: "RESOURCE_MANAGEMENT", label: "Resource Management", translationKey: "shell:nav.items.resourceManagement", href: "", icon: getNavigationIcon("Archive") ?? Folder, childDisplay: "Inline", isGroup: true, children: [
{ menuCode: "FILE_MANAGEMENT", label: "File Management", translationKey: "shell:nav.items.fileManagement", href: "/resource-management/files", icon: getNavigationIcon("Folder") ?? Folder, childDisplay: "Inline" },
{ menuCode: "DATA_SOURCE_MANAGEMENT", label: "Data Source Management", translationKey: "shell:nav.items.dataSourceManagement", href: "/resource-management/data-sources", icon: getNavigationIcon("DatabaseZap") ?? Folder, childDisplay: "Inline" },
{ menuCode: "SERVER_MANAGEMENT", label: "Server Management", translationKey: "shell:nav.items.serverManagement", href: "/resource-management/servers", icon: getNavigationIcon("Server") ?? Folder, childDisplay: "Inline" },
{ menuCode: "SERVER_ACCOUNT_MANAGEMENT", label: "Server Account Management", translationKey: "shell:nav.items.serverAccountManagement", href: "/resource-management/server-accounts", icon: getNavigationIcon("UserRound") ?? Folder, childDisplay: "Inline" }
] }
] }
] },
  { id: "CHAT_PROJECTS", label: "Topics", translationKey: "shell:nav.items.projects", icon: getNavigationIcon("FolderKanban") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "CHAT_PROJECTS", label: "Topics", translationKey: "shell:nav.items.projects", href: "", icon: getNavigationIcon("FolderKanban") ?? Folder, childDisplay: "Inline", widgetCode: "chat.projects" }
] },
  { id: "CHAT_RECENTS", label: "Recents", translationKey: "shell:chatSidebar.recent", icon: getNavigationIcon("History") ?? Folder, standalone: true, flyout: false, items: [
{ menuCode: "CHAT_RECENTS", label: "Recents", translationKey: "shell:chatSidebar.recent", href: "", icon: getNavigationIcon("History") ?? Folder, childDisplay: "Inline", widgetCode: "chat.recents" }
] },
];
